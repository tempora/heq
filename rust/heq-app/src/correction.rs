use eframe::egui::{self, Ui};

use heq_core::model::{ChannelTarget, EqModel};
use heq_core::storage::{store, Correction};

use crate::curve::{self, CurveState};

// A group's left/right fix. It is edited on its own curve, and folded into the drawn totals of
// the main one, so what is on screen stays what the backend is given.
pub struct CorrectionEditor {
    pub model: EqModel,
    pub enabled: bool,
    folder: Option<String>,
    open_for: Option<String>,
    curve: CurveState,
}

impl CorrectionEditor {
    pub fn new() -> Self {
        CorrectionEditor {
            model: EqModel::new(),
            enabled: true,
            folder: None,
            open_for: None,
            curve: CurveState::new(18.0),
        }
    }

    pub fn is_open(&self) -> bool {
        self.open_for.is_some()
    }

    pub fn folder(&self) -> Option<&str> {
        self.folder.as_deref()
    }

    // the correction that reaches the backend, or none when nothing applies
    pub fn current(&self) -> Option<Correction> {
        let mut c = Correction::from_model(&self.model);
        c.enabled = self.enabled;
        c.applies().then_some(c)
    }

    pub fn overlay(&self) -> Option<&EqModel> {
        (!self.model.bands().is_empty() && self.enabled).then_some(&self.model)
    }

    // called whenever the loaded preset's group changes
    pub fn load_for(&mut self, folder: Option<&str>) {
        if self.is_open() {
            return;
        }

        let c = folder.map(store::load_correction).unwrap_or_default();
        self.enabled = c.enabled;
        c.apply_to(&mut self.model);
        self.folder = folder.map(str::to_string);
    }

    pub fn open(&mut self, folder: &str, db_range: f64) {
        let c = store::load_correction(folder);
        self.enabled = c.enabled;
        c.apply_to(&mut self.model);

        self.curve = CurveState::new(db_range);
        self.curve.place_on = ChannelTarget::Left;
        self.folder = Some(folder.to_string());
        self.open_for = Some(folder.to_string());
    }

    pub fn close(&mut self) {
        let Some(folder) = self.open_for.take() else {
            return;
        };

        let mut c = Correction::from_model(&self.model);
        c.enabled = self.enabled;
        store::save_correction(&folder, &c);
    }

    pub fn show(&mut self, ui: &mut Ui, baseline_folder: Option<&str>) {
        let Some(folder) = self.open_for.clone() else {
            return;
        };

        let mut close = false;

        egui::Window::new(format!("correction · {}", folder))
            .collapsible(false)
            .resizable(true)
            .default_size([560.0, 360.0])
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    let mut on_right = self.curve.place_on == ChannelTarget::Right;
                    if ui.selectable_label(!on_right, "left").clicked() {
                        on_right = false;
                    }
                    if ui.selectable_label(on_right, "right").clicked() {
                        on_right = true;
                    }
                    self.curve.place_on = if on_right {
                        ChannelTarget::Right
                    } else {
                        ChannelTarget::Left
                    };

                    ui.separator();
                    ui.checkbox(&mut self.enabled, "apply");

                    if ui.button("clear").clicked() {
                        self.model.clear();
                        self.curve.selected = None;
                    }

                    ui.separator();
                    let summary = Correction::from_model(&self.model).summary();
                    ui.label(egui::RichText::new(summary).color(crate::theme::TEXT_DIM));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("done").clicked() {
                            close = true;
                        }
                    });
                });

                ui.separator();
                curve::show(ui, &mut self.model, None, &mut self.curve);
            });

        if close {
            self.close();
            self.load_for(baseline_folder);
        }
    }
}
