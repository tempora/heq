use super::{Backend, BackendError, Snapshot, Status};

// Stands in when no output backend was found, so the editor still runs.
pub struct NullBackend {
    message: String,
}

impl NullBackend {
    pub fn new(message: impl Into<String>) -> Self {
        NullBackend {
            message: message.into(),
        }
    }
}

impl Backend for NullBackend {
    fn name(&self) -> &str {
        "none"
    }

    fn apply(&mut self, _snapshot: &Snapshot) -> Result<(), BackendError> {
        Ok(())
    }

    fn status(&self) -> Status {
        Status::offline(self.message.clone())
    }
}
