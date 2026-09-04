#![cfg_attr(windows, windows_subsystem = "windows")]

mod band_card;
mod curve;
mod theme;

use eframe::egui;

use heq_core::backend::{platform, BackendKind, BackendWorker, Snapshot, Status};
use heq_core::model::{AbTester, EqModel};
use heq_core::storage::{store, Preset, Settings};

use curve::CurveState;

fn main() -> eframe::Result<()> {
    let settings = store::load_settings();
    store::migrate_loose_presets();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([settings.window_width as f32, settings.window_height as f32])
            .with_min_inner_size([720.0, 480.0])
            .with_title("heq"),
        ..Default::default()
    };

    eframe::run_native(
        "heq",
        options,
        Box::new(|cc| Ok(Box::new(HeqApp::new(cc, settings)))),
    )
}

struct HeqApp {
    model: EqModel,
    #[allow(dead_code)] // wired up with the library panel in the next phase
    ab: AbTester,
    curve: CurveState,
    settings: Settings,
    backend: BackendKind,
    worker: BackendWorker,
    status: Status,
    sent_revision: Option<u64>,
}

impl HeqApp {
    fn new(cc: &eframe::CreationContext<'_>, settings: Settings) -> Self {
        theme::apply(&cc.egui_ctx);

        let mut model = EqModel::new();
        model.set_sample_rate(settings.sample_rate);
        if let Some(preset) = &settings.current {
            preset.apply_to(&mut model);
        }

        let backend = platform::default_kind();
        let worker = BackendWorker::spawn(backend.create());

        HeqApp {
            curve: CurveState::new(settings.db_range),
            status: worker.status(),
            model,
            ab: AbTester::new(),
            settings,
            backend,
            worker,
            sent_revision: None,
        }
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("top").show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let mut auto = self.model.auto_gain();
                if ui.checkbox(&mut auto, "auto gain").changed() {
                    self.model.set_auto_gain(auto);
                }

                let mut preamp = self.model.base_preamp_db();
                let field = ui.add_enabled(
                    !self.model.auto_gain(),
                    egui::DragValue::new(&mut preamp)
                        .speed(0.1)
                        .range(-30.0..=30.0)
                        .max_decimals(1)
                        .suffix(" dB"),
                );
                if field.changed() {
                    self.model.set_preamp_db(preamp);
                }

                ui.separator();

                if ui
                    .selectable_label(self.settings.bypassed, "bypass")
                    .clicked()
                {
                    self.settings.bypassed = !self.settings.bypassed;
                    self.model.touch();
                }

                ui.separator();

                let mut range = self.curve.db_range;
                egui::ComboBox::from_id_salt("db_range")
                    .selected_text(format!("±{} dB", range))
                    .width(84.0)
                    .show_ui(ui, |ui| {
                        for r in [6.0, 12.0, 18.0, 24.0, 30.0] {
                            ui.selectable_value(&mut range, r, format!("±{} dB", r));
                        }
                    });
                self.curve.db_range = range;
                self.settings.db_range = range;

                if platform::SUPPORTED.len() > 1 {
                    ui.separator();
                    self.backend_picker(ui);
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} · {}",
                            self.backend.label(),
                            self.status.message
                        ))
                        .color(if self.status.connected {
                            theme::TEXT_DIM
                        } else {
                            theme::col(heq_core::ui::palette::ACCENT)
                        }),
                    );

                    let trim = self.model.loudness_trim_db();
                    if trim < 0.0 {
                        ui.label(
                            egui::RichText::new(format!("match {:.1} dB", trim))
                                .color(theme::TEXT_DIM),
                        );
                    }
                });
            });
            ui.add_space(4.0);
        });
    }

    fn backend_picker(&mut self, ui: &mut egui::Ui) {
        let mut chosen = self.backend;
        egui::ComboBox::from_id_salt("backend")
            .selected_text(chosen.label())
            .width(120.0)
            .show_ui(ui, |ui| {
                for kind in platform::SUPPORTED {
                    ui.selectable_value(&mut chosen, *kind, kind.label());
                }
            });

        if chosen != self.backend {
            self.backend = chosen;
            self.worker = BackendWorker::spawn(chosen.create());
            self.sent_revision = None;
        }
    }

    fn push_if_changed(&mut self) {
        let revision = self.model.revision();
        if self.sent_revision == Some(revision) {
            return;
        }

        self.worker.schedule(Snapshot::capture(
            &self.model,
            None,
            self.settings.bypassed,
        ));
        self.sent_revision = Some(revision);
    }
}

impl eframe::App for HeqApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.status = self.worker.status();

        self.top_bar(ui);

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                curve::show(ui, &mut self.model, None, &mut self.curve);
            });

        let ctx = ui.ctx().clone();
        band_card::show(&ctx, &mut self.model, &mut self.curve);

        self.push_if_changed();
    }

    fn on_exit(&mut self) {
        self.settings.current = Some(Preset::from_model(&self.model, "current"));
        self.settings.sample_rate = self.model.sample_rate();
        store::save_settings(&self.settings);
    }
}
