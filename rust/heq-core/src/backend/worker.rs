use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use super::{Backend, Snapshot, Status};

// The debounce that keeps a drag from making the backend reload every frame.
pub const DEBOUNCE: Duration = Duration::from_millis(140);

pub struct BackendWorker {
    tx: Sender<Snapshot>,
    state: Arc<Mutex<Status>>,
}

impl BackendWorker {
    pub fn spawn(mut backend: Box<dyn Backend + Send>) -> Self {
        let (tx, rx) = mpsc::channel::<Snapshot>();
        let state = Arc::new(Mutex::new(backend.status()));
        let shared = Arc::clone(&state);

        thread::spawn(move || {
            let mut pending: Option<Snapshot> = None;
            let mut due: Option<Instant> = None;

            loop {
                let wait = match due {
                    Some(at) => at.saturating_duration_since(Instant::now()),
                    None => Duration::from_secs(3600),
                };

                match rx.recv_timeout(wait) {
                    Ok(snapshot) => {
                        pending = Some(snapshot);
                        due = Some(Instant::now() + DEBOUNCE);
                    }
                    Err(RecvTimeoutError::Timeout) => {
                        if let Some(snapshot) = pending.take() {
                            let _ = backend.apply(&snapshot);
                            if let Ok(mut s) = shared.lock() {
                                *s = backend.status();
                            }
                        }
                        due = None;
                    }
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            }
        });

        BackendWorker { tx, state }
    }

    pub fn schedule(&self, snapshot: Snapshot) {
        let _ = self.tx.send(snapshot);
    }

    pub fn status(&self) -> Status {
        self.state
            .lock()
            .map(|s| s.clone())
            .unwrap_or_else(|_| Status::offline("backend thread died"))
    }
}
