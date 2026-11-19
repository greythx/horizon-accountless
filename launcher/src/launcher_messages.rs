//! The backend's messages, drained every frame (`Launcher::drain`).
use super::*;

impl Launcher {
    pub(crate) fn drain(&mut self, window: gpui::AnyWindowHandle, cx: &mut Context<Self>) {
        let mut changed = false;
        self.perf.tick();
        match self.bench.as_mut().map(|b| b.tick(&self.perf)) {
            Some(perf::Step::Go(page)) => {
                let pages: Vec<Page> = (Page::ALL.iter().map(|p| p.0)).collect();
                self.go(pages[page % pages.len()]);
                changed = true;
            }
            Some(perf::Step::Scroll) => {
                self.go(Page::Versions);
                changed = true;
            }
            Some(perf::Step::Quit) => cx.quit(),
            Some(perf::Step::Wait) | None => {}
        }
        while let Ok(msg) = self.rx.try_recv() {
            changed = true;
            match msg {
                Msg::Releases(r) => {
                    if let (Ok(_), Some(checked)) = (&r, self.checked) {
                        self.perf.record_since("launcher.release_list_ms", checked);
                    }
                    if let Ok(list) = &r {
                        horizon_launcher::nightly::note_published(list);
                    }
                    self.releases = r.map_or_else(Load::Failed, Load::Ready);
                    self.notes.clear();
                }
                // The page reads is_prepared when drawn; `changed` redraws it.
                Msg::Reused(_) => self.installed = backend::installed(),
                Msg::Playing(playing) => self.playing = playing,
                Msg::JavaGame(id, home, cmd) => {
                    self.starting_client = None;
                    let spawned = cmd.and_then(|(cmd, vars)| {
                        self.client_vars = vars;
                        backend::spawn(Job::Client, cmd, self.tx.clone())
                    });
                    match spawned {
                        Ok(r) => {
                            self.client = Some(r);
                            self.client_home = Some(home);
                            self.client_run = Some((id, Instant::now()));
                            self.step_aside(cx);
                        }
                        Err(e) => self.client_error = Some(e),
                    }
                }
                Msg::SessionDone => self.start_play(),
                Msg::JarConsent { jars, reply } => jar_launch::consent(window, jars, reply, cx),
                Msg::JarClient { run, prepared } => {
                    self.starting_client = None;
                    match *prepared {
                        Ok(cmd) => {
                            self.client_home = cmd.get_current_dir().map(|d| d.to_path_buf());
                            match backend::spawn(Job::Client, cmd, self.tx.clone()) {
                                Ok(r) => {
                                    self.client = Some(r);
                                    self.client_run = run.map(|id| (id, Instant::now()));
                                    self.step_aside(cx);
                                }
                                Err(e) => {
                                    self.client_error = Some(e);
                                }
                            }
                        }
                        // A start that does not happen leaves no session file behind.
                        Err(e) => {
                            self.client_error = Some(e);
                        }
                    }
                }
                Msg::Progress(tag, p) => {
                    self.bar = (self.bar.1, p);
                    self.installing = Some((tag, p));
                }
                Msg::Installed(tag, result) => {
                    self.installing = None;
                    self.bar = (0., 0.);
                    self.installed = backend::installed();
                    let for_instance = self.instance_install.take_if(|t| *t == tag).is_some();
                    if let (Ok(()), Some(started)) = (&result, self.install_started.take()) {
                        self.perf.record_since("launcher.install_ms", started);
                    }
                    match result {
                        Ok(()) => {
                            // An instance's own version leaves the other instances on theirs.
                            // Judged now, not when the download started: a version picked on
                            // Versions meanwhile stays picked.
                            let before: Vec<String> = self
                                .installed
                                .iter()
                                .filter(|t| **t != tag)
                                .cloned()
                                .collect();
                            let select = !for_instance || self.launcher_version().is_none();
                            let select = select
                                && backend::select_after_update(
                                    self.settings.selected.as_deref(),
                                    &before,
                                );
                            if select {
                                self.settings.selected = Some(tag);
                            }
                            // Pruning reads settings from disk: protect the selection only once
                            // it has been saved there. A failed save leaves all versions in place.
                            if self.save_settings() {
                                let tx = self.tx.clone();
                                std::thread::spawn(move || {
                                    let _ = tx.send(Msg::Pruned(backend::prune_old_versions()));
                                });
                            }
                        }
                        Err(e) => self.install_error = Some((tag, e)),
                    }
                    if let Some(next) = self.install_next.take() {
                        self.install_for_instance(next);
                    }
                }
                Msg::Removed(tag, result) => {
                    self.removing.retain(|t| *t != tag);
                    self.installed = backend::installed();
                    let selected = if self.settings_save_error.is_some() {
                        self.settings.selected.take()
                    } else {
                        Settings::load().selected
                    };
                    self.settings.selected = selection_after_removal(&self.installed, selected);
                    if let Err(e) = result {
                        self.install_error = Some((tag, e));
                    }
                }
                Msg::DiskUse(disk) => self.disk = disk,
                Msg::Storage(result) => self.storage_arrived(result),
                Msg::Imported(result) => {
                    self.importing_build = false;
                    self.installed = backend::installed();
                    match result {
                        // `build_import::import` saved the selection; this keeps the window's copy.
                        Ok(tag) => self.settings.selected = Some(tag),
                        Err(e) => self.import_error = Some(e),
                    }
                }
                Msg::Pruned(gone) => {
                    if !gone.is_empty() {
                        self.installed = backend::installed();
                    }
                }
                Msg::NativeRegistryConsent(inventory, reply) => {
                    native_registry_consent::show(window, inventory, reply, cx)
                }
                Msg::NativeProgress(step, progress) => {
                    self.native_step = Some(step);
                    self.native_progress = Some(progress);
                }
                Msg::Native(result) => {
                    // The last step of a build that worked says how it went.
                    let last = self.native_step.take();
                    self.native_progress = None;
                    self.first_build = None;
                    // Build and play: the game starts once the build worked, and only for the
                    // instance it was pressed for, still on the build just made (another may
                    // have been picked, or its version changed, meanwhile).
                    let wanted = self.play_after_build.take();
                    let built = result.as_ref().ok().cloned();
                    let cancelled = matches!(&result, Err(native_mods::Failed::Other(error)) if error == native_mods::CANCELLED);
                    self.native_done = match (&result, cancelled) {
                        (Ok(_), _) => last,
                        (Err(_), true) => Some(tr(native_mods::CANCELLED).into()),
                        (Err(_), false) => None,
                    };
                    self.installed = backend::installed();
                    self.reload_instances();
                    // A build attempt ended: an open panel reads the log it left.
                    self.native_log.stale = true;
                    // No permission: the question is the answer, not the refusal's words.
                    let no_consent = matches!(result, Err(native_mods::Failed::NoConsent));
                    if no_consent {
                        self.request_native_build_consent();
                    }
                    if let (Err(e), false) = (result, cancelled || no_consent) {
                        if let native_mods::Failed::Worlds(stake) = &e {
                            self.keep_stop(stake);
                        }
                        if let native_mods::Failed::Mods { left, .. } = &e {
                            self.native_failed_mods = left.iter().map(|l| l.id.clone()).collect();
                        }
                        // Read against the mods this build was given, not the instance now
                        // selected.
                        let built = std::mem::take(&mut self.native_built_mods);
                        self.native_failed(String::from(e), &built);
                    }
                    let play_now = wanted.is_some()
                        && built.is_some()
                        && self
                            .selected_instance()
                            .is_some_and(|i| Some(&i.id) == wanted.as_ref() && i.version == built);
                    if play_now {
                        self.play();
                    }
                }
                Msg::PackLine(line) => {
                    self.pack_log.push_back(line);
                    if self.pack_log.len() > LOG_LINES {
                        self.pack_log.pop_front();
                    }
                }
                Msg::PackDone(result) => self.pack_done(result),
                Msg::FoundPacks(found) => {
                    self.found_scanning = false;
                    self.found_packs = found;
                }
                // Old versions log their session; it never reaches the window.
                Msg::Line(_, line) if line.contains("Session ID is") => {}
                Msg::Line(job, line) => {
                    if job == Job::Client && line.starts_with(perf::GAME_FIRST_FRAME) {
                        if let Some(pressed) = self.play_pressed.take() {
                            self.perf
                                .record_since("launcher.play_to_game_frame_ms", pressed);
                        }
                    }
                    // The MCP server keeps its own server's log; the launcher's are the preparer's
                    // and the game's.
                    if job != Job::Server {
                        console_page::keep_line(
                            &mut self.prepare_log,
                            &mut self.log_revision,
                            line,
                        );
                    }
                }
                Msg::Exit(job, result) => {
                    // A preparation (or a build) ended: the disk may have changed under the pages.
                    backend::forget_prepared();
                    let error = match result.err() {
                        // The preparation stopped for the one reason it may: the player never
                        // accepted the download. Ask, instead of showing the text.
                        Some(e) if e.starts_with(backend::CONSENT_REQUIRED_PREFIX) => {
                            self.mojang_consent = Some(crate::MojangConsentRequest::Prepare);
                            None
                        }
                        Some(e) if e == horizon_launcher::mojang::CANCELLED => Some(
                            crate::i18n::tr(
                                "Download cancelled. Download again to go on where it stopped.",
                            )
                            .to_string(),
                        ),
                        other => other,
                    };
                    match job {
                        Job::Prepare => {
                            self.preparing = false;
                            // Play pressed before the version was prepared plays now.
                            if let Some((tag, instance)) = self.play_when_prepared.take() {
                                if error.is_none() && backend::check_prepared(&tag) {
                                    self.starting_pack = None;
                                    self.start_client(tag, Some(instance));
                                }
                            }
                            if let (None, Some(started)) = (&error, self.prepare_started.take()) {
                                self.perf.record_since("launcher.prepare_ms", started);
                            }
                            // A preparer process's exit status says little, so its last line
                            // follows; the launcher's own errors are full sentences already.
                            self.prepare_error = error.map(|e| {
                                let tail = self
                                    .prepare_log
                                    .iter()
                                    .rev()
                                    .find(|l| !l.trim().is_empty())
                                    .filter(|_| e.starts_with("Exited with"));
                                tail.map_or(e.clone(), |t| format!("{e}: {t}"))
                            });
                        }
                        Job::Client => {
                            self.client = None;
                            self.come_back(cx);
                            // The game saved: last played, sizes and pictures changed.
                            self.worlds.stale();
                            self.multiplayer.stale();
                            self.screenshots.stale();
                            self.game_packs.stale();
                            let vars = std::mem::take(&mut self.client_vars);
                            if let Some((id, started)) = self.client_run.take() {
                                if let Err(e) = instances::record_play(&id, started.elapsed()) {
                                    eprintln!("Play time not saved: {e}");
                                }
                                if let Some(i) = instances::list().into_iter().find(|i| i.id == id)
                                {
                                    horizon_launcher::launch::run_after_exit(
                                        &i.launch, &i.home, &vars,
                                    );
                                }
                                self.reload_instances();
                            }
                            // The run's own game folder: an instance's or a pack's keeps its reports.
                            let home = self.client_home.clone().unwrap_or_else(backend::game_home);
                            let report = self
                                .client_started
                                .take()
                                .and_then(|since| backend::crash_report_since(&home, since));
                            let switched = if error.is_some() && report.is_none() {
                                self.switched_from_vulkan()
                            } else {
                                None
                            };
                            if let Some(api) = switched {
                                self.client_error = Some(trf(
                                    "Horizon's Vulkan renderer crashed while starting on this \
                                     graphics card, so the game now uses {api}. Press Play to \
                                     start it again. Video Settings > Graphics API switches back.",
                                    &[("api", api)],
                                ));
                            } else if error.is_some() || report.is_some() {
                                let summary = report
                                    .as_ref()
                                    .and_then(|p| std::fs::read_to_string(p).ok())
                                    .and_then(|text| backend::crash_summary(&text))
                                    .or_else(|| {
                                        let tail = self
                                            .prepare_log
                                            .iter()
                                            .rev()
                                            .find(|l| !l.trim().is_empty());
                                        error.as_ref().map(|e| {
                                            tail.map_or(e.clone(), |t| format!("{e}: {t}"))
                                        })
                                    })
                                    .unwrap_or_default();
                                self.crash = Some(Crash {
                                    report,
                                    summary,
                                    copied: false,
                                });
                            }
                        }
                        Job::Server => {}
                    }
                }
            }
        }
        changed |= self.servers_tick();
        changed |= self.mods_tick();
        changed |= self.github_tick();
        changed |= self.native_log_read(cx);
        changed |= self.toast_tick();
        if changed {
            cx.notify();
        }
    }
}
