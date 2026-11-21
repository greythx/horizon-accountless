//! Home in the night look (docs/design-system.md, "Night"): the selected instance's name over the
//! sky with what it plays and whether it is ready, and Play and the actions beside it.
use super::*;
use crate::plate::alpha;
use crate::shell::{action, c, label, Action};
use theme::night;

impl Launcher {
    /// The focal point: what will start and its state, the instance's name, when it was played.
    /// No `tag`: a Java instance's Minecraft (`game` then names its loader too).
    pub(super) fn hero(&mut self, tag: &str, game: &str, prepared: bool) -> Div {
        let t = self.theme;
        let n = t.night;
        let icon = self
            .selected_instance()
            .cloned()
            .map(|i| self.instance_icon(&i, 56.));
        let instance = self.selected_instance();
        let name = instance.map_or(
            if tag.is_empty() {
                "Minecraft"
            } else {
                "Horizon"
            },
            |i| i.name.as_str(),
        );
        let mut what = vec![format!("Minecraft: Java Edition {game}")];
        if !tag.is_empty() {
            what.push(format!(
                "Horizon {}",
                horizon_launcher::backend::nightly_label(tag)
                    .or_else(|| horizon_launcher::build_import::label(tag))
                    .unwrap_or_else(|| tag.trim_start_matches('v').to_string())
            ));
        }
        what.push(tr(if prepared { "Ready" } else { "Needs preparing" }).to_string());
        let state = div()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(
                div()
                    .flex_none()
                    .size(px(6.))
                    .bg(if prepared { t.success } else { t.warn }),
            )
            .child(
                shell::text(night::CAPTION, c(n.ink_soft))
                    .font_weight(FontWeight::MEDIUM)
                    .flex()
                    .gap(px(4.))
                    .child(what.join(" · ")),
            );
        // Never squeezed by the rows below.
        div()
            .flex()
            .flex_none()
            .flex_col()
            .gap(px(10.))
            .child(state)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(16.))
                    .children(icon)
                    .child(
                        shell::text(night::DISPLAY, c(n.ink))
                            .min_w_0()
                            .truncate()
                            .child(name.to_string()),
                    ),
            )
            .children(instance.map(|i| {
                shell::text(night::UI, c(n.ink_soft)).child(instances_page::played_line(i))
            }))
    }

    /// Play, then what else the instance offers: its version (Horizon), its mods, its folder.
    /// `ready`: prepared, or a Java instance (its game comes with Play).
    pub(super) fn home_actions(&self, ready: bool, java: bool, cx: &mut Context<Self>) -> Div {
        let t = self.theme;
        let running = self.client.is_some() || self.starting_client.is_some();
        if self.preparing || self.java_downloading {
            // Play waits for the steps below it; a download can stop (the next one goes on from
            // there). A Java instance's game files download under the same bar as a prepare.
            let download = download_progress();
            let busy_text = busy_label(self.preparing, self.java_downloading);
            let busy = match download {
                Some((done, total)) => trf(
                    "{status} {percent}",
                    &[
                        ("status", busy_text),
                        (
                            "percent",
                            &format!("{:.0}%", done as f32 * 100. / total as f32),
                        ),
                    ],
                ),
                None => busy_text.to_string(),
            };
            return div()
                .flex()
                .items_center()
                .gap(px(10.))
                .child(
                    action("play", Action::Main, &t)
                        .min_w(px(260.))
                        .cursor_default()
                        .child(label(busy)),
                )
                .children(download.map(|_| {
                    action("download-cancel", Action::Quiet, &t)
                        .child(label(tr("Cancel").to_string()))
                        .on_click(|_, _, _| {
                            horizon_launcher::mojang::CANCEL.store(true, Ordering::Relaxed)
                        })
                }));
        }
        let mut play = action("play", Action::Main, &t)
            .min_w(px(180.))
            .child(
                div()
                    .relative()
                    .child(blocks::icon("play", 20., c(t.night.on_accent))),
            )
            .child(label(
                tr(if running { "Running" } else { "Play" }).to_string(),
            ));
        match play_press(
            ready,
            // No account verdict in this fork: an absent owner never disables Play.
            Some(true),
            self.preparing || running,
        ) {
            PlayPress::Nothing => play = disabled(play),
            PlayPress::Play => {
                play = play.on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.picker.open = false;
                    this.play();
                    cx.notify();
                }))
            }
            PlayPress::PrepareThenPlay => {
                play = play.on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.picker.open = false;
                    let tag = this.selected().cloned();
                    let instance = this.selected_instance().map(|i| i.id.clone());
                    this.play_when_prepared = tag.map(|tag| (tag, instance));
                    this.prepare();
                    cx.notify();
                }))
            }
        }
        let folder = self.selected_instance().map(|i| i.home.clone());
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(10.))
            .child(play)
            // Horizon's versions don't apply to a Java instance.
            .children((!java).then(|| self.picker(cx)))
            .child(
                action("home-mods", Action::Quiet, &t)
                    .child(label(tr("Mods").to_string()))
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.go(Page::Mods);
                        cx.notify();
                    })),
            )
            .children(folder.map(|home| {
                action("home-folder", Action::Quiet, &t)
                    .child(label(tr("Folder").to_string()))
                    .on_click(move |_, _, cx| {
                        let _ = std::fs::create_dir_all(&home);
                        crate::glass_toast::open(cx, &home);
                    })
            }))
    }
}

