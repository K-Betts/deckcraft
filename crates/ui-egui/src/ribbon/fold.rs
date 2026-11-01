//! The responsive ribbon: groups that do not fit the window fold into a More menu instead of
//! being cut off.

use egui::{Rect, Ui, vec2};

use super::render_tab;
use crate::SlideApp;
use crate::icons::Icon;
use crate::widgets::big_button;

/// The width reserved at the right of the card for the "More" button.
const MORE_W: f32 = 66.0;

/// Draw the current tab's groups, folding the ones that do not fit into a "More" menu.
///
/// The tab functions just call `group` in order. Each group's width is recorded when it is drawn
/// and remembered per tab, so the next frame can keep whole groups left to right while they fit
/// and put the rest behind a More button; its menu runs the same tab function again drawing only
/// the groups that were left out. The first time a tab is shown (widths unknown) every group is
/// drawn once, clipped as the ribbon always was, which teaches the widths; one frame later it
/// folds. Nothing is lost in a narrow window, and nothing invisible is ever built (so no phantom
/// controls for screen readers).
pub(super) fn responsive_tab(app: &mut SlideApp, cui: &mut Ui, inner: Rect) {
    if !app.maths.ribbon_fold {
        app.maths.ribbon_folded = 0;
        render_tab(app, cui);
        return;
    }
    let ctx = cui.ctx().clone();
    let tab = app.ui.tab.clone();
    let known: Vec<f32> = app.maths.ribbon_widths.get(&tab).cloned().unwrap_or_default();
    // Decide how many whole groups fit, from what was learned earlier (everything when unknown).
    let n_known = known.len();
    let complete = n_known > 0 && known.iter().all(|w| *w > 0.0);
    let cut = if complete { groups_that_fit(&known, inner.width()) } else { usize::MAX };
    app.maths.ribbon_folded = if complete { n_known - cut } else { 0 };
    set_fold(&ctx, Fold { pass: FoldPass::Show, cut, ..Default::default() });
    render_tab(app, cui);
    let seen = take_fold(&ctx);
    // Learn the widths of the groups just drawn; a change in the number of groups starts over.
    let mut learned = if seen.index == n_known { known.clone() } else { vec![0.0; seen.index] };
    for (i, w) in &seen.measured {
        if let Some(slot) = learned.get_mut(*i) {
            *slot = *w;
        }
    }
    if learned != known {
        app.maths.ribbon_widths.insert(tab, learned.clone());
        ctx.request_repaint();
    }
    if complete && cut < n_known {
        let more = big_button(cui, Icon::More, "More", true);
        more.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, "More tools"));
        let more = more.on_hover_text("More tools");
        // The folded groups, in a menu.
        egui::Popup::menu(&more).width(640.0).show(|ui| {
            ui.set_max_width(640.0);
            ui.spacing_mut().item_spacing = vec2(1.0, 6.0);
            set_fold(&ctx, Fold { pass: FoldPass::Overflow, cut, ..Default::default() });
            ui.horizontal_wrapped(|ui| render_tab(app, ui));
            take_fold(&ctx);
        });
    }
}

/// How many leading groups fit in `available` points, leaving room for a More button when some do not.
pub(crate) fn groups_that_fit(widths: &[f32], available: f32) -> usize {
    let total: f32 = widths.iter().sum();
    if total <= available {
        return widths.len();
    }
    let room = available - MORE_W;
    let mut used = 0.0;
    let mut n = 0;
    for w in widths {
        if used + w > room {
            break;
        }
        used += w;
        n += 1;
    }
    n
}

/// Which pass of the responsive ribbon is laying out groups right now (see `ribbon::show`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FoldPass {
    /// An ordinary layout: every group is drawn (dialogs, tests, anything outside the ribbon).
    #[default]
    Off,
    /// The visible ribbon: groups before `cut` are drawn (and their widths recorded), the rest are skipped.
    Show,
    /// The "More" menu: groups from `cut` on are drawn, the earlier ones are skipped.
    Overflow,
}

/// The responsive ribbon's state for the pass in progress, kept in egui's temporary memory so the
/// tab functions need no extra parameter: they just call [`group`] in order.
#[derive(Clone, Debug, Default)]
pub struct Fold {
    pub pass: FoldPass,
    /// How many groups `group` has seen so far in this pass (at the end: how many the tab has).
    pub index: usize,
    /// `(group index, width in points)` of the groups drawn in this pass.
    pub measured: Vec<(usize, f32)>,
    pub cut: usize,
}

fn fold_id() -> egui::Id {
    egui::Id::unique("deckcraft.ribbon.fold")
}

pub fn set_fold(ctx: &egui::Context, fold: Fold) {
    ctx.data_mut(|d| d.insert_temp(fold_id(), fold));
}

pub fn take_fold(ctx: &egui::Context) -> Fold {
    ctx.data_mut(|d| d.remove_temp::<Fold>(fold_id())).unwrap_or_default()
}

/// A ribbon group (`widgets::group`): inside the responsive ribbon it may be drawn or skipped
/// according to the current [`FoldPass`]; anywhere else it is simply drawn.
pub(crate) fn group<R: Default>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    let ctx = ui.ctx().clone();
    let (pass, index, cut) = ctx.data_mut(|d| match d.get_temp_mut_or_default::<Fold>(fold_id()) {
        f if f.pass == FoldPass::Off => (FoldPass::Off, 0, 0),
        f => {
            let i = f.index;
            f.index += 1;
            (f.pass, i, f.cut)
        }
    });
    let draw = |ui: &mut Ui| {
        let x0 = ui.cursor().min.x;
        let r = ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing = vec2(1.0, 1.0);
            add(ui)
        });
        crate::widgets::separator(ui);
        (r.inner, ui.cursor().min.x - x0)
    };
    match pass {
        FoldPass::Off => draw(ui).0,
        FoldPass::Show if index < cut => {
            let (r, w) = draw(ui);
            ctx.data_mut(|d| d.get_temp_mut_or_default::<Fold>(fold_id()).measured.push((index, w)));
            r
        }
        FoldPass::Overflow if index >= cut => draw(ui).0,
        _ => R::default(),
    }
}
