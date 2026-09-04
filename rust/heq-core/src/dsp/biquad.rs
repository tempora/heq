use std::f64::consts::PI;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Biquad {
    pub b0: f64,
    pub b1: f64,
    pub b2: f64,
    pub a0: f64,
    pub a1: f64,
    pub a2: f64,
}

impl Biquad {
    pub fn magnitude_squared(&self, w: f64) -> f64 {
        let s = (w * 0.5).sin();
        let phi = s * s;

        let b_sum = self.b0 + self.b1 + self.b2;
        let a_sum = self.a0 + self.a1 + self.a2;

        let num = b_sum * b_sum
            - 4.0 * (self.b0 * self.b1 + 4.0 * self.b0 * self.b2 + self.b1 * self.b2) * phi
            + 16.0 * self.b0 * self.b2 * phi * phi;
        let den = a_sum * a_sum
            - 4.0 * (self.a0 * self.a1 + 4.0 * self.a0 * self.a2 + self.a1 * self.a2) * phi
            + 16.0 * self.a0 * self.a2 * phi * phi;

        if den <= 0.0 {
            return 0.0;
        }
        (num.max(0.0)) / den
    }

    pub fn gain_db(&self, w: f64) -> f64 {
        let m2 = self.magnitude_squared(w);
        if m2 <= 1e-20 {
            -200.0
        } else {
            10.0 * m2.log10()
        }
    }

    pub fn peaking(freq: f64, sample_rate: f64, gain_db: f64, q: f64) -> Self {
        let w0 = omega(freq, sample_rate);
        let a = 10f64.powf(gain_db / 40.0);
        let alpha = w0.sin() / (2.0 * q);
        let cos = w0.cos();

        Biquad {
            b0: 1.0 + alpha * a,
            b1: -2.0 * cos,
            b2: 1.0 - alpha * a,
            a0: 1.0 + alpha / a,
            a1: -2.0 * cos,
            a2: 1.0 - alpha / a,
        }
    }

    pub fn low_shelf(freq: f64, sample_rate: f64, gain_db: f64, q: f64) -> Self {
        let w0 = omega(freq, sample_rate);
        let a = 10f64.powf(gain_db / 40.0);
        let alpha = w0.sin() / (2.0 * q);
        let cos = w0.cos();
        let sqrt_a2_alpha = 2.0 * a.sqrt() * alpha;

        Biquad {
            b0: a * ((a + 1.0) - (a - 1.0) * cos + sqrt_a2_alpha),
            b1: 2.0 * a * ((a - 1.0) - (a + 1.0) * cos),
            b2: a * ((a + 1.0) - (a - 1.0) * cos - sqrt_a2_alpha),
            a0: (a + 1.0) + (a - 1.0) * cos + sqrt_a2_alpha,
            a1: -2.0 * ((a - 1.0) + (a + 1.0) * cos),
            a2: (a + 1.0) + (a - 1.0) * cos - sqrt_a2_alpha,
        }
    }

    pub fn high_shelf(freq: f64, sample_rate: f64, gain_db: f64, q: f64) -> Self {
        let w0 = omega(freq, sample_rate);
        let a = 10f64.powf(gain_db / 40.0);
        let alpha = w0.sin() / (2.0 * q);
        let cos = w0.cos();
        let sqrt_a2_alpha = 2.0 * a.sqrt() * alpha;

        Biquad {
            b0: a * ((a + 1.0) + (a - 1.0) * cos + sqrt_a2_alpha),
            b1: -2.0 * a * ((a - 1.0) + (a + 1.0) * cos),
            b2: a * ((a + 1.0) + (a - 1.0) * cos - sqrt_a2_alpha),
            a0: (a + 1.0) - (a - 1.0) * cos + sqrt_a2_alpha,
            a1: 2.0 * ((a - 1.0) - (a + 1.0) * cos),
            a2: (a + 1.0) - (a - 1.0) * cos - sqrt_a2_alpha,
        }
    }

    pub fn high_pass(freq: f64, sample_rate: f64, q: f64) -> Self {
        let w0 = omega(freq, sample_rate);
        let alpha = w0.sin() / (2.0 * q);
        let cos = w0.cos();

        Biquad {
            b0: (1.0 + cos) / 2.0,
            b1: -(1.0 + cos),
            b2: (1.0 + cos) / 2.0,
            a0: 1.0 + alpha,
            a1: -2.0 * cos,
            a2: 1.0 - alpha,
        }
    }

    pub fn low_pass(freq: f64, sample_rate: f64, q: f64) -> Self {
        let w0 = omega(freq, sample_rate);
        let alpha = w0.sin() / (2.0 * q);
        let cos = w0.cos();

        Biquad {
            b0: (1.0 - cos) / 2.0,
            b1: 1.0 - cos,
            b2: (1.0 - cos) / 2.0,
            a0: 1.0 + alpha,
            a1: -2.0 * cos,
            a2: 1.0 - alpha,
        }
    }

    pub fn notch(freq: f64, sample_rate: f64, q: f64) -> Self {
        let w0 = omega(freq, sample_rate);
        let alpha = w0.sin() / (2.0 * q);
        let cos = w0.cos();

        Biquad {
            b0: 1.0,
            b1: -2.0 * cos,
            b2: 1.0,
            a0: 1.0 + alpha,
            a1: -2.0 * cos,
            a2: 1.0 - alpha,
        }
    }

    pub fn band_pass(freq: f64, sample_rate: f64, q: f64) -> Self {
        let w0 = omega(freq, sample_rate);
        let alpha = w0.sin() / (2.0 * q);
        let cos = w0.cos();

        Biquad {
            b0: alpha,
            b1: 0.0,
            b2: -alpha,
            a0: 1.0 + alpha,
            a1: -2.0 * cos,
            a2: 1.0 - alpha,
        }
    }

    pub fn all_pass(freq: f64, sample_rate: f64, q: f64) -> Self {
        let w0 = omega(freq, sample_rate);
        let alpha = w0.sin() / (2.0 * q);
        let cos = w0.cos();

        Biquad {
            b0: 1.0 - alpha,
            b1: -2.0 * cos,
            b2: 1.0 + alpha,
            a0: 1.0 + alpha,
            a1: -2.0 * cos,
            a2: 1.0 - alpha,
        }
    }
}

fn omega(freq: f64, sample_rate: f64) -> f64 {
    let f = freq.min(sample_rate * 0.4999).max(1.0);
    2.0 * PI * f / sample_rate
}

pub fn butterworth_qs(order: usize) -> Vec<f64> {
    let mut order = order.max(2);
    if order % 2 != 0 {
        order += 1;
    }
    let sections = order / 2;
    (0..sections)
        .map(|k| 1.0 / (2.0 * ((2.0 * k as f64 + 1.0) * PI / (2.0 * order as f64)).cos()))
        .collect()
}
