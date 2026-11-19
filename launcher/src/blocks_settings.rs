//! The blocky look's Settings page, the native build consent and the settings rows.
use super::*;

impl Launcher {
    pub(crate) fn settings_page(&mut self, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        let prune = switch("auto-prune", self.settings.auto_prune(), &t).on_click(cx.listener(
            |this, _: &ClickEvent, _, cx| {
                this.settings.prune_old_versions = Some(!this.settings.auto_prune());
                this.save_settings();
                cx.notify();
            },
        ));
        let opening = switch("opening", !self.settings.skip_opening, &t).on_click(cx.listener(
            |this, _: &ClickEvent, _, cx| {
                this.settings.skip_opening = !this.settings.skip_opening;
                this.save_settings();
                cx.notify();
            },
        ));
        // Linux: the launcher's entry in the applications menu (`horizon_launcher::desktop`).
        let menu_entry = switch("desktop-entry", self.settings.desktop_entry(), &t).on_click(
            cx.listener(|this, _: &ClickEvent, _, cx| {
                let on = !this.settings.desktop_entry();
                this.settings.desktop_entry = Some(on);
                this.save_settings();
                let _ = if on {
                    horizon_launcher::desktop::install(backend::exe_path()).map(|_| ())
                } else {
                    horizon_launcher::desktop::remove()
                };
                cx.notify();
            }),
        );
        let mono = |s: String| div().font_family(theme::MONO).child(s).into_any_element();
        let plain = |s: &str| div().child(tr(s).to_string()).into_any_element();
        if t.glass {
            return self.glass_settings(
                glass_settings::Built {
                    prune,
                    opening,
                    menu_entry,
                },
                cx,
            );
        }
        let n = 0;
        let group = |label: &'static str, icon: &'static str| {
            section_title(label, icon, &t).mt(px(20.)).mb(px(12.))
        };
        div()
            .flex()
            .flex_col()
            .gap(px(32.))
            .child(page_header(
                "Settings",
                "The launcher, its look and where your game lives.",
                &t,
            ))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(group("Launcher", "monitor").mt_0())
                    .child(index_row(
                        n + 1,
                        tr("Delete old versions automatically"),
                        Some(
                            div()
                                .child(trf(
                                    "Keeps the {count} newest, the one in use and the ones your instances use. Deleted versions install again from the Versions page.",
                                    &[("count", &self.settings.keep_newest().to_string())],
                                ))
                                .into_any_element(),
                        ),
                        Some(prune.into_any_element()),
                        &t,
                    ))
                    .child(group("Launcher", "monitor"))
                    .child(self.launcher_theme_row(cx))
                    .child(self.accent_row(cx))
                    .child(self.home_background_row(cx))
                    .child(self.language_row(cx))
                    .child(self.while_playing_row(n + 3, cx))
                    .child(index_row(
                        n + 3,
                        tr("Show the Horizon logo at startup"),
                        Some(plain("Shown for a moment while the version list loads.")),
                        Some(opening.into_any_element()),
                        &t,
                    ))
                    .child(group("Network", "globe"))
                    .child(self.proxy_row(n + 4, cx))
                    .child(group("Game", "gamepad"))
                    .child(self.game_architecture_row(n + 5, cx))
                    .child(self.game_args_row(n + 6, cx))
                    .child(group("AI", "terminal"))
                    .child(self.no_ai_row(n + 6, cx))
                    .child(group("About", "info-box"))
                    .child(self.location_row(n + 9, cx))
                    .child(index_row(
                        n + 10,
                        tr("Logs"),
                        Some(mono(format!(
                            "{}  ·  {}",
                            short(&backend::logs_dir()),
                            tr("crash reports beside it")
                        ))),
                        Some(
                            button("open-logs", "Open", Kind::Secondary, &t)
                                .on_click(|_, _, cx| {
                                    let dir = backend::logs_dir();
                                    let _ = std::fs::create_dir_all(&dir);
                                    crate::glass_toast::open(cx, &dir);
                                })
                                .into_any_element(),
                        ),
                        &t,
                    ))
                    .when(cfg!(target_os = "linux"), |d| {
                        d.child(index_row(
                            n + 11,
                            tr("Applications menu"),
                            Some(plain(if self.settings.desktop_entry() {
                                "Horizon and its icon are in your desktop's menu."
                            } else {
                                "Not in your desktop's menu."
                            })),
                            Some(menu_entry.into_any_element()),
                            &t,
                        ))
                    }),
            )
            .children(self.agents_section(cx))
    }

    pub(crate) fn request_native_build_consent(&mut self) {
        self.native_consent_build = None;
        self.pending_consent = Some(PendingConsent::Build(
            native_status::enabled(&self.mods.home())
                .into_iter()
                .map(|(id, _, _)| id)
                .collect(),
        ));
    }

    /// Cancel and Escape share the transition; neither can execute a pending action.
    pub(crate) fn cancel_native_consent(&mut self) {
        self.play_after_build = None;
        self.native_consent_build = None;
        if let Err(error) = native_consent::resolve(
            &mut self.pending_consent,
            false,
            true,
            &mut self.settings,
            &self.mods.home(),
        ) {
            self.mods.message = Some(Err(error));
        }
        self.consent_error = None;
        self.instance_mods_changed();
    }

    /// The download question under way: Cancel starts nothing.
    pub(crate) fn cancel_mojang_consent(&mut self) {
        if let Err(error) =
            mojang_consent::resolve(&mut self.mojang_consent, false, &mut self.settings)
        {
            self.mojang_consent_error = Some(error);
        } else {
            self.mojang_consent_checked = false;
            self.mojang_consent_error = None;
        }
    }

    /// Allow the download and resume the action the player picked.
    pub(crate) fn allow_mojang_consent(&mut self) {
        match mojang_consent::resolve(&mut self.mojang_consent, true, &mut self.settings) {
            Err(error) => self.mojang_consent_error = Some(error),
            Ok(Some(MojangConsentRequest::Prepare)) => {
                self.mojang_consent_checked = false;
                self.mojang_consent_error = None;
                self.prepare();
            }
            Ok(Some(MojangConsentRequest::Java(id, server))) => {
                self.mojang_consent_checked = false;
                self.mojang_consent_error = None;
                match self.instances.list.iter().find(|i| i.id == id).cloned() {
                    Some(instance) => self.start_java(instance, server),
                    None => {
                        self.mojang_consent_error = Some(tr("That instance is gone.").to_string())
                    }
                }
            }
            Ok(None) => {}
        }
    }

    pub(crate) fn native_consent(&self, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        // The mods asked about, when known (a build with none enabled names none).
        let name = self
            .pending_consent
            .as_ref()
            .map(PendingConsent::name)
            .filter(|name| !name.is_empty());
        let build = matches!(self.pending_consent, Some(PendingConsent::Build(_)));
        let bytes = horizon_launcher::rust_toolchain::host().map(|host| host.download_bytes());
        let size = bytes
            .map(storage::format_bytes)
            .unwrap_or_else(|| tr("Unavailable on this platform").into());
        div().absolute().inset_0().occlude().flex().items_center().justify_center().bg(crate::glass_ui::scrim())
            .child(plate(&t).w(px(520.)).p(px(28.)).bg(t.surface).rounded(px(16.)).border_1().border_color(t.rule).shadow_lg().flex().flex_col().gap(px(18.))
                .child(div().size(px(48.)).rounded(px(12.)).bg(t.warn.opacity(0.12)).flex().items_center().justify_center().child(crate::glass_ui::icon("information-circle", 26., t.warn)))
                .child(div().text_size(px(24.)).font_weight(FontWeight::BOLD).child(tr("Allow native mods?")))
                .children(name.map(|name| div().font_weight(FontWeight::BOLD).child(name)))
                .child(tr("These mods are programs that run inside your game. Like any program you install, they can read and change your files. Allow them only if you trust who made them."))
                .children(horizon_launcher::rust_toolchain::installed().is_none().then(|| trf("Horizon will first download a compiler ({size}) to build them.", &[("size", &size)])))
                .child(tr("You can take this permission back in Settings > Storage."))
                .children(self.consent_error.clone().map(|error| div().text_color(t.danger).child(error)))
                .children(bytes.is_none().then(|| div().text_color(t.danger).child(tr("Native mods can't be built on this computer, so they can't be allowed here."))))
                .child(div().flex().justify_end().gap(px(12.))
                    .child(button("native-cancel", "Cancel", Kind::Secondary, &t).w(px(120.)).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.cancel_native_consent();
                        cx.notify();
                    })))
                    .child({
                        let allow = button("native-allow", if build { tr("Allow and build") } else { tr("Allow") }, Kind::Primary, &t).w(px(160.));
                        if bytes.is_none() {
                            disabled(allow)
                        } else {
                            allow.on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        match native_consent::resolve(&mut this.pending_consent, true, true, &mut this.settings, &this.mods.home()) {
                            Err(error) => this.consent_error = Some(error),
                            Ok(Some(action)) => { this.consent_error = None; this.storage_revoke_error = None; match action {
                                PendingConsent::Build(_) => {
                                    let (base, back_up) = this.native_consent_build.take().unwrap_or_default();
                                    this.build_native_on(base, back_up)
                                }
                                PendingConsent::Toggle(entry) => this.mods_switch(&entry),
                                PendingConsent::Install(source, id, known) => this.mods_install(source, id, known),
                                PendingConsent::Github(found, only) => this.github_install(found, only),
                                PendingConsent::Local(id) => {
                                    this.mods.message = horizon_launcher::browser::java::set_enabled(&this.mods.home(), &id, true).err().map(Err);
                                    this.instance_mods_changed();
                                }
                            } }
                            Ok(None) => {},
                        }
                        cx.notify();
                    }))
                        }
                    })
                ))
    }

    /// The generic disclaimer, once, before the launcher downloads Minecraft's files from
    /// Mojang's servers (#434). Not shown when the files are in an install or Horizon's own
    /// folder, or when the player already agreed.
    pub(crate) fn mojang_consent(&self, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        let checked = self.mojang_consent_checked;
        let checkbox = |id: &'static str, on: bool| {
            div()
                .id(id)
                .flex_none()
                .size(px(18.))
                .border_1()
                .border_color(if on { t.accent } else { t.muted })
                .bg(if on { t.accent } else { t.surface })
                .flex()
                .items_center()
                .justify_center()
                .text_color(t.on_accent)
                .text_size(px(12.))
                .cursor_pointer()
                .when(on, |d| d.child("✓"))
        };
        div().absolute().inset_0().occlude().flex().items_center().justify_center().bg(crate::glass_ui::scrim())
            .child(plate(&t).w(px(520.)).p(px(28.)).bg(t.surface).rounded(px(16.)).border_1().border_color(t.rule).shadow_lg().flex().flex_col().gap(px(18.))
                .child(div().size(px(48.)).rounded(px(12.)).bg(t.warn.opacity(0.12)).flex().items_center().justify_center().child(crate::glass_ui::icon("information-circle", 26., t.warn)))
                .child(div().text_size(px(24.)).font_weight(FontWeight::BOLD).child(tr("Download Minecraft's files?")))
                .child(tr("Horizon doesn't include Minecraft's files. It can download them from Mojang's servers, for any version you select, to this computer. The files are used only to play and are never shared."))
                .child(tr("You must own Minecraft: Java Edition. Horizon remembers your choice; take it back in Settings > Storage."))
                .child(div().flex().items_center().gap(px(10.))
                    .child(checkbox("mojang-own", checked).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.mojang_consent_checked = !this.mojang_consent_checked;
                        cx.notify();
                    })))
                    .child(div().font_weight(FontWeight::BOLD).child(tr("I own Minecraft: Java Edition"))))
                .children(self.mojang_consent_error.clone().map(|error| div().text_color(t.danger).child(error)))
                .child(div().flex().justify_end().gap(px(12.))
                    .child(button("mojang-cancel", "Cancel", Kind::Secondary, &t).w(px(120.)).on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.cancel_mojang_consent();
                        cx.notify();
                    })))
                    .child({
                        let allow = button("mojang-download", "Download", Kind::Primary, &t).w(px(120.));
                        if !checked {
                            disabled(allow)
                        } else {
                            allow.on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.allow_mojang_consent();
                                cx.notify();
                            }))
                        }
                    })
                ))
    }

    /// Settings' Game arguments row: the extra arguments the game starts with, kept in
    /// `settings.json` and appended to the launcher's own (quotes group a word with spaces).
    pub(crate) fn game_architecture_row(&self, n: usize, cx: &mut Context<Self>) -> Div {
        if !backend::can_choose_game_architecture() {
            return div();
        }
        let t = self.theme;
        let picker = segments(
            "game-architecture",
            &[
                (backend::GameArchitecture::Auto, "Automatic"),
                (backend::GameArchitecture::X64, "x64"),
                (backend::GameArchitecture::Arm64, "ARM64"),
            ],
            self.settings.game_architecture,
            &t,
            {
                let this = cx.entity().downgrade();
                move |choice, _, cx| {
                    let _ = this.update(cx, |this, cx| {
                        if this.settings.game_architecture == choice {
                            return;
                        }
                        this.settings.game_architecture = choice;
                        this.save_settings();
                        // The release and update lists contain the selected game's assets.
                        this.releases = Load::Loading;
                        this.fetch_startup_releases();
                        cx.notify();
                    });
                }
            },
        );
        index_row(
            n,
            tr("Game architecture"),
            Some(
                div()
                    .child(tr(
                        "Automatic follows this launcher's architecture. On Windows ARM, choose x64 or ARM64. Releases without ARM64 use the signed x64 build.",
                    ))
                    .into_any_element(),
            ),
            Some(picker.into_any_element()),
            &t,
        )
    }

    pub(crate) fn game_args_row(&self, n: usize, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        let focused = self.args_focused;
        let field = div()
            .id("game-args")
            .track_focus(&self.args_focus)
            .flex()
            .items_center()
            .w(px(300.))
            .h(px(30.))
            .px(px(12.))
            .overflow_hidden()
            .border_2()
            .border_color(if focused { t.accent } else { t.outline })
            .bg(t.well)
            .font_family(theme::MONO)
            .text_size(px(12.))
            .cursor_text()
            .child(line_edit::content(
                &self.args_input,
                &t,
                focused,
                false,
                false,
                tr("For example --server-thread"),
            ))
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                window.focus(&this.args_focus, cx);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, window, cx| {
                match line_edit::handle(&mut this.args_input, ev, cx) {
                    line_edit::Key::Pass => return,
                    line_edit::Key::Escape => window.blur(cx),
                    line_edit::Key::Edited => {
                        this.settings.game_args = this.args_input.text().to_string();
                        this.save_settings();
                    }
                    _ => {}
                }
                cx.stop_propagation();
                cx.notify();
            }));
        let field = line_edit::mouse(field, cx, |this| &mut this.args_input);
        if glass_ui::flat() {
            return glass_ui::setting_row_stacked(
                tr("Game arguments"),
                Some(
                    div()
                        .child(tr("Extra arguments for the game, passed through as-is (quotes group a word with spaces). Applies the next time the game starts."))
                        .into_any_element(),
                ),
                glass_ui::wide_field(field, focused).into_any_element(),
            );
        }
        index_row(
            n,
            tr("Game arguments"),
            Some(
                div()
                    .child(tr("Extra arguments for the game, passed through as-is (quotes group a word with spaces). Applies the next time the game starts."))
                    .into_any_element(),
            ),
            Some(field.into_any_element()),
            &t,
        )
    }

    /// Settings' Proxy row: the proxy every download goes through, kept in
    /// `settings.json` and read back by `proxy.rs` on each request.
    pub(crate) fn proxy_row(&self, n: usize, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        let focused = self.proxy_focused;
        let field = div()
            .id("proxy")
            .track_focus(&self.proxy_focus)
            .flex()
            .items_center()
            .w(px(300.))
            .h(px(30.))
            .px(px(12.))
            .overflow_hidden()
            .border_2()
            .border_color(if focused { t.accent } else { t.outline })
            .bg(t.well)
            .font_family(theme::MONO)
            .text_size(px(12.))
            .cursor_text()
            .child(line_edit::content(
                &self.proxy_input,
                &t,
                focused,
                false,
                false,
                tr("For example http://127.0.0.1:7890"),
            ))
            .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                window.focus(&this.proxy_focus, cx);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, window, cx| {
                match line_edit::handle(&mut this.proxy_input, ev, cx) {
                    line_edit::Key::Pass => return,
                    line_edit::Key::Escape => window.blur(cx),
                    line_edit::Key::Edited => this.save_proxy(),
                    _ => {}
                }
                cx.stop_propagation();
                cx.notify();
            }));
        let field = line_edit::mouse(field, cx, |this| &mut this.proxy_input);
        // A half-typed address is no reason to complain; one left behind is.
        let detail = match (
            focused,
            horizon_launcher::proxy::refuse(self.proxy_input.text()),
        ) {
            (false, Some(why)) => div().text_color(t.danger).child(why).into_any_element(),
            _ => div()
                .child(tr(
                    "Downloads and update installs go through this proxy (http://host:port, \
                     socks5://host:port). Empty means none: HTTPS_PROXY is used instead when the \
                     launcher was started with one.",
                ))
                .into_any_element(),
        };
        if glass_ui::flat() {
            return glass_ui::setting_row_stacked(
                tr("Proxy"),
                Some(detail),
                glass_ui::wide_field(field, focused).into_any_element(),
            );
        }
        index_row(
            n,
            tr("Proxy"),
            Some(detail),
            Some(field.into_any_element()),
            &t,
        )
    }

    /// Keeps the Proxy field in `settings.json`, and tells the parts that build their own agents
    /// (`horizon-port-pack`) which proxy to use.
    pub(crate) fn save_proxy(&mut self) {
        let value = self.proxy_input.text().trim().to_owned();
        self.settings.proxy = (!value.is_empty()).then_some(value);
        self.save_settings();
        horizon_launcher::proxy::announce(self.settings.proxy.clone());
    }
}
