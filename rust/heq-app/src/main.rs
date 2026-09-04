#![cfg_attr(windows, windows_subsystem = "windows")]

mod ab;
mod band_card;
mod correction;
mod curve;
mod library;
mod theme;

use eframe::egui;

use heq_core::backend::{platform, BackendKind, BackendWorker, Snapshot, Status};
use heq_core::model::{AbTester, EqModel};
use heq_core::storage::{store, Preset, Settings};

use correction::CorrectionEditor;
use curve::CurveState;
use library::Library;

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
    ab: AbTester,
    curve: CurveState,
    library: Library,
    correction: CorrectionEditor,
    settings: Settings,
    backend: BackendKind,
    worker: BackendWorker,
    status: Status,
    sent_revision: Option<u64>,
    sent_correction: Option<u64>,
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

        let mut app = HeqApp {
            curve: CurveState::new(settings.db_range),
            status: worker.status(),
            library: Library::new(&settings),
            correction: CorrectionEditor::new(),
            model,
            ab: AbTester::new(),
            settings,
            backend,
            worker,
            sent_revision: None,
            sent_correction: None,
        };

        if let (Some(folder), Some(name)) = (
            app.settings.last_folder.clone(),
            app.settings.last_preset.clone(),
        ) {
            if store::exists(&folder, &name) {
                app.library
                    .load_preset(&folder, &name, &mut app.model, &mut app.ab);
            }
        }

        app.correction.load_for(app.library.baseline_folder());
        app.library.recompute_folder_target(app.model.sample_rate());

        app.library
            .restore_b(&app.settings, &mut app.model, &mut app.ab);
        app.library
            .apply_folder_target(&app.settings, &mut app.ab, &mut app.model);

        app
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

                ui.separator();
                ab::pill(ui, &mut self.ab, &mut self.library, &mut self.model);

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
        let correction = self.correction.model.revision();

        if self.sent_revision == Some(revision) && self.sent_correction == Some(correction) {
            return;
        }

        self.ab.refresh(&mut self.model);

        self.worker.schedule(Snapshot::capture(
            &self.model,
            self.correction.current().as_ref(),
            self.settings.bypassed,
        ));

        self.sent_revision = Some(self.model.revision());
        self.sent_correction = Some(correction);
    }
}

impl eframe::App for HeqApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.status = self.worker.status();
        self.library.refresh_edited(&self.model);

        self.top_bar(ui);

        self.library.show(
            ui,
            &mut self.model,
            &mut self.ab,
            &mut self.curve,
            &mut self.settings,
        );

        if let Some(folder) = self.library.open_correction.take() {
            self.correction.open(&folder, self.curve.db_range);
        }

        // a preset loaded from another group brings that group's correction with it
        if !self.correction.is_open()
            && self.correction.folder() != self.library.baseline_folder()
        {
            self.correction.load_for(self.library.baseline_folder());
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                curve::show(
                    ui,
                    &mut self.model,
                    self.correction.overlay(),
                    &mut self.curve,
                );
            });

        let ctx = ui.ctx().clone();
        band_card::show(&ctx, &mut self.model, &mut self.curve);
        self.correction.show(ui, self.library.baseline_folder());

        self.push_if_changed();
    }

    fn on_exit(&mut self) {
        self.correction.close();

        self.settings.current = Some(Preset::from_model(&self.model, "current"));
        self.settings.sample_rate = self.model.sample_rate();
        self.settings.last_folder = self.library.folder.clone();
        self.settings.last_preset = self.library.current.clone();

        let ab = self.library.ab_source.as_ref();
        self.settings.ab_folder = ab.map(|r| r.folder.clone());
        self.settings.ab_preset = ab.map(|r| r.name.clone());

        store::save_settings(&self.settings);
    }
}
