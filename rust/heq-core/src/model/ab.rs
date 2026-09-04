use crate::dsp::loudness;
use crate::model::EqModel;
use crate::storage::Preset;

// The side you hear is the live EqModel, so it stays editable; the other side is parked here
// as a snapshot with its measured level cached. Loudness matching is unconditional by design.
#[derive(Default)]
pub struct AbTester {
    other: Option<Preset>,
    other_name: Option<String>,
    other_level: f64,
    current_name: Option<String>,
    folder_target: Option<f64>,
    on_b: bool,
}

impl AbTester {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn active(&self) -> bool {
        self.other.is_some()
    }

    pub fn on_b(&self) -> bool {
        self.on_b
    }

    pub fn current_name(&self) -> Option<&str> {
        self.current_name.as_deref()
    }

    pub fn a_name(&self) -> Option<&str> {
        if self.on_b {
            self.other_name.as_deref()
        } else {
            self.current_name.as_deref()
        }
    }

    pub fn b_name(&self) -> Option<&str> {
        if self.on_b {
            self.current_name.as_deref()
        } else {
            self.other_name.as_deref()
        }
    }

    pub fn folder_target_db(&self) -> Option<f64> {
        self.folder_target
    }

    pub fn set_folder_target_db(&mut self, target: Option<f64>, model: &mut EqModel) {
        if self.folder_target == target {
            return;
        }
        self.folder_target = target;
        self.refresh(model);
    }

    pub fn set_b(&mut self, preset: Option<Preset>, name: Option<String>, model: &mut EqModel) {
        let Some(preset) = preset else {
            self.clear(model);
            return;
        };

        if self.on_b {
            self.switch_to(false, model);
        }

        self.other_level = loudness::level_db(&preset.to_model(model.sample_rate()));
        self.other = Some(preset);
        self.other_name = name;
        self.on_b = false;

        self.refresh(model);
    }

    pub fn clear(&mut self, model: &mut EqModel) {
        if !self.active() && model.loudness_trim_db() == 0.0 {
            return;
        }

        self.other = None;
        self.other_name = None;
        self.on_b = false;
        model.set_loudness_trim_db(0.0);
    }

    pub fn switch_to(&mut self, to_b: bool, model: &mut EqModel) {
        if !self.active() || to_b == self.on_b {
            return;
        }

        let parked = self.other.take().expect("active");
        let parked_name = self.other_name.take();

        self.other = Some(Preset::from_model(model, if self.on_b { "B" } else { "A" }));
        self.other_name = self.current_name.take();
        self.other_level = loudness::level_db(model);

        self.on_b = to_b;
        self.current_name = parked_name;
        parked.apply_to(model);

        self.refresh(model);
    }

    pub fn name_current(&mut self, name: Option<String>) {
        self.current_name = name;
    }

    pub fn refresh(&mut self, model: &mut EqModel) {
        if !self.active() && self.folder_target.is_none() {
            model.set_loudness_trim_db(0.0);
            return;
        }

        let mine = loudness::level_db(model);
        let target = [
            self.active().then_some(self.other_level),
            self.folder_target,
        ]
        .into_iter()
        .flatten()
        .fold(f64::MAX, f64::min);

        model.set_loudness_trim_db((target - mine).min(0.0));
    }
}
