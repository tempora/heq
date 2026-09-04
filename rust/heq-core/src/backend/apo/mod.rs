pub mod format;
pub mod install;
pub mod writer;

pub use install::{find_config_dir, scan_devices, ApoDevice, HEQ_FILE_NAME, MAIN_CONFIG_NAME};
pub use writer::{ApoWriter, IncludeSpot, IncludeStatus};

use crate::backend::{Backend, BackendError, Snapshot, Status};

pub struct ApoBackend {
    writer: ApoWriter,
    status: Status,
}

impl ApoBackend {
    pub fn new(writer: ApoWriter) -> Self {
        ApoBackend {
            writer,
            status: Status::offline("not applied yet"),
        }
    }

    pub fn discover() -> Option<Self> {
        find_config_dir().map(|dir| Self::new(ApoWriter::new(dir)))
    }

    pub fn writer(&self) -> &ApoWriter {
        &self.writer
    }
}

impl Backend for ApoBackend {
    fn name(&self) -> &str {
        "Equalizer APO"
    }

    fn apply(&mut self, snapshot: &Snapshot) -> Result<(), BackendError> {
        match self.writer.write_eq(snapshot) {
            Ok(()) => {
                let spot = self.writer.find_include();
                self.status = if spot.duplicated {
                    Status::ok("config.txt includes heq.txt more than once")
                } else if spot.exists {
                    match spot.device {
                        Some(d) => Status::ok(format!("applied under {}", d)),
                        None => Status::ok("applied"),
                    }
                } else {
                    Status::offline("config.txt has no Include: heq.txt")
                };
                Ok(())
            }
            Err(e) => {
                self.status = Status::offline(format!("write failed: {}", e));
                Err(BackendError::Io(e))
            }
        }
    }

    fn status(&self) -> Status {
        self.status.clone()
    }
}
