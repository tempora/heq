use crate::backend::Snapshot;
use crate::format::num;
use crate::model::{BandId, ChannelTarget, EqBand, FilterKind};

pub fn build_config(s: &Snapshot) -> String {
    let mut out = String::new();

    if s.preamp_db.abs() > 0.001 {
        out.push_str(&format!("Preamp: {} dB\r\n", num(s.preamp_db, 1)));
    }

    let left: Vec<&EqBand> = s.on_channel(ChannelTarget::Left).collect();
    let right: Vec<&EqBand> = s.on_channel(ChannelTarget::Right).collect();

    append_bands(&mut out, s.on_channel(ChannelTarget::Both));
    append_channel(&mut out, "L", &left);
    append_channel(&mut out, "R", &right);

    if !left.is_empty() || !right.is_empty() {
        out.push_str("Channel: ALL\r\n");
    }

    out
}

fn append_channel(out: &mut String, channel: &str, bands: &[&EqBand]) {
    if bands.is_empty() {
        return;
    }
    out.push_str(&format!("Channel: {}\r\n", channel));
    append_bands(out, bands.iter().copied());
}

fn append_bands<'a>(out: &mut String, bands: impl Iterator<Item = &'a EqBand>) {
    for b in bands {
        for line in filter_lines(b) {
            out.push_str(&line);
            out.push_str("\r\n");
        }
    }
}

pub fn filter_lines(b: &EqBand) -> Vec<String> {
    let state = if b.enabled { "ON" } else { "OFF" };
    let f = num(b.freq, 2);
    let gain = num(b.gain_db, 2);
    let q = num(b.q, 4);

    let one = |tag: &str| {
        vec![format!(
            "Filter: {} {} Fc {} Hz Gain {} dB Q {}",
            state, tag, f, gain, q
        )]
    };
    let no_gain = |tag: &str| vec![format!("Filter: {} {} Fc {} Hz Q {}", state, tag, f, q)];
    let cascade = |tag: &str| {
        b.cut_qs()
            .into_iter()
            .map(|q| format!("Filter: {} {} Fc {} Hz Q {}", state, tag, f, num(q, 4)))
            .collect()
    };

    match b.kind {
        FilterKind::Bell => one("PK"),
        FilterKind::LowShelf => one("LSC"),
        FilterKind::HighShelf => one("HSC"),
        FilterKind::Notch => no_gain("NO"),
        FilterKind::BandPass => no_gain("BP"),
        FilterKind::AllPass => no_gain("AP"),
        FilterKind::LowCut => cascade("HPQ"),
        FilterKind::HighCut => cascade("LPQ"),
    }
}

// reading

#[derive(Debug, Default)]
pub struct ParseResult {
    pub bands: Vec<EqBand>,
    pub preamp: Option<f64>,
    pub warnings: Vec<String>,
}

pub fn parse(text: &str) -> ParseResult {
    let mut result = ParseResult::default();
    let mut channel = ChannelTarget::Both;

    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some(rest) = strip_key(line, "Channel:") {
            channel = match rest.trim().to_uppercase().as_str() {
                "L" | "1" | "LEFT" => ChannelTarget::Left,
                "R" | "2" | "RIGHT" => ChannelTarget::Right,
                _ => ChannelTarget::Both,
            };
            continue;
        }

        if let Some(rest) = strip_key(line, "Preamp:") {
            if let Some(v) = tokens(rest).first().and_then(|t| try_num(t)) {
                result.preamp = Some(v);
            }
            continue;
        }

        let Some(colon) = line.find(':') else { continue };
        if !line[..colon].trim().starts_with("Filter") {
            continue;
        }

        if let Some(mut band) = parse_filter(&line[colon + 1..], &mut result.warnings) {
            band.channel = channel;
            result.bands.push(band);
        }
    }

    result
}

fn parse_filter(body: &str, warnings: &mut Vec<String>) -> Option<EqBand> {
    let t = tokens(body);
    if t.is_empty() {
        return None;
    }

    let mut i = 0;
    let mut enabled = true;
    if t[i].eq_ignore_ascii_case("ON") {
        i += 1;
    } else if t[i].eq_ignore_ascii_case("OFF") {
        enabled = false;
        i += 1;
    }

    let type_ = t.get(i)?.to_uppercase();
    i += 1;

    if type_ == "NONE" {
        return None; // AutoEq pads unused slots with these
    }

    let mut forced_q = None;
    if (type_ == "LS" || type_ == "HS")
        && t.get(i).is_some_and(|v| v.to_lowercase().ends_with("db"))
    {
        forced_q = Some(if t[i].starts_with('6') { 0.5 } else { 0.7071 });
        i += 1;
    }

    let (mut fc, mut gain) = (1000.0, 0.0);
    let (mut q, mut bw) = (f64::NAN, f64::NAN);

    while i < t.len() {
        let k = t[i].to_uppercase();
        match k.as_str() {
            "FC" => {
                if let Some(v) = t.get(i + 1).and_then(|s| try_num(s)) {
                    fc = v;
                    i += 1;
                }
            }
            "GAIN" => {
                if let Some(v) = t.get(i + 1).and_then(|s| try_num(s)) {
                    gain = v;
                    i += 1;
                }
            }
            "Q" => {
                if let Some(v) = t.get(i + 1).and_then(|s| try_num(s)) {
                    q = v;
                    i += 1;
                }
            }
            "BW" => {
                if let Some(v) = t.get(i + 2).and_then(|s| try_num(s)) {
                    bw = v;
                    i += 2;
                }
            }
            _ => {}
        }
        i += 1;
    }

    if q.is_nan() {
        q = if !bw.is_nan() {
            bandwidth_to_q(bw)
        } else {
            forced_q.unwrap_or(0.7071)
        };
    }

    let kind = match type_.as_str() {
        "PK" | "PEQ" | "MODAL" => FilterKind::Bell,
        "LS" | "LSC" | "LSQ" => FilterKind::LowShelf,
        "HS" | "HSC" | "HSQ" => FilterKind::HighShelf,
        "HP" | "HPQ" => FilterKind::LowCut,
        "LP" | "LPQ" => FilterKind::HighCut,
        "NO" => FilterKind::Notch,
        "BP" => FilterKind::BandPass,
        "AP" => FilterKind::AllPass,
        _ => {
            warnings.push(format!("Skipped unsupported filter type '{}'.", type_));
            return None;
        }
    };

    if type_ == "HP" || type_ == "LP" {
        q = 0.7071; // plain HP/LP are Butterworth
    }

    let mut b = EqBand::new(BandId(0));
    b.kind = kind;
    b.freq = fc;
    b.gain_db = if kind.uses_gain() { gain } else { 0.0 };
    b.q = q;
    b.enabled = enabled;
    b.slope_db_per_oct = 12;
    b.clamp();
    Some(b)
}

pub fn bandwidth_to_q(bw_octaves: f64) -> f64 {
    if bw_octaves <= 0.0 {
        return 0.7071;
    }
    let p = 2f64.powf(bw_octaves);
    p.sqrt() / (p - 1.0)
}

fn strip_key<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    line.get(..key.len())
        .filter(|head| head.eq_ignore_ascii_case(key))
        .map(|_| &line[key.len()..])
}

fn tokens(s: &str) -> Vec<&str> {
    s.split([' ', '\t']).filter(|t| !t.is_empty()).collect()
}

fn try_num(s: &str) -> Option<f64> {
    s.trim().replace(',', ".").parse().ok()
}
