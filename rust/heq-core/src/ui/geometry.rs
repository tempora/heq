use crate::format;

pub const FREQ_MIN: f64 = 20.0;
pub const FREQ_MAX: f64 = 20000.0;

pub const DEFAULT_Q: f64 = 0.7;

pub const HANDLE_RADIUS: f64 = 6.0;
pub const HANDLE_HIT_RADIUS: f64 = 11.0;
pub const AXIS_BOTTOM: f64 = 18.0; // room for frequency labels
pub const AXIS_RIGHT: f64 = 30.0; // room for dB labels

// X is log frequency, Y is linear dB over ±db_range. Coordinates are local to the curve rect.
#[derive(Clone, Copy, Debug)]
pub struct Plot {
    pub width: f64,
    pub height: f64,
    pub db_range: f64,
}

impl Plot {
    pub fn new(width: f64, height: f64, db_range: f64) -> Self {
        Plot {
            width,
            height,
            db_range: db_range.clamp(3.0, 36.0),
        }
    }

    pub fn plot_width(&self) -> f64 {
        (self.width - AXIS_RIGHT).max(1.0)
    }

    pub fn plot_height(&self) -> f64 {
        (self.height - AXIS_BOTTOM).max(1.0)
    }

    pub fn freq_to_x(&self, f: f64) -> f64 {
        (f.max(1e-6).log10() - log_min()) / log_span() * self.plot_width()
    }

    pub fn x_to_freq(&self, x: f64) -> f64 {
        10f64.powf(log_min() + x / self.plot_width() * log_span())
    }

    pub fn db_to_y(&self, db: f64) -> f64 {
        self.plot_height() * 0.5 * (1.0 - db / self.db_range)
    }

    pub fn y_to_db(&self, y: f64) -> f64 {
        (1.0 - y / (self.plot_height() * 0.5)) * self.db_range
    }

    pub fn clamp_to_range(&self, db: f64) -> f64 {
        db.clamp(-self.db_range, self.db_range)
    }

    pub fn db_step(&self) -> f64 {
        if self.db_range <= 6.0 {
            2.0
        } else if self.db_range <= 12.0 {
            3.0
        } else if self.db_range <= 18.0 {
            6.0
        } else {
            10.0
        }
    }

    pub fn db_lines(&self) -> Vec<f64> {
        let step = self.db_step();
        let mut out = Vec::new();
        let mut db = -self.db_range;
        while db <= self.db_range + 1e-9 {
            out.push(db);
            db += step;
        }
        out
    }
}

fn log_min() -> f64 {
    FREQ_MIN.log10()
}

fn log_span() -> f64 {
    FREQ_MAX.log10() - log_min()
}

// 1-2-5 of each decade are labelled, the rest are minor grid lines
pub fn frequency_ticks() -> Vec<(f64, Option<String>)> {
    let mut out = Vec::new();
    for decade in [10.0, 100.0, 1000.0, 10000.0] {
        for m in 1..=9 {
            let f = decade * m as f64;
            if f < FREQ_MIN || f > FREQ_MAX {
                continue;
            }
            let label = matches!(m, 1 | 2 | 5).then(|| format::freq_label(f));
            out.push((f, label));
        }
    }
    out
}
