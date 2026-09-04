use std::env;
use std::fs;
use std::path::PathBuf;

use crate::backend::{Backend, BackendError, Snapshot, Status};
use crate::format::num;
use crate::model::{ChannelTarget, EqBand, FilterKind};

pub const FILE_NAME: &str = "99-heq.conf";

// PipeWire loads a filter-chain at startup, so this backend is an export target: the file is
// written immediately, but nothing is heard until PipeWire restarts.
pub struct PipeWireBackend {
    path: PathBuf,
    status: Status,
}

impl PipeWireBackend {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        PipeWireBackend {
            path: path.into(),
            status: Status::offline("not written yet"),
        }
    }

    pub fn default_path() -> PathBuf {
        let home = env::var("HOME").map(PathBuf::from).unwrap_or_default();
        env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home.join(".config"))
            .join("pipewire/pipewire.conf.d")
            .join(FILE_NAME)
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Backend for PipeWireBackend {
    fn name(&self) -> &str {
        "PipeWire"
    }

    fn apply(&mut self, snapshot: &Snapshot) -> Result<(), BackendError> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }

        fs::write(&self.path, build_config(snapshot))?;
        self.status = Status::ok("written — restart PipeWire to hear it");
        Ok(())
    }

    fn status(&self) -> Status {
        self.status.clone()
    }
}

pub fn build_config(s: &Snapshot) -> String {
    let left = chain(s, ChannelTarget::Left, "l");
    let right = chain(s, ChannelTarget::Right, "r");

    let mut nodes = String::new();
    let mut links = String::new();

    for side in [&left, &right] {
        for node in &side.nodes {
            nodes.push_str(&format!("          {}\n", node));
        }
        for link in &side.links {
            links.push_str(&format!("          {}\n", link));
        }
    }

    format!(
        "# written by heq — do not edit, your changes will be overwritten\n\
         context.modules = [\n\
         \x20 {{ name = libpipewire-module-filter-chain\n\
         \x20   args = {{\n\
         \x20     node.description = \"heq\"\n\
         \x20     media.name       = \"heq\"\n\
         \x20     filter.graph = {{\n\
         \x20       nodes = [\n{nodes}\
         \x20       ]\n\
         \x20       links = [\n{links}\
         \x20       ]\n\
         \x20       inputs  = [ \"{lin}\" \"{rin}\" ]\n\
         \x20       outputs = [ \"{lout}\" \"{rout}\" ]\n\
         \x20     }}\n\
         \x20     capture.props = {{\n\
         \x20       node.name   = \"heq\"\n\
         \x20       media.class = \"Audio/Sink\"\n\
         \x20       audio.channels = 2\n\
         \x20       audio.position = [ FL FR ]\n\
         \x20     }}\n\
         \x20     playback.props = {{\n\
         \x20       node.name    = \"heq_out\"\n\
         \x20       node.passive = true\n\
         \x20       audio.channels = 2\n\
         \x20       audio.position = [ FL FR ]\n\
         \x20     }}\n\
         \x20   }}\n\
         \x20 }}\n\
         ]\n",
        nodes = nodes,
        links = links,
        lin = left.input,
        rin = right.input,
        lout = left.output,
        rout = right.output,
    )
}

struct Chain {
    nodes: Vec<String>,
    links: Vec<String>,
    input: String,
    output: String,
}

fn chain(s: &Snapshot, side: ChannelTarget, tag: &str) -> Chain {
    let mut nodes = Vec::new();
    let mut names = Vec::new();

    if !s.bypassed && s.preamp_db.abs() > 0.001 {
        // builtin `linear` takes a linear amplitude, not dB
        let amp = 10f64.powf(s.preamp_db / 20.0);
        push_node(
            &mut nodes,
            &mut names,
            tag,
            "linear",
            &format!("\"Gain\" = {}", num(amp, 6)),
        );
    }

    if !s.bypassed {
        for band in s.all_bands() {
            if !band.enabled || !band.channel.applies_to(side) {
                continue;
            }
            for (label, controls) in band_nodes(band) {
                push_node(&mut nodes, &mut names, tag, &label, &controls);
            }
        }
    }

    if names.is_empty() {
        push_node(&mut nodes, &mut names, tag, "copy", "");
    }

    let links = names
        .windows(2)
        .map(|w| format!("{{ output = \"{}:Out\" input = \"{}:In\" }}", w[0], w[1]))
        .collect();

    Chain {
        input: format!("{}:In", names.first().expect("at least one node")),
        output: format!("{}:Out", names.last().expect("at least one node")),
        nodes,
        links,
    }
}

fn push_node(nodes: &mut Vec<String>, names: &mut Vec<String>, tag: &str, label: &str, controls: &str) {
    let name = format!("heq_{}_{}", tag, names.len() + 1);
    nodes.push(format!(
        "{{ type = builtin name = {} label = {} control = {{ {} }} }}",
        name, label, controls
    ));
    names.push(name);
}

fn band_nodes(b: &EqBand) -> Vec<(String, String)> {
    let peq = |label: &str| {
        vec![(
            label.to_string(),
            format!(
                "\"Freq\" = {} \"Q\" = {} \"Gain\" = {}",
                num(b.freq, 2),
                num(b.q, 4),
                num(b.gain_db, 2)
            ),
        )]
    };
    let no_gain = |label: &str| {
        vec![(
            label.to_string(),
            format!("\"Freq\" = {} \"Q\" = {}", num(b.freq, 2), num(b.q, 4)),
        )]
    };
    let cascade = |label: &str| {
        b.cut_qs()
            .into_iter()
            .map(|q| {
                (
                    label.to_string(),
                    format!("\"Freq\" = {} \"Q\" = {}", num(b.freq, 2), num(q, 4)),
                )
            })
            .collect()
    };

    match b.kind {
        FilterKind::Bell => peq("bq_peaking"),
        FilterKind::LowShelf => peq("bq_lowshelf"),
        FilterKind::HighShelf => peq("bq_highshelf"),
        FilterKind::Notch => no_gain("bq_notch"),
        FilterKind::BandPass => no_gain("bq_bandpass"),
        FilterKind::AllPass => no_gain("bq_allpass"),
        FilterKind::LowCut => cascade("bq_highpass"),
        FilterKind::HighCut => cascade("bq_lowpass"),
    }
}
