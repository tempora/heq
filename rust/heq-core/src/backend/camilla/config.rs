use serde_json::{json, Map, Value};

use crate::backend::Snapshot;
use crate::model::{ChannelTarget, EqBand, FilterKind};

// heq owns the `filters` and `pipeline` sections; everything else in the user's CamillaDSP
// config (devices, mixers, resampling) is left exactly as it was found.
pub fn splice(config: &mut Value, s: &Snapshot) {
    let built = build(s);

    let Some(map) = config.as_object_mut() else {
        *config = json!({ "filters": built.filters, "pipeline": built.pipeline });
        return;
    };

    map.insert("filters".into(), Value::Object(built.filters));
    map.insert("pipeline".into(), Value::Array(built.pipeline));
}

pub struct Built {
    pub filters: Map<String, Value>,
    pub pipeline: Vec<Value>,
}

pub fn build(s: &Snapshot) -> Built {
    let mut filters = Map::new();
    let mut left: Vec<Value> = Vec::new();
    let mut right: Vec<Value> = Vec::new();

    if s.bypassed {
        return Built {
            filters,
            pipeline: Vec::new(),
        };
    }

    if s.preamp_db.abs() > 0.001 {
        filters.insert(
            "heq_preamp".into(),
            json!({
                "type": "Gain",
                "parameters": { "gain": s.preamp_db, "inverted": false, "mute": false },
            }),
        );
        left.push("heq_preamp".into());
        right.push("heq_preamp".into());
    }

    for (i, band) in s.all_bands().enumerate() {
        if !band.enabled {
            continue;
        }

        for (section, params) in band_parameters(band).into_iter().enumerate() {
            let name = if section == 0 {
                format!("heq_{}", i + 1)
            } else {
                format!("heq_{}_{}", i + 1, section + 1)
            };

            filters.insert(
                name.clone(),
                json!({ "type": "Biquad", "parameters": params }),
            );

            if band.channel != ChannelTarget::Right {
                left.push(name.clone().into());
            }
            if band.channel != ChannelTarget::Left {
                right.push(name.into());
            }
        }
    }

    let mut pipeline = Vec::new();
    for (channel, names) in [(0, left), (1, right)] {
        if names.is_empty() {
            continue;
        }
        pipeline.push(json!({
            "type": "Filter",
            "channels": [channel],
            "names": names,
        }));
    }

    Built { filters, pipeline }
}

// Same RBJ shapes the curve is drawn from, expressed the way CamillaDSP names them.
fn band_parameters(b: &EqBand) -> Vec<Value> {
    let freq = b.freq;
    let gain = b.gain_db;
    let q = b.q;

    match b.kind {
        FilterKind::Bell => vec![json!({ "type": "Peaking", "freq": freq, "gain": gain, "q": q })],
        FilterKind::LowShelf => {
            vec![json!({ "type": "Lowshelf", "freq": freq, "gain": gain, "q": q })]
        }
        FilterKind::HighShelf => {
            vec![json!({ "type": "Highshelf", "freq": freq, "gain": gain, "q": q })]
        }
        FilterKind::Notch => vec![json!({ "type": "Notch", "freq": freq, "q": q })],
        FilterKind::BandPass => vec![json!({ "type": "Bandpass", "freq": freq, "q": q })],
        FilterKind::AllPass => vec![json!({ "type": "Allpass", "freq": freq, "q": q })],
        FilterKind::LowCut => b
            .cut_qs()
            .into_iter()
            .map(|q| json!({ "type": "Highpass", "freq": freq, "q": q }))
            .collect(),
        FilterKind::HighCut => b
            .cut_qs()
            .into_iter()
            .map(|q| json!({ "type": "Lowpass", "freq": freq, "q": q }))
            .collect(),
    }
}
