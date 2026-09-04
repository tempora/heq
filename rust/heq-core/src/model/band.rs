use std::f64::consts::PI;

use crate::dsp::{butterworth_qs, Biquad};

pub const MIN_FREQ: f64 = 10.0;
pub const MAX_FREQ: f64 = 24000.0;
pub const MIN_Q: f64 = 0.025;
pub const MAX_Q: f64 = 40.0;
pub const MAX_GAIN: f64 = 30.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FilterKind {
    Bell,
    LowShelf,
    HighShelf,
    LowCut,
    HighCut,
    Notch,
    BandPass,
    AllPass,
}

impl FilterKind {
    pub const ALL: [FilterKind; 8] = [
        FilterKind::Bell,
        FilterKind::LowShelf,
        FilterKind::HighShelf,
        FilterKind::LowCut,
        FilterKind::HighCut,
        FilterKind::Notch,
        FilterKind::BandPass,
        FilterKind::AllPass,
    ];

    pub fn display_name(self) -> &'static str {
        match self {
            FilterKind::Bell => "Bell",
            FilterKind::LowShelf => "Low Shelf",
            FilterKind::HighShelf => "High Shelf",
            FilterKind::LowCut => "Low Cut",
            FilterKind::HighCut => "High Cut",
            FilterKind::Notch => "Notch",
            FilterKind::BandPass => "Band Pass",
            FilterKind::AllPass => "All Pass",
        }
    }

    pub fn uses_gain(self) -> bool {
        matches!(
            self,
            FilterKind::Bell | FilterKind::LowShelf | FilterKind::HighShelf
        )
    }

    pub fn uses_slope(self) -> bool {
        matches!(self, FilterKind::LowCut | FilterKind::HighCut)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ChannelTarget {
    #[default]
    Both,
    Left,
    Right,
}

impl ChannelTarget {
    pub const ALL: [ChannelTarget; 3] = [
        ChannelTarget::Both,
        ChannelTarget::Left,
        ChannelTarget::Right,
    ];

    pub fn ear_name(self) -> &'static str {
        match self {
            ChannelTarget::Left => "Left only",
            ChannelTarget::Right => "Right only",
            ChannelTarget::Both => "Both ears",
        }
    }

    pub fn applies_to(self, asked: ChannelTarget) -> bool {
        self == ChannelTarget::Both || self == asked
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BandId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EqBand {
    pub id: BandId,
    pub kind: FilterKind,
    pub freq: f64,
    pub gain_db: f64,
    pub q: f64,
    pub slope_db_per_oct: i32,
    pub enabled: bool,
    pub channel: ChannelTarget,
}

impl EqBand {
    pub fn new(id: BandId) -> Self {
        EqBand {
            id,
            kind: FilterKind::Bell,
            freq: 1000.0,
            gain_db: 0.0,
            q: 1.0,
            slope_db_per_oct: 12,
            enabled: true,
            channel: ChannelTarget::Both,
        }
    }

    // every edit funnels through EqModel, which calls this before bumping the revision
    pub fn clamp(&mut self) {
        self.freq = clamp(self.freq, MIN_FREQ, MAX_FREQ);
        self.gain_db = clamp(self.gain_db, -MAX_GAIN, MAX_GAIN);
        self.q = clamp(self.q, MIN_Q, MAX_Q);
        if self.slope_db_per_oct <= 0 {
            self.slope_db_per_oct = 12;
        }
    }

    pub fn sections(&self, sample_rate: f64) -> Vec<Biquad> {
        match self.kind {
            FilterKind::Bell => vec![Biquad::peaking(self.freq, sample_rate, self.gain_db, self.q)],
            FilterKind::LowShelf => {
                vec![Biquad::low_shelf(self.freq, sample_rate, self.gain_db, self.q)]
            }
            FilterKind::HighShelf => vec![Biquad::high_shelf(
                self.freq,
                sample_rate,
                self.gain_db,
                self.q,
            )],
            FilterKind::Notch => vec![Biquad::notch(self.freq, sample_rate, self.q)],
            FilterKind::BandPass => vec![Biquad::band_pass(self.freq, sample_rate, self.q)],
            FilterKind::AllPass => vec![Biquad::all_pass(self.freq, sample_rate, self.q)],
            FilterKind::LowCut => self
                .cut_qs()
                .into_iter()
                .map(|q| Biquad::high_pass(self.freq, sample_rate, q))
                .collect(),
            FilterKind::HighCut => self
                .cut_qs()
                .into_iter()
                .map(|q| Biquad::low_pass(self.freq, sample_rate, q))
                .collect(),
        }
    }

    pub fn cut_qs(&self) -> Vec<f64> {
        let order = (self.slope_db_per_oct / 6).max(2);
        if order <= 2 {
            vec![self.q]
        } else {
            butterworth_qs(order as usize)
        }
    }

    pub fn response_db(&self, freq_hz: f64, sample_rate: f64) -> f64 {
        if !self.enabled {
            return 0.0;
        }

        let w = 2.0 * PI * freq_hz / sample_rate;
        self.sections(sample_rate)
            .iter()
            .map(|s| s.gain_db(w))
            .sum()
    }
}

fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    if v.is_nan() {
        lo
    } else {
        v.clamp(lo, hi)
    }
}