impl Launcher {
    /// How long the download under way has left, from its pace since it was first seen: none
    /// for its first seconds, or without one.
    pub(super) fn download_left(&mut self) -> Option<Duration> {
        if getting_ready_preview() {
            return Some(Duration::from_secs(70));
        }
        let Some((done, total)) = horizon_launcher::mojang::progress() else {
            self.download_seen = None;
            return None;
        };
        let (since, from) = match self.download_seen {
            // A new Java download starts counting again.
            Some(seen) if seen.1 <= done => seen,
            _ => {
                self.download_seen = Some((Instant::now(), done));
                return None;
            }
        };
        time_left(since.elapsed(), from, done, total)
    }

    /// Under Play while it prepares: a bar, what is happening and the time left, and the steps,
    /// the one under way lit, those done ticked.
    pub(super) fn steps(&self, left: Option<Duration>) -> Div {
        let t = self.theme;
        let n = t.night;
        let now = backend::prepare_step();
        let download = download_progress();
        let doing = match (download, now) {
            (Some((done, total)), _) => trf(
                "Downloading Minecraft's Java from Mojang · {done} of {total} files",
                &[
                    ("done", &thousands(done as u64)),
                    ("total", &thousands(total as u64)),
                ],
            ),
            (None, _) => self.prepare_log.back().cloned().unwrap_or_default(),
        };
        let left = left.map(|d| match d.as_secs().div_ceil(60) {
            0 | 1 => tr("about a minute left").to_string(),
            m => trf("about {n} min left", &[("n", &m.to_string())]),
        });
        // Play is the last step when it was pressed, not Prepare.
        let play = self.play_when_prepared.is_some();
        let pill = |name: &str, state: std::cmp::Ordering| {
            let (mark, color) = match state {
                std::cmp::Ordering::Less => (
                    Some(blocks::icon("check", 12., c(n.success))),
                    c(n.ink_soft),
                ),
                std::cmp::Ordering::Equal => (None, c(n.ink)),
                std::cmp::Ordering::Greater => (None, c(n.muted)),
            };
            div()
                .flex()
                .items_center()
                .gap(px(6.))
                .child(match mark {
                    Some(icon) => icon.into_any_element(),
                    None => div()
                        .size(px(6.))
                        .bg(if state == std::cmp::Ordering::Equal {
                            c(n.accent_hi)
                        } else {
                            alpha(n.muted, 0.5).into()
                        })
                        .into_any_element(),
                })
                .child(
                    shell::text(night::CAPTION, color)
                        .font_weight(FontWeight::MEDIUM)
                        .child(tr(name).to_string()),
                )
        };
        div()
            .flex()
            .flex_col()
            .gap(px(10.))
            .max_w(px(620.))
            .child(div().flex().child(progress_bar(
                download.map(|(done, total)| {
                    let f = done as f32 / total as f32;
                    (f, f)
                }),
                &t,
            )))
            .child(
                div()
                    .flex()
                    .justify_between()
                    .gap(px(16.))
                    .child(
                        shell::text(night::CAPTION, c(n.ink_soft))
                            .min_w_0()
                            .truncate()
                            .child(doing),
                    )
                    .children(
                        left.map(|l| shell::text(night::CAPTION, c(n.muted)).flex_none().child(l)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(16.))
                    .children(
                        backend::PrepareStep::ALL
                            .iter()
                            .map(|s| pill(s.name(), s.cmp(&now))),
                    )
                    .children(play.then(|| pill("Play", std::cmp::Ordering::Greater))),
            )
    }
}

/// What Play says while files are fetched: a Java instance downloads its game, it isn't preparing
/// a Horizon version. Both strings are translation keys.
fn busy_label(preparing: bool, java_downloading: bool) -> &'static str {
    if java_downloading && !preparing {
        tr("Downloading game files")
    } else {
        tr("Getting ready…")
    }
}

/// `HORIZON_LAUNCHER_PAGE=getting-ready` opens Home as if Play had been pressed and the game's
/// Java were a third of the way through its download: for screenshots, nothing runs.
pub(super) fn getting_ready_preview() -> bool {
    static PREVIEW: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *PREVIEW
        .get_or_init(|| std::env::var("HORIZON_LAUNCHER_PAGE").as_deref() == Ok("getting-ready"))
}

/// The download under way, files done of how many.
fn download_progress() -> Option<(usize, usize)> {
    horizon_launcher::mojang::progress().or_else(|| getting_ready_preview().then_some((1754, 5160)))
}

/// The time `total - done` files take at the pace of `done - from` in `spent`: none for the
/// first 5 s, or before a file is done.
fn time_left(spent: Duration, from: usize, done: usize, total: usize) -> Option<Duration> {
    (spent >= Duration::from_secs(5) && done > from)
        .then(|| spent.mul_f64(total.saturating_sub(done) as f64 / (done - from) as f64))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_left_follows_the_pace_since_first_seen() {
        let s = Duration::from_secs;
        assert_eq!(time_left(s(10), 100, 600, 3100), Some(s(50)));
        assert_eq!(time_left(s(4), 100, 600, 3100), None, "too early");
        assert_eq!(time_left(s(10), 600, 600, 3100), None, "nothing done yet");
        assert_eq!(time_left(s(10), 0, 50, 50), Some(s(0)));
    }

    #[test]
    fn a_java_download_says_it_is_downloading() {
        assert_eq!(busy_label(false, true), tr("Downloading game files"));
        assert_eq!(busy_label(true, false), tr("Getting ready…"));
        assert_eq!(busy_label(true, true), tr("Getting ready…"));
    }

    #[test]
    fn steps_run_in_their_order() {
        use backend::PrepareStep::*;
        assert_eq!(backend::PrepareStep::ALL, [Files, Java, Sounds, Prepare]);
        for s in backend::PrepareStep::ALL {
            assert_eq!(backend::PrepareStep::ALL[s as usize], s);
        }
    }
}
