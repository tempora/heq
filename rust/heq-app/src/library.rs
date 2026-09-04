use std::collections::HashSet;

use eframe::egui::{self, Ui};

use heq_core::dsp::loudness;
use heq_core::model::{AbTester, EqModel};
use heq_core::storage::{store, Preset, PresetRef, Settings};

use crate::curve::CurveState;

// What the interface calls a group is a folder on disk; the presets in it are that group's.
pub struct Library {
    pub folder: Option<String>,
    pub current: Option<String>,
    pub edited: bool,
    pub status: Option<String>,

    folders: Vec<String>,
    presets: Vec<String>,
    excluded: HashSet<String>,
    folder_level: Option<f64>,

    baseline: Option<Preset>,
    baseline_folder: Option<String>,

    pub ab_source: Option<PresetRef>,
    pub open_correction: Option<String>,

    prompt: Option<Prompt>,
}

enum Action {
    SavePreset,
    RenamePreset(String),
    NewFolder,
    RenameFolder,
    DeletePreset(String),
    DeleteFolder,
}

struct Prompt {
    action: Action,
    title: String,
    label: String,
    text: String,
    confirm_only: bool,
}

impl Library {
    pub fn new(settings: &Settings) -> Self {
        let mut lib = Library {
            folder: settings.last_folder.clone(),
            current: settings.last_preset.clone(),
            edited: false,
            status: None,
            folders: Vec::new(),
            presets: Vec::new(),
            excluded: HashSet::new(),
            folder_level: None,
            baseline: None,
            baseline_folder: None,
            ab_source: None,
            open_correction: None,
            prompt: None,
        };
        lib.refresh_folders(None);
        lib
    }

    // ==== listing ====

    pub fn refresh_folders(&mut self, select: Option<&str>) {
        self.folders = store::list_folders();

        let want = select.map(str::to_string).or_else(|| self.folder.clone());
        self.folder = want
            .and_then(|w| self.folders.iter().find(|f| same(f, &w)).cloned())
            .or_else(|| self.folders.first().cloned());

        self.refresh_presets();
    }

    pub fn refresh_presets(&mut self) {
        self.presets = match &self.folder {
            Some(f) => store::list_presets(f),
            None => Vec::new(),
        };
    }

    pub fn recompute_folder_target(&mut self, sample_rate: f64) {
        self.excluded.clear();
        let mut target: Option<f64> = None;

        let Some(folder) = self.folder.clone() else {
            self.folder_level = None;
            return;
        };

        for name in store::list_presets(&folder) {
            let Some(p) = store::load(&folder, &name) else {
                continue;
            };

            if p.exclude_from_loudness {
                self.excluded.insert(name);
                continue;
            }

            let level = loudness::level_db(&p.to_model(sample_rate));
            target = Some(target.map_or(level, |t: f64| t.min(level)));
        }

        self.folder_level = target;
    }

    pub fn apply_folder_target(&self, settings: &Settings, ab: &mut AbTester, model: &mut EqModel) {
        let loaded_here = self.current.is_some()
            && self
                .baseline_folder
                .as_deref()
                .zip(self.folder.as_deref())
                .is_some_and(|(a, b)| same(a, b));

        let target = if settings.match_folder_loudness && loaded_here {
            self.folder_level
        } else {
            None
        };
        ab.set_folder_target_db(target, model);
    }

    // ==== presets ====

    pub fn load_preset(&mut self, folder: &str, name: &str, model: &mut EqModel, ab: &mut AbTester) {
        let Some(p) = store::load(folder, name) else {
            self.status = Some(format!("Could not read “{}”.", name));
            return;
        };

        p.apply_to(model);
        self.current = Some(name.to_string());
        self.set_baseline(folder, Some(p), model);
        ab.name_current(Some(name.to_string()));
        ab.refresh(model);
    }

    pub fn set_baseline(&mut self, folder: &str, saved: Option<Preset>, model: &EqModel) {
        self.edited = saved.as_ref().is_some_and(|s| !s.matches(model));
        self.baseline_folder = saved.as_ref().map(|_| folder.to_string());
        self.baseline = saved;
    }

    pub fn refresh_edited(&mut self, model: &EqModel) {
        self.edited = self.baseline.as_ref().is_some_and(|b| !b.matches(model));
    }

    pub fn baseline_folder(&self) -> Option<&str> {
        self.baseline_folder.as_deref()
    }

    pub fn clear_to_default(&mut self, model: &mut EqModel, ab: &mut AbTester) {
        model.batch(|m| {
            m.clear();
            m.set_auto_gain(true);
            m.set_preamp_db(0.0);
        });

        self.current = None;
        self.baseline = None;
        self.baseline_folder = None;
        self.edited = false;
        ab.name_current(None);
    }

