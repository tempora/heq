use std::f64::consts::PI;
use std::sync::OnceLock;

use super::Biquad;
use crate::model::{ChannelTarget, EqModel};

const FREQ_MIN: f64 = 20.0;
const FREQ_MAX: f64 = 20000.0;
const BINS: usize = 240;

const WEIGHTING_RATE: f64 = 48000.0;

struct Grid {
    freqs: [f64; BINS],
    weights: [f64; BINS],
    weight_sum: f64,
}

fn grid() -> &'static Grid {
    static GRID: OnceLock<Grid> = OnceLock::new();
    GRID.get_or_init(|| {
        let shelf = Biquad {
            b0: 1.53512485958697,
            b1: -2.69169618940638,
            b2: 1.19839281085285,
            a0: 1.0,
            a1: -1.69065929318241,
            a2: 0.73248077421585,
        };
        let high_pass = Biquad {
            b0: 1.0,
            b1: -2.0,
            b2: 1.0,
            a0: 1.0,
            a1: -1.99004745483398,
            a2: 0.99007225036621,
        };

        let log_min = FREQ_MIN.log10();
        let span = FREQ_MAX.log10() - log_min;

        let mut g = Grid {
            freqs: [0.0; BINS],
            weights: [0.0; BINS],
            weight_sum: 0.0,
        };

        for i in 0..BINS {
            let f = 10f64.powf(log_min + span * i as f64 / (BINS - 1) as f64);
            let w = 2.0 * PI * f / WEIGHTING_RATE;

            g.freqs[i] = f;
            g.weights[i] = 10f64.powf((shelf.gain_db(w) + high_pass.gain_db(w)) / 10.0);
            g.weight_sum += g.weights[i];
        }
        g
    })
}

pub fn level_db(model: &EqModel) -> f64 {
    let g = grid();
    let split = model.has_per_channel_bands();
    let mut energy = 0.0;

    for i in 0..BINS {
        let f = g.freqs[i];
        let gain = if split {
            let l = power(model.response_db(f, ChannelTarget::Left));
            let r = power(model.response_db(f, ChannelTarget::Right));
            (l + r) * 0.5
        } else {
            power(model.response_db(f, ChannelTarget::Both))
        };

        energy += g.weights[i] * gain;
    }

    let mean = energy / g.weight_sum;
    let curve = if mean <= 1e-12 {
        -120.0
    } else {
        10.0 * mean.log10()
    };
    curve + model.base_preamp_db()
}

fn power(db: f64) -> f64 {
    10f64.powf(db / 10.0)
}
