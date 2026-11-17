//! The first-run welcome: the player picks how the launcher looks: theme, accent, each flourish
//! (glow, gradients, the hover shine) and motion, then Continue. The rows are Settings' own,
//! so a choice made here is the same one Settings shows later.
use super::*;
use crate::glass_ui::{self as ui, white};
use crate::plate;
use horizon_theme::glass::{ink, radius};

impl Launcher {
    /// Whether the welcome covers the window: a first run, not welcomed before, and the opening
    /// over.
    pub(crate) fn welcoming(&self) -> bool {
        self.opening.is_none() && !self.settings.welcomed
    }

    /// Escape answers the welcome as Continue does: the look as it is now stays.
    pub(crate) fn welcome_escape(&mut self) -> bool {
        if self.welcoming() {
            self.settings.welcomed = true;
            self.save_settings();
            return true;
        }
        false
    }

    pub(crate) fn welcome_screen(&mut self, cx: &mut Context<Self>) -> Stateful<Div> {
        let t = self.theme;
        let rows = ui::flat_rows(|| {
            let mut rows = vec![self.launcher_theme_row(cx), self.glass_accent_row(cx)];
            rows.push(self.motion_row(cx));
            rows.extend(self.effects_rows(cx));
            rows
        });
        let head = div()
            .flex()
            .flex_col()
            .flex_none()
            .gap(px(6.))
            .px(px(28.))
            .pt(px(26.))
            .pb(px(14.))
            .child(
                div()
                    .font_family(horizon_theme::glass::DISPLAY_FONT)
                    .font_size(px(30.))
                    .font_weight(FontWeight::BOLD)
                    .child(tr("How should Horizon look?")),
            )
            .child(
                ui::text(15., FontWeight::NORMAL, ink::QUIET)
                    .child(tr("Everything you pick here can be changed later in Settings.")),
            );
        let cont = ui::quiet("welcome-continue", tr("Continue"), 44., &t)
            .justify_center()
            .w(px(320.))
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.settings.welcomed = true;
                this.save_settings();
                cx.notify();
            }));
        div()
            .absolute()
            .inset_0()
            .occlude()
            .bg(gpui::black().opacity(0.55))
            .flex()
            .items_center()
            .justify_center()
            .child(
                plate(&t.night)
                    .id("welcome")
                    .w(px(520.))
                    .max_w(px(560.))
                    .flex()
                    .flex_col()
                    .child(head)
                    .child(div().max_h(px(380.)).overflow_y_scroll().px(px(28.)).pb(px(12.)).children(rows))
                    .child(div().flex().justify_center().pb(px(26.)).child(cont)),
            )
    }
}
