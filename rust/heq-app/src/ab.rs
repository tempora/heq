use eframe::egui::{self, Ui};

use heq_core::model::{AbTester, EqModel};

use crate::library::Library;
use crate::theme;

// The side you hear is the live model; the pill only says which side that is.
pub fn pill(
    ui: &mut Ui,
    ab: &mut AbTester,
    library: &mut Library,
    model: &mut EqModel,
) {
    let active = ab.active();

    ui.scope(|ui| {
        if !active {
            ui.disable();
        }

        let on_b = ab.on_b();
        let mut want = on_b;

        if ui.selectable_label(!on_b, "A").clicked() {
            want = false;
        }
        if ui.selectable_label(on_b, "B").clicked() {
            want = true;
        }

        if want != on_b {
            library.switch_ab(want, model, ab);
        }
    });

    let tip = if active {
        format!(
            "A · {}    B · {}",
            side_name(ab.a_name()),
            side_name(ab.b_name())
        )
    } else {
        "Set a preset as B to compare against it".to_string()
    };
    ui.label(egui::RichText::new(tip).color(theme::TEXT_DIM))
        .on_hover_text("right-click a preset to set it as B");

    if active && ui.button("×").on_hover_text("Clear B").clicked() {
        library.clear_b(model, ab);
    }
}

fn side_name(name: Option<&str>) -> &str {
    match name {
        Some(n) if !n.is_empty() => n,
        _ => "working EQ",
    }
}
