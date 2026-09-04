use serde::{Deserialize, Serialize};

use crate::model::{BandId, ChannelTarget, EqBand, FilterKind};

// the one place the stored band shape is defined; field names are the C# app's
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct BandDto {
    #[serde(rename = "Type")]
    pub kind: FilterKind,
    pub freq: f64,
    pub gain: f64,
    pub q: f64,
    #[serde(default = "default_slope")]
    pub slope: i32,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub channel: ChannelTarget,
}

fn default_slope() -> i32 {
    12
}

fn default_enabled() -> bool {
    true
}

impl BandDto {
    pub fn from_band(b: &EqBand) -> Self {
        BandDto {
            kind: b.kind,
            freq: b.freq,
            gain: b.gain_db,
            q: b.q,
            slope: b.slope_db_per_oct,
            enabled: b.enabled,
            channel: b.channel,
        }
    }

    pub fn to_band(self) -> EqBand {
        let mut b = EqBand::new(BandId(0));
        b.kind = self.kind;
        b.freq = self.freq;
        b.gain_db = self.gain;
        b.q = self.q;
        b.slope_db_per_oct = if self.slope <= 0 { 12 } else { self.slope };
        b.enabled = self.enabled;
        b.channel = self.channel;
        b.clamp();
        b
    }

    pub fn matches(&self, b: &EqBand) -> bool {
        b.kind == self.kind
            && b.enabled == self.enabled
            && b.channel == self.channel
            && b.slope_db_per_oct == if self.slope <= 0 { 12 } else { self.slope }
            && same(b.freq, self.freq)
            && same(b.gain_db, self.gain)
            && same(b.q, self.q)
    }
}

fn same(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9
}