    fn save_preset(&mut self, name: &str, model: &EqModel) {
        let Some(folder) = self.folder.clone() else { return };

        let mut preset = Preset::from_model(model, name);
        if let Some(existing) = store::load(&folder, name) {
            preset.exclude_from_loudness = existing.exclude_from_loudness;
        }

        match store::save(&preset, &folder) {
            Ok(()) => {
                self.current = Some(name.to_string());
                self.set_baseline(&folder, Some(preset), model);
                self.refresh_presets();
                self.status = None;
            }
            Err(e) => self.status = Some(format!("Could not save “{}”: {}", name, e)),
        }
    }

    // ==== the panel ====

    #[allow(clippy::too_many_arguments)]
    pub fn show(
        &mut self,
        ui: &mut Ui,
        model: &mut EqModel,
        ab: &mut AbTester,
        curve: &mut CurveState,
        settings: &mut Settings,
    ) {
        egui::Panel::left("library")
            .default_size(210.0)
            .show(ui, |ui| {
                ui.add_space(6.0);
                self.folder_row(ui, model, ab, settings);
                ui.separator();
                self.preset_list(ui, model, ab, curve, settings);
                ui.separator();
                self.footer(ui, model, ab, settings);
            });

        self.prompt_modal(ui, model, ab, settings);
    }

