use super::{Backend, NullBackend};

// Which backends exist at all is a property of the platform, decided here at compile time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackendKind {
    Apo,
    Camilla,
    PipeWire,
}

#[cfg(windows)]
pub const SUPPORTED: &[BackendKind] = &[BackendKind::Apo];

#[cfg(target_os = "linux")]
pub const SUPPORTED: &[BackendKind] = &[BackendKind::Camilla, BackendKind::PipeWire];

#[cfg(target_os = "macos")]
pub const SUPPORTED: &[BackendKind] = &[BackendKind::Camilla];

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
pub const SUPPORTED: &[BackendKind] = &[BackendKind::Camilla];

impl BackendKind {
    pub fn label(self) -> &'static str {
        match self {
            BackendKind::Apo => "Equalizer APO",
            BackendKind::Camilla => "CamillaDSP",
            BackendKind::PipeWire => "PipeWire",
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        SUPPORTED.iter().copied().find(|k| k.label() == label)
    }

    pub fn create(self) -> Box<dyn Backend + Send> {
        match self {
            BackendKind::Apo => match super::ApoBackend::discover() {
                Some(b) => Box::new(b),
                None => Box::new(NullBackend::new("Equalizer APO not found")),
            },
            BackendKind::Camilla => Box::new(super::CamillaBackend::new(
                super::camilla::DEFAULT_URL,
                None,
            )),
            BackendKind::PipeWire => Box::new(super::PipeWireBackend::new(
                super::PipeWireBackend::default_path(),
            )),
        }
    }
}

pub fn default_kind() -> BackendKind {
    SUPPORTED[0]
}

pub fn default_backend() -> Box<dyn Backend + Send> {
    default_kind().create()
}
