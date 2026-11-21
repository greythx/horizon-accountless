//! The launcher's state set up (`Launcher::new`), its settings saved, page changes and the
//! release list.
use super::*;

impl Launcher {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        horizon_launcher::storage::eviction::cleanup_after_start();
        let (tx, rx) = channel();
        let appearance = cx.observe_window_appearance(window, |this, window, cx| {
            this.theme = this.settings_theme(window);
            cx.notify();
        });
        let window_handle = window.window_handle();
        cx.spawn(async move |this, cx| loop {
            cx.background_executor()
                .timer(Duration::from_millis(80))
                .await;
            if this
                .update(cx, |this, cx| this.drain(window_handle, cx))
                .is_err()
            {
                break;
            }
        })
        .detach();
        let mut settings = Settings::load();
        i18n::set(settings.language());
        if settings.minecraft_dir.is_none() {
            settings.minecraft_dir = backend::default_minecraft_dir();
        }
        // Release builds on Linux put their entry back in the applications menu when there is
        // none or it starts a program that is gone (a moved or deleted binary), never over one
        // that works; `HORIZON_LAUNCHER_DESKTOP_ENTRY=1` does it in a development build.
        if cfg!(target_os = "linux")
            && settings.desktop_entry()
            && (!cfg!(debug_assertions)
                || std::env::var_os("HORIZON_LAUNCHER_DESKTOP_ENTRY").is_some())
        {
            std::thread::spawn(|| {
                let _ = horizon_launcher::desktop::repair(backend::exe_path());
            });
        }
        let installed = backend::installed();
        if settings
            .selected
            .as_ref()
            .is_none_or(|s| !installed.contains(s))
        {
            settings.selected = installed.first().cloned();
        }
        // Pin the window's chrome (macOS title bar) to the choice before the theme reads
        // `window.appearance()`, so both draw the same look.
        titlebar::apply_appearance(window, settings.appearance);
        let scene = cx.new(|_| scene::Scene::new());
        let sky = cx.new(|cx| sky::SkyView::new(window, cx));
        let rain = cx.new(|_| rain::Rain::new());
        let backdrop =
            cx.new(|_| window_root::Backdrop::new(scene.clone(), sky.clone(), rain.clone()));
        let mut this = Self {
            theme: crate::theme::from_settings(&settings, window.appearance()),
            page: Page::from_env(),
            behind: Page::Play,
            switches: 0,
            content_scroll: Scrollbar::default(),
            nav_strip: nav_strip::NavStrip::default(),
            settings_pane: gpui::ScrollHandle::new(),
            settings_query: line_edit::LineEdit::new(String::new(), 80, 40),
            settings_query_focus: cx.focus_handle(),
            settings_query_focused: false,
            whats_new_open: None,
            settings_marked: 0,
            credit_asked: false,
            version_menu: false,
            credit_picture: None,
            sheet_size: None,
            mods_sticky: sticky::Sticky::default(),
            long_list: cull::Cull::default(),
            opening: (!settings.skip_opening).then(|| {
                let focus = cx.focus_handle();
                window.focus(&focus, cx);
                Opening {
                    started: None,
                    reveal: None,
                    focus,
                }
            }),
            checked: None,
            instance_install: None,
            install_next: None,
            bar: (0., 0.),
            releases: Load::Loading,
            installed,
            installing: None,
            removing: Vec::new(),
            disk: Vec::new(),
            storage_report: None,
            storage_pending: false,
            storage_confirm: None,
            storage_error: None,
            storage_revoke: None,
            storage_counting: None,
            storage_visit: 0,
            storage_revoke_error: None,
            native_consent_build: None,
            disk_for: Vec::new(),
            install_error: None,
            importing_build: false,
            import_error: None,
            native_step: None,
            native_progress: None,
            first_build: None,
            play_after_build: None,
            native_done: None,
            native_error: None,
            native_failed_mods: Vec::new(),
            native_worlds: None,
            pending_consent: None,
            consent_error: None,
            mojang_consent: None,
            mojang_consent_checked: false,
            mojang_consent_error: None,
            native_for: None,
            native_built_mods: Vec::new(),
            rooms: Default::default(),
            native_builds: Default::default(),
            native_log: Default::default(),
            confirm_remove: None,
            notes_open: Default::default(),
            notes: Default::default(),
            technical_open: Default::default(),
            picker: glass_home::PickerState::default(),
            accent_strip: false,
            look_sliders: Default::default(),
            hue_area: std::rc::Rc::new(std::cell::Cell::new(None)),
            hue_dragging: false,
            language_menu: false,
            row_menu: Default::default(),
            rail: Default::default(),
            sheet_tab: Default::default(),
            look: horizon_theme::glass::Glass::default_look(),
            menu_closed_at: None,
            // The first draw takes the launcher theme from Settings.
            restyle: true,
            background_error: None,
            location_target: None,
            location_error: location_page::startup_error(),
            location_note: location_page::startup_note(),
            args_input: line_edit::LineEdit::new(settings.game_args.clone(), 1024, 38),
            args_focused: false,
            args_focus: cx.focus_handle(),
            proxy_input: line_edit::LineEdit::new(
                settings.proxy.clone().unwrap_or_default(),
                256,
                44,
            ),
            proxy_focused: false,
            proxy_focus: cx.focus_handle(),
            settings,
            settings_save_error: None,
            preparing: false,
            prepare_log: VecDeque::new(),
            log_revision: 0,
            prepare_error: None,
            client: None,
            client_error: None,
            client_started: None,
            client_home: None,
            crash: None,
            servers: servers_page::Servers::new(),
            internet: internet::Internet::new(),
            root_focus: cx.focus_handle(),
            palette: None,
            toast_seen: None,
            starting_client: None,
            starting_instance: None,
            java_downloading: false,
            play_when_prepared: None,

            window_width: 0.,
            window_long_side: 0.,
            scale_factor: 1.,
            joining: None,
            quick_play: None,
            client_run: None,
            client_vars: Vec::new(),
            agents: agents_page::AgentsUi::default(),
            playing: None,
            tx,
            rx,
            _appearance: appearance,
            _activation: cx.observe_window_activation(window, |_, _, cx| cx.notify()),
            perf: perf::Perf::new("session"),
            install_started: None,
            prepare_started: None,
            download_seen: None,
            home_tab: home_tabs::HomeTab::from_env(),
            home_menu: Default::default(),
            folder_hovered: false,
            list_hover: None,
            list_menu: None,
            home_strip: Default::default(),
            client_since: None,
            instance_mods: Default::default(),
            mod_preview: Default::default(),
            project_icons: Default::default(),
            play_pressed: None,
            bench: perf::Bench::from_env(),
            packs: Vec::new(),
            registry_packs: Vec::new(),
            packs_loaded: None,
            pack_open: None,
            pack_filter: None,
            pack_busy: false,
            pack_log: VecDeque::new(),
            pack_error: None,
            found_packs: Vec::new(),
            found_scanning: false,
            starting_pack: None,
            mods: mods_page::Mods::new(cx),
            github: github_mods_page::Github::new(cx),
            instances: instances_page::Instances::default(),
            worlds: worlds_page::Worlds::default(),
            multiplayer: multiplayer_page::Multiplayer::default(),
            screenshots: screenshots_page::Screenshots::default(),
            shot_blur: Default::default(),
            pictures: Default::default(),
            game_packs: packs_page::Packs::default(),
            world_packs: datapacks_page::WorldPacks::default(),
            export: None,
            launch: None,
            memory_slider: Default::default(),
            importing: Default::default(),
            console: Default::default(),
            aside: while_playing::Aside::new(window.window_handle()),
            shortcuts: Default::default(),
            side_servers: horizon_launcher::servers::list(),
            sky,
            scene,
            rain,
            backdrop,
            pushed: None,
            scale: window.scale_factor(),
        };
        if this.location_error.is_some() || this.location_note.is_some() {
            this.page = Page::Settings;
        }
        this.reload_instances();
        if this.bench.is_some() {
            this.perf.keep_local_log(false);
        }
        let tx = this.tx.clone();
        std::thread::spawn(move || {
            let reused = backend::reuse_prepared_everywhere();
            if !reused.is_empty() {
                let _ = tx.send(Msg::Reused(reused));
            }
        });
        this.fetch_startup_releases();
        this.mods_auto_update();
        if home::getting_ready_preview() {
            this.preparing = true;
            let tag = this.selected().cloned();
            let instance = this.selected_instance().map(|i| i.id.clone());
            this.play_when_prepared = tag.map(|tag| (tag, instance));
        }
        // `HORIZON_LAUNCHER_PAGE=new-instance`: the dialog open on Modded, for screenshots
        // (`new-instance-horizon`, `new-instance-vanilla`: on those).
        let kind = match std::env::var("HORIZON_LAUNCHER_PAGE").as_deref() {
            Ok("new-instance") => Some(new_instance::NewKind::Modded),
            Ok("new-instance-horizon") => Some(new_instance::NewKind::Horizon),
            Ok("new-instance-vanilla") => Some(new_instance::NewKind::Vanilla),
            _ => None,
        };
        if let Some(kind) = kind {
            this.open_instance_dialog(None);
            this.pick_kind(kind, cx);
        }
        if std::env::var("HORIZON_LAUNCHER_PAGE").as_deref() == Ok("edit-instance") {
            this.sheet_tab = glass_sheet::SheetTab::from_env().unwrap_or_default();
            this.open_instance_dialog(this.selected_instance().cloned());
        }
        // What the game and the launcher kept, if the player allows it and there is a server.
        this.mod_preview_from_env();
        this
    }

    /// Keeps the current choices in memory if writing fails; every page offers the same retry.
    pub(crate) fn save_settings(&mut self) -> bool {
        self.settings_save_error = self.settings.save().err();
        self.settings_save_error.is_none()
    }

    pub(crate) fn settings_save_notice(&self, cx: &mut Context<Self>) -> Option<Div> {
        let error = self.settings_save_error.as_ref()?;
        let t = self.theme;
        Some(
            div()
                .flex_none()
                .px(px(theme::PAGE_GUTTER))
                .py(px(12.))
                .child(
                    crate::plate::plate(&t.night)
                        .flex()
                        .items_center()
                        .gap(px(16.))
                        .p(px(16.))
                        .child(
                            div()
                                .relative()
                                .flex_1()
                                .min_w_0()
                                .child(
                                    shell::text(theme::night::UI, t.danger)
                                        .child(tr("Settings were not saved.")),
                                )
                                .child(
                                    shell::text(theme::night::CAPTION, t.ink_soft).child(trf(
                                        "Your changes are still in this window. Retry to save them. {error}",
                                        &[("error", error)],
                                    )),
                                ),
                        )
                        .child(
                            button("settings-save-retry", "Retry", Kind::Secondary, &t)
                                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                    this.save_settings();
                                    cx.notify();
                                })),
                        ),
                ),
        )
    }

    pub(crate) fn go(&mut self, page: Page) {
        // A Turn off question left open isn't still asked the next time Settings opens.
        self.storage_revoke = None;
        // A Turn off error is read on the visit it shows in: leaving Settings settles it.
        if self.page == Page::Settings && page != Page::Settings {
            self.storage_revoke_error = None;
        }
        self.storage_counting = None;
        self.storage_visit += 1;
        // The glass look's Settings is a dialog over the page: opening or closing it keeps the
        // page under it as it was (its scroll, no fade).
        let dialog = self.theme.glass
            && (page == Page::Settings || (self.page == Page::Settings && page == self.behind));
        if page != Page::Settings {
            self.behind = page;
        }
        if page != self.page && dialog {
            self.page = page;
        } else if page != self.page {
            self.page = page;
            self.switches += 1;
            self.content_scroll = Scrollbar::default();
            self.long_list = cull::Cull::default();
            self.mods_sticky.reset();
            if page == Page::Mods {
                self.mods.reset_lists();
            }
        }
        self.picker.open = false;
        self.language_menu = false;
        self.mods.browse.sort_menu = false;
    }

    // The Horizon version being installed as an update: removed in this fork (no self-update).
    /// The native mods turned on in the instance Play starts, unpacked and checked, with their
    /// bundles: what the warning names and the build compiles.
    pub(crate) fn native_mods_to_build(
        &self,
    ) -> Result<(std::path::PathBuf, Vec<native_mods::Bundle>), String> {
        let folder = native_mods::installed_mods(&instances::selected().home)?;
        let found = native_mods::bundles(&folder)?;
        Ok((folder, found))
    }

    /// What installed native build `version` was made from (`native_status::Build`), read once.
    pub(crate) fn native_build_of(&self, version: &str) -> Option<native_status::Build> {
        if !native_mods::is_native(version) {
            return None;
        }
        self.native_builds
            .borrow_mut()
            .entry(version.to_owned())
            .or_insert_with(|| native_status::Build::read(version))
            .clone()
    }

    pub(crate) fn fetch_releases(&mut self) {
        self.releases = Load::Loading;
        self.notes.clear();
        self.checked = Some(Instant::now());
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(Msg::Releases(backend::releases()));
        });
    }

    pub(crate) fn fetch_startup_releases(&mut self) {
        self.checked = Some(Instant::now());
        let tx = self.tx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(Msg::Releases(backend::releases()));
        });
    }
}
