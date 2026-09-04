use std::cell::Cell;

use super::band::{BandId, ChannelTarget, EqBand, FilterKind};

const FREQ_MIN: f64 = 20.0;
const FREQ_SPAN: f64 = 1000.0; // 20 Hz to 20 kHz

pub struct EqModel {
    bands: Vec<EqBand>,
    next_id: u64,

    preamp_db: f64,
    auto_gain: bool,
    sample_rate: f64,
    loudness_trim_db: f64,

    revision: u64,
    edits: u64,
    hold: u32,
    pending: bool,

    peak: Cell<f64>,
    peak_edits: Cell<Option<u64>>,
}

impl Default for EqModel {
    fn default() -> Self {
        EqModel {
            bands: Vec::new(),
            next_id: 1,
            preamp_db: 0.0,
            auto_gain: true,
            sample_rate: 48000.0,
            loudness_trim_db: 0.0,
            revision: 0,
            edits: 0,
            hold: 0,
            pending: false,
            peak: Cell::new(0.0),
            peak_edits: Cell::new(None),
        }
    }
}

impl EqModel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn bands(&self) -> &[EqBand] {
        &self.bands
    }

    pub fn band(&self, id: BandId) -> Option<&EqBand> {
        self.bands.iter().find(|b| b.id == id)
    }

    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
    }

    pub fn set_sample_rate(&mut self, v: f64) {
        if self.sample_rate != v {
            self.sample_rate = v;
            self.touch();
        }
    }

    pub fn preamp_db(&self) -> f64 {
        self.preamp_db
    }

    pub fn set_preamp_db(&mut self, v: f64) {
        let v = nan_to_zero(v).clamp(-30.0, 30.0);
        if self.preamp_db != v {
            self.preamp_db = v;
            self.touch();
        }
    }

    pub fn auto_gain(&self) -> bool {
        self.auto_gain
    }

    pub fn set_auto_gain(&mut self, v: bool) {
        if self.auto_gain != v {
            self.auto_gain = v;
            self.touch();
        }
    }

    pub fn loudness_trim_db(&self) -> f64 {
        self.loudness_trim_db
    }

    pub fn set_loudness_trim_db(&mut self, v: f64) {
        let v = nan_to_zero(v).clamp(-30.0, 0.0);
        if self.loudness_trim_db != v {
            self.loudness_trim_db = v;
            self.touch();
        }
    }

    pub fn base_preamp_db(&self) -> f64 {
        if self.auto_gain {
            self.auto_preamp_db()
        } else {
            self.preamp_db
        }
    }

    pub fn effective_preamp_db(&self) -> f64 {
        (self.base_preamp_db() + self.loudness_trim_db).clamp(-60.0, 30.0)
    }

    // bands

    pub fn add_band(&mut self, kind: FilterKind, freq: f64, gain_db: f64, q: f64) -> BandId {
        let id = BandId(self.next_id);
        self.next_id += 1;

        let mut b = EqBand::new(id);
        b.kind = kind;
        b.freq = freq;
        b.gain_db = gain_db;
        b.q = q;
        b.clamp();

        self.bands.push(b);
        self.touch();
        id
    }

    pub fn push_band(&mut self, band: EqBand) -> BandId {
        let id = BandId(self.next_id);
        self.next_id += 1;

        let mut b = band;
        b.id = id;
        b.clamp();

        self.bands.push(b);
        self.touch();
        id
    }

    pub fn remove_band(&mut self, id: BandId) {
        let before = self.bands.len();
        self.bands.retain(|b| b.id != id);
        if self.bands.len() != before {
            self.touch();
        }
    }

    pub fn clear(&mut self) {
        if !self.bands.is_empty() {
            self.bands.clear();
            self.touch();
        }
    }

    pub fn edit(&mut self, id: BandId, f: impl FnOnce(&mut EqBand)) {
        let Some(band) = self.bands.iter_mut().find(|b| b.id == id) else {
            return;
        };

        let before = *band;
        f(band);
        band.id = before.id;
        band.clamp();

        if *band != before {
            self.touch();
        }
    }

    // response

    pub fn response_db(&self, freq_hz: f64, channel: ChannelTarget) -> f64 {
        self.bands
            .iter()
            .filter(|b| b.enabled && b.channel.applies_to(channel))
            .map(|b| b.response_db(freq_hz, self.sample_rate))
            .sum()
    }

    pub fn has_per_channel_bands(&self) -> bool {
        self.bands
            .iter()
            .any(|b| b.enabled && b.channel != ChannelTarget::Both)
    }

    fn peak_db(&self) -> f64 {
        if self.peak_edits.get() != Some(self.edits) {
            self.peak.set(self.sweep_peak_db());
            self.peak_edits.set(Some(self.edits));
        }
        self.peak.get()
    }

    fn sweep_peak_db(&self) -> f64 {
        const STEPS: i32 = 512;
        let split = self.has_per_channel_bands();
        let mut peak = 0.0f64;

        for i in 0..=STEPS {
            let f = FREQ_MIN * FREQ_SPAN.powf(i as f64 / STEPS as f64);
            let mut v = self.response_db(f, ChannelTarget::Both);
            if split {
                v = v.max(self.response_db(f, ChannelTarget::Left));
                v = v.max(self.response_db(f, ChannelTarget::Right));
            }
            peak = peak.max(v);
        }
        peak
    }

    fn auto_preamp_db(&self) -> f64 {
        let peak = self.peak_db();
        if peak <= 0.0 {
            0.0
        } else {
            -round_to(peak + 0.2, 1) // headroom for inter-sample peaks
        }
    }

    // `revision` is what the UI and the backend worker watch, and a batch moves it once;
    // `edits` counts every mutation so a mid-batch read cannot see a stale peak.

    pub fn batch<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        self.hold += 1;
        let out = f(self);
        self.end_batch();
        out
    }

    fn end_batch(&mut self) {
        self.hold -= 1;
        if self.hold > 0 || !self.pending {
            return;
        }
        self.pending = false;
        self.revision += 1;
    }

    pub fn touch(&mut self) {
        self.edits += 1;
        if self.hold > 0 {
            self.pending = true;
        } else {
            self.revision += 1;
        }
    }
}

fn nan_to_zero(v: f64) -> f64 {
    if v.is_nan() {
        0.0
    } else {
        v
    }
}

fn round_to(v: f64, decimals: u32) -> f64 {
    let scale = 10f64.powi(decimals as i32);
    (v * scale).round() / scale
}
