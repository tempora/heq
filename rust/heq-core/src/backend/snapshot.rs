use crate::model::{ChannelTarget, EqBand, EqModel};
use crate::storage::Correction;

// Everything a backend needs, detached from the model so it can cross a thread.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Snapshot {
    pub bands: Vec<EqBand>,
    pub correction: Vec<EqBand>,
    pub preamp_db: f64,
    pub bypassed: bool,
    pub sample_rate: f64,
}

impl Snapshot {
    pub fn capture(model: &EqModel, correction: Option<&Correction>, bypassed: bool) -> Self {
        Snapshot {
            bands: model.bands().to_vec(),
            correction: correction
                .filter(|c| c.applies())
                .map(Correction::to_bands)
                .unwrap_or_default(),
            preamp_db: model.effective_preamp_db(),
            bypassed,
            sample_rate: model.sample_rate(),
        }
    }

    pub fn all_bands(&self) -> impl Iterator<Item = &EqBand> {
        self.bands.iter().chain(self.correction.iter())
    }

    pub fn on_channel(&self, channel: ChannelTarget) -> impl Iterator<Item = &EqBand> {
        self.all_bands().filter(move |b| b.channel == channel)
    }
}