    fn folder_row(
        &mut self,
        ui: &mut Ui,
        model: &mut EqModel,
        ab: &mut AbTester,
        settings: &mut Settings,
    ) {
        let mut chosen = self.folder.clone();

        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("folder")
                .selected_text(chosen.clone().unwrap_or_else(|| "no groups".into()))
                .width(150.0)
                .show_ui(ui, |ui| {
                    for f in &self.folders {
                        ui.selectable_value(&mut chosen, Some(f.clone()), f);
                    }
                });

            if ui.button("+").on_hover_text("New group").clicked() {
                self.ask(Action::NewFolder, "New group", "Name", "");
            }
        });

        if chosen != self.folder {
            self.folder = chosen;
            settings.last_folder = self.folder.clone();
            self.refresh_presets();
            self.recompute_folder_target(model.sample_rate());
            self.apply_folder_target(settings, ab, model);
        }
    }

    fn preset_list(
        &mut self,
        ui: &mut Ui,
        model: &mut EqModel,
        ab: &mut AbTester,
        curve: &mut CurveState,
        settings: &mut Settings,
    ) {
        if self.presets.is_empty() {
            ui.add_space(6.0);
            ui.label(if self.folder.is_none() {
                "No groups yet."
            } else {
                "Nothing saved here yet."
            });
            ui.add_space(6.0);
            return;
        }

        let folder = self.folder.clone().unwrap_or_default();
        let mut pending: Option<Box<dyn FnOnce(&mut Self, &mut EqModel, &mut AbTester)>> = None;

        egui::ScrollArea::vertical()
            .max_height(ui.available_height() - 90.0)
            .show(ui, |ui| {
                for name in self.presets.clone() {
                    let selected = self
                        .current
                        .as_deref()
                        .is_some_and(|c| same(c, &name) && self.owns_folder());
                    let is_b = self
                        .ab_source
                        .as_ref()
                        .is_some_and(|r| r.matches(&folder, &name));

                    let label = format!(
                        "{}{}{}",
                        name,
                        if is_b { "  ·B" } else { "" },
                        if selected && self.edited { "  ·" } else { "" },
                    );

                    let row = ui.selectable_label(selected, label);

                    if row.clicked() {
                        let (f, n) = (folder.clone(), name.clone());
                        pending = Some(Box::new(move |lib, model, ab| {
                            lib.load_preset(&f, &n, model, ab)
                        }));
                        curve.selected = None;
                    }

                    let (f, n) = (folder.clone(), name.clone());
                    row.context_menu(|ui| {
                        if ui.button("Set as B").clicked() {
                            let (f, n) = (f.clone(), n.clone());
                            pending = Some(Box::new(move |lib, model, ab| lib.set_b(&f, &n, model, ab)));
                            ui.close();
                        }
                        if ui.button("Rename…").clicked() {
                            self.ask(
                                Action::RenamePreset(n.clone()),
                                "Rename preset",
                                "New name",
                                &n,
                            );
                            ui.close();
                        }
                        if ui.button("Delete…").clicked() {
                            self.confirm(
                                Action::DeletePreset(n.clone()),
                                "Delete preset",
                                &format!("Delete “{}”?", n),
                            );
                            ui.close();
                        }

                        ui.separator();

                        let excluded = self.excluded.contains(&n);
                        if ui
                            .selectable_label(excluded, "Exclude from group loudness")
                            .clicked()
                        {
                            self.toggle_exclusion(&f, &n, model.sample_rate());
                            ui.close();
                        }

                        ui.separator();
                        ui.menu_button("Move to", |ui| {
                            for other in self.folders.clone() {
                                if same(&other, &f) {
                                    continue;
                                }
                                if ui.button(&other).clicked() {
                                    if store::move_preset(&f, &n, &other) {
                                        self.refresh_presets();
                                    } else {
                                        self.status = Some(format!("Could not move “{}”.", n));
                                    }
                                    ui.close();
                                }
                            }
                        });
                    });
                }
            });

        if let Some(action) = pending {
            action(self, model, ab);
            self.recompute_folder_target(model.sample_rate());
            self.apply_folder_target(settings, ab, model);
        }
    }

    fn footer(
        &mut self,
        ui: &mut Ui,
        model: &mut EqModel,
        ab: &mut AbTester,
        settings: &mut Settings,
    ) {
        ui.horizontal(|ui| {
            if ui.button("save").clicked() {
                if self.folder.is_none() {
                    self.ask(Action::NewFolder, "New group", "Name", "");
                } else {
                    let name = self.current.clone().unwrap_or_else(|| "New preset".into());
                    self.ask(Action::SavePreset, "Save preset", "Name", &name);
                }
            }
            if ui.button("new").clicked() {
                self.clear_to_default(model, ab);
            }
            if ui
                .add_enabled(self.folder.is_some(), egui::Button::new("correction…"))
                .clicked()
            {
                self.open_correction = self.folder.clone();
            }
        });

        ui.horizontal(|ui| {
            if ui
                .add_enabled(self.folder.is_some(), egui::Button::new("rename group"))
                .clicked()
            {
                let current = self.folder.clone().unwrap_or_default();
                self.ask(Action::RenameFolder, "Rename group", "New name", &current);
            }
            if ui
                .add_enabled(self.folder.is_some(), egui::Button::new("delete group"))
                .clicked()
            {
                let current = self.folder.clone().unwrap_or_default();
                self.confirm(
                    Action::DeleteFolder,
                    "Delete group",
                    &format!("Delete “{}” and everything in it?", current),
                );
            }
        });

        let mut match_loudness = settings.match_folder_loudness;
        if ui
            .checkbox(&mut match_loudness, "match group loudness")
            .changed()
        {
            settings.match_folder_loudness = match_loudness;
            self.apply_folder_target(settings, ab, model);
        }

        if let Some(status) = &self.status {
            ui.label(egui::RichText::new(status).color(crate::theme::col(
                heq_core::ui::palette::ACCENT,
            )));
        } else if self.edited {
            ui.label(
                egui::RichText::new("unsaved changes").color(crate::theme::TEXT_DIM),
            );
        }
    }

    // ==== A/B ====

    pub fn set_b(&mut self, folder: &str, name: &str, model: &mut EqModel, ab: &mut AbTester) {
        let Some(p) = store::load(folder, name) else {
            self.status = Some(format!("Could not read “{}”.", name));
            return;
        };

        self.ab_source = Some(PresetRef::new(folder, name));
        ab.set_b(Some(p), Some(name.to_string()), model);
    }

    pub fn clear_b(&mut self, model: &mut EqModel, ab: &mut AbTester) {
        self.ab_source = None;
        ab.clear(model);
    }

    pub fn restore_b(&mut self, settings: &Settings, model: &mut EqModel, ab: &mut AbTester) {
        let (Some(folder), Some(name)) = (&settings.ab_folder, &settings.ab_preset) else {
            return;
        };
        let (folder, name) = (folder.clone(), name.clone());
        self.set_b(&folder, &name, model, ab);
    }

    pub fn switch_ab(&mut self, to_b: bool, model: &mut EqModel, ab: &mut AbTester) {
        if !ab.active() {
            return;
        }

        ab.switch_to(to_b, model);
        self.current = ab.current_name().map(str::to_string);

        let folder = self.baseline_folder.clone().or_else(|| self.folder.clone());
        match (folder, self.current.clone()) {
            (Some(f), Some(n)) => {
                let saved = store::load(&f, &n);
                self.set_baseline(&f, saved, model);
            }
            _ => {
                self.baseline = None;
                self.baseline_folder = None;
                self.edited = false;
            }
        }
    }

    fn toggle_exclusion(&mut self, folder: &str, name: &str, sample_rate: f64) {
        let Some(mut p) = store::load(folder, name) else {
            self.status = Some(format!("Could not read “{}”.", name));
            return;
        };

        p.exclude_from_loudness = !p.exclude_from_loudness;
        if store::save(&p, folder).is_ok() {
            self.recompute_folder_target(sample_rate);
        }
    }

    fn owns_folder(&self) -> bool {
        match (&self.baseline_folder, &self.folder) {
            (Some(a), Some(b)) => same(a, b),
            _ => false,
        }
    }

    // ==== prompts ====

    fn ask(&mut self, action: Action, title: &str, label: &str, text: &str) {
        self.prompt = Some(Prompt {
            action,
            title: title.to_string(),
            label: label.to_string(),
            text: text.to_string(),
            confirm_only: false,
        });
    }

    fn confirm(&mut self, action: Action, title: &str, question: &str) {
        self.prompt = Some(Prompt {
            action,
            title: title.to_string(),
            label: question.to_string(),
            text: String::new(),
            confirm_only: true,
        });
    }

    fn prompt_modal(
        &mut self,
        ui: &mut Ui,
        model: &mut EqModel,
        ab: &mut AbTester,
        settings: &mut Settings,
    ) {
        let Some(prompt) = &mut self.prompt else { return };

        let mut text = prompt.text.clone();
        let confirm_only = prompt.confirm_only;
        let (title, label) = (prompt.title.clone(), prompt.label.clone());

        let mut outcome = None;

        egui::Modal::new(egui::Id::new("library_prompt")).show(ui.ctx(), |ui| {
            ui.set_width(280.0);
            ui.heading(&title);
            ui.add_space(6.0);
            ui.label(&label);

            if !confirm_only {
                let field = ui.add(egui::TextEdit::singleline(&mut text).desired_width(f32::INFINITY));
                field.request_focus();
                if field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    outcome = Some(true);
                }
            }

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("cancel").clicked() {
                    outcome = Some(false);
                }
                let ok = if confirm_only { "delete" } else { "ok" };
                if ui.button(ok).clicked() {
                    outcome = Some(true);
                }
            });
        });

        if let Some(prompt) = &mut self.prompt {
            prompt.text = text.clone();
        }

        match outcome {
            None => return,
            Some(false) => {
                self.prompt = None;
                return;
            }
            Some(true) => {}
        }

        let Some(prompt) = self.prompt.take() else { return };
        let text = text.trim().to_string();

        if !prompt.confirm_only && text.is_empty() {
            return;
        }

        self.run(prompt.action, &text, model, ab, settings);
    }

    fn run(
        &mut self,
        action: Action,
        text: &str,
        model: &mut EqModel,
        ab: &mut AbTester,
        settings: &mut Settings,
    ) {
        match action {
            Action::SavePreset => self.save_preset(text, model),

            Action::RenamePreset(old) => {
                let Some(folder) = self.folder.clone() else { return };
                match store::rename(&folder, &old, text) {
                    Some(new_name) => {
                        if self.current.as_deref().is_some_and(|c| same(c, &old)) {
                            self.current = Some(new_name);
                        }
                        self.refresh_presets();
                    }
                    None => self.status = Some(format!("Could not rename to “{}”.", text)),
                }
            }

            Action::DeletePreset(name) => {
                let Some(folder) = self.folder.clone() else { return };
                store::delete(&folder, &name);

                if self.current.as_deref().is_some_and(|c| same(c, &name)) {
                    self.current = None;
                    self.baseline = None;
                    self.baseline_folder = None;
                    self.edited = false;
                }
                if self
                    .ab_source
                    .as_ref()
                    .is_some_and(|r| r.matches(&folder, &name))
                {
                    self.clear_b(model, ab);
                }

                self.refresh_presets();
                self.recompute_folder_target(model.sample_rate());
                self.apply_folder_target(settings, ab, model);
            }

            Action::NewFolder => match store::create_folder(text) {
                Some(name) => {
                    settings.last_folder = Some(name.clone());
                    self.refresh_folders(Some(&name));
                }
                None => self.status = Some(format!("“{}” already exists.", text)),
            },

            Action::RenameFolder => {
                let Some(folder) = self.folder.clone() else { return };
                match store::rename_folder(&folder, text) {
                    Some(name) => {
                        if self.baseline_folder.as_deref().is_some_and(|f| same(f, &folder)) {
                            self.baseline_folder = Some(name.clone());
                        }
                        self.refresh_folders(Some(&name));
                    }
                    None => self.status = Some(format!("Could not rename to “{}”.", text)),
                }
            }

            Action::DeleteFolder => {
                let Some(folder) = self.folder.clone() else { return };
                store::delete_folder(&folder);

                if self.baseline_folder.as_deref().is_some_and(|f| same(f, &folder)) {
                    self.current = None;
                    self.baseline = None;
                    self.baseline_folder = None;
                    self.edited = false;
                }

                self.folder = None;
                self.refresh_folders(None);
                self.recompute_folder_target(model.sample_rate());
                self.apply_folder_target(settings, ab, model);
            }
        }
    }
}

fn same(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}
