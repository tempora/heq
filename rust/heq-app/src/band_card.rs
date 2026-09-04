use eframe::egui::{self, ComboBox, DragValue, Ui};

use heq_core::model::{BandId, ChannelTarget, EqModel, FilterKind};

use crate::curve::CurveState;

const SLOPES: [i32; 4] = [12, 24, 36, 48];

pub fn show(ctx: &egui::Context, model: &mut EqModel, state: &mut CurveState) {
    let Some(id) = state.selected else { return };
    let Some(band) = model.band(id).copied() else {
        state.selected = None;
        return;
    };

    egui::Window::new("band")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(12.0, -12.0))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("{}", index_of(model, id) + 1));
                kind_picker(ui, model, id, band.kind);

                number(ui, "Hz", band.freq, 1.0, |v| {
                    model.edit(id, |b| b.freq = v)
                });

                if band.kind.uses_gain() {
                    number(ui, "dB", band.gain_db, 0.1, |v| {
                        model.edit(id, |b| b.gain_db = v)
                    });
                }

                if band.kind.uses_slope() {
                    slope_picker(ui, model, id, band.slope_db_per_oct);
                } else {
                    number(ui, "Q", band.q, 0.01, |v| model.edit(id, |b| b.q = v));
                }

                channel_picker(ui, model, id, band.channel);

                let bypass = if band.enabled { "bypass" } else { "enable" };
                if ui.button(bypass).clicked() {
                    model.edit(id, |b| b.enabled = !b.enabled);
                }
                if ui.button("delete").clicked() {
                    model.remove_band(id);
                    state.selected = None;
                }
            });
        });
}

fn index_of(model: &EqModel, id: BandId) -> usize {
    model.bands().iter().position(|b| b.id == id).unwrap_or(0)
}

fn kind_picker(ui: &mut Ui, model: &mut EqModel, id: BandId, current: FilterKind) {
    ComboBox::from_id_salt("band_kind")
        .selected_text(current.display_name())
        .width(96.0)
        .show_ui(ui, |ui| {
            for kind in FilterKind::ALL {
                if ui
                    .selectable_label(kind == current, kind.display_name())
                    .clicked()
                {
                    model.edit(id, |b| b.kind = kind);
                }
            }
        });
}

fn channel_picker(ui: &mut Ui, model: &mut EqModel, id: BandId, current: ChannelTarget) {
    ComboBox::from_id_salt("band_channel")
        .selected_text(current.ear_name())
        .width(90.0)
        .show_ui(ui, |ui| {
            for ear in ChannelTarget::ALL {
                if ui.selectable_label(ear == current, ear.ear_name()).clicked() {
                    model.edit(id, |b| b.channel = ear);
                }
            }
        });
}

fn slope_picker(ui: &mut Ui, model: &mut EqModel, id: BandId, current: i32) {
    ComboBox::from_id_salt("band_slope")
        .selected_text(format!("{} dB/oct", current))
        .width(90.0)
        .show_ui(ui, |ui| {
            for slope in SLOPES {
                if ui
                    .selectable_label(slope == current, format!("{} dB/oct", slope))
                    .clicked()
                {
                    model.edit(id, |b| b.slope_db_per_oct = slope);
                }
            }
        });
}

fn number(ui: &mut Ui, suffix: &str, value: f64, speed: f64, mut set: impl FnMut(f64)) {
    let mut v = value;
    let response = ui.add(
        DragValue::new(&mut v)
            .speed(speed)
            .max_decimals(2)
            .suffix(format!(" {}", suffix)),
    );
    if response.changed() {
        set(v);
    }
}
