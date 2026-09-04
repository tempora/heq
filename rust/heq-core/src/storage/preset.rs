use serde::{Deserialize, Serialize};

use super::BandDto;
use crate::model::EqModel;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Preset {
    #[serde(default)]
    pub name: Option<String>,
    pub preamp: f64,
    #[serde(default = "default_true")]
    pub auto_gain: bool,
    #[serde(default)]
    pub exclude_from_loudness: bool,
    #[serde(default)]
    pub bands: Vec<BandDto>,
}

fn default_true() -> bool {
    true
}

impl Default for Preset {
    fn default() -> Self {
        Preset {
            name: None,
            preamp: 0.0,
            auto_gain: true,
            exclude_from_loudness: false,
            bands: Vec::new(),
        }
    }
}

impl Preset {
    pub fn from_model(m: &EqModel, name: impl Into<String>) -> Self {
        Preset {
            name: Some(name.into()),
            preamp: m.preamp_db(),
            auto_gain: m.auto_gain(),
            exclude_from_loudness: false,
            bands: m.bands().iter().map(BandDto::from_band).collect(),
        }
    }

    pub fn apply_to(&self, m: &mut EqModel) {
        m.batch(|m| {
            m.clear();
            for d in &self.bands {
                m.push_band(d.to_band());
            }
            m.set_auto_gain(self.auto_gain);
            m.set_preamp_db(self.preamp);
            m.touch();
        });
    }

    pub fn to_model(&self, sample_rate: f64) -> EqModel {
        let mut m = EqModel::new();
        m.set_sample_rate(if sample_rate <= 0.0 { 48000.0 } else { sample_rate });
        self.apply_to(&mut m);
        m
    }

    pub fn matches(&self, m: &EqModel) -> bool {
        if m.auto_gain() != self.auto_gain || m.bands().len() != self.bands.len() {
            return false;
        }
        if (m.preamp_db() - self.preamp).abs() > 1e-9 {
            return false;
        }
        self.bands
            .iter()
            .zip(m.bands())
            .all(|(d, b)| d.matches(b))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresetRef {
    pub folder: String,
    pub name: String,
}

impl PresetRef {
    pub fn new(folder: impl Into<String>, name: impl Into<String>) -> Self {
        PresetRef {
            folder: folder.into(),
            name: name.into(),
        }
    }

    pub fn display(&self) -> String {
        format!("{} / {}", self.folder, self.name)
    }

    pub fn matches(&self, folder: &str, name: &str) -> bool {
        self.folder.eq_ignore_ascii_case(folder) && self.name.eq_ignore_ascii_case(name)
    }
}
