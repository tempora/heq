// Every number that reaches a config file or a text field goes through here, so a locale can
// never put a comma in one.

pub fn num(v: f64, decimals: usize) -> String {
    let v = if v.is_finite() { v } else { 0.0 };
    let scale = 10f64.powi(decimals as i32);
    let rounded = (v * scale).round() / scale;

    let mut s = format!("{:.*}", decimals, rounded);
    if s.contains('.') {
        s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    }
    if s == "-0" {
        s = "0".to_string();
    }
    s
}

pub fn freq_label(f: f64) -> String {
    if f >= 1000.0 {
        format!("{}k", num(f / 1000.0, 1))
    } else {
        num(f, 0)
    }
}

pub fn gain_label(db: f64) -> String {
    if db > 0.0 {
        format!("+{}", num(db, 1))
    } else {
        num(db, 1)
    }
}

// accepts "1k", "1.2 kHz", "440hz", "-3 dB"
pub fn parse_number(s: &str) -> Option<f64> {
    let s = s.trim().replace(',', ".").to_lowercase();

    for suffix in ["khz", "hz", "db", "k"] {
        if let Some(head) = s.strip_suffix(suffix) {
            let v: f64 = head.trim().parse().ok()?;
            let kilo = suffix == "khz" || suffix == "k";
            return Some(if kilo { v * 1000.0 } else { v });
        }
    }

    s.parse().ok()
}
