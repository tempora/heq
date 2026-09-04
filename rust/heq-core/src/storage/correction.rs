use serde::{Deserialize, Serialize};

use super::BandDto;
use crate::model::{ChannelTarget, EqBand, EqModel};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Correction {
    #[serde(default)]
    pub bands: Vec<BandDto>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

impl Default for Correction {
    fn default() -> Self {
        Correction {
            bands: Vec::new(),
            enabled: true,
        }
    }
}

impl Correction {
    pub fn from_model(m: &EqModel) -> Self {
        Correction {
            bands: m.bands().iter().map(BandDto::from_band).collect(),
            enabled: true,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.bands.is_empty()
    }

    pub fn applies(&self) -> bool {
        self.enabled && self.bands.iter().any(|b| b.enabled)
    }

    pub fn apply_to(&self, m: &mut EqModel) {
        m.batch(|m| {
            m.clear();
            for b in self.to_bands() {
                m.push_band(b);
            }
            m.set_auto_gain(false);
            m.set_preamp_db(0.0);
            m.touch();
        });
    }

    pub fn to_bands(&self) -> Vec<EqBand> {
        self.bands
            .iter()
            .map(|d| {
                let mut b = d.to_band();
                if b.channel == ChannelTarget::Both {
                    b.channel = ChannelTarget::Left;
                }
                b
            })
            .collect()
    }

    pub fn summary(&self) -> String {
        if self.bands.is_empty() {
            return "no correction".to_string();
        }
        let left = self
            .bands
            .iter()
            .filter(|b| b.channel != ChannelTarget::Right)
            .count();
        format!("{} left · {} right", left, self.bands.len() - left)
    }
}
