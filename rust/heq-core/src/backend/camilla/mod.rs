pub mod config;

use std::fs;
use std::net::TcpStream;
use std::path::PathBuf;

use serde_json::{json, Value};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};

use crate::backend::{yaml, Backend, BackendError, Snapshot, Status};

pub const DEFAULT_URL: &str = "ws://127.0.0.1:1234";

type Socket = WebSocket<MaybeTlsStream<TcpStream>>;

pub struct CamillaBackend {
    url: String,
    config_path: Option<PathBuf>,
    socket: Option<Socket>,
    status: Status,
}

impl CamillaBackend {
    pub fn new(url: impl Into<String>, config_path: Option<PathBuf>) -> Self {
        CamillaBackend {
            url: url.into(),
            config_path,
            socket: None,
            status: Status::offline("not connected"),
        }
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn set_config_path(&mut self, path: Option<PathBuf>) {
        self.config_path = path;
    }

    pub fn disconnect(&mut self) {
        if let Some(mut socket) = self.socket.take() {
            let _ = socket.close(None);
        }
        self.status = Status::offline("not connected");
    }

    fn connect(&mut self) -> Result<&mut Socket, BackendError> {
        if self.socket.is_none() {
            let (socket, _) = tungstenite::connect(&self.url)
                .map_err(|e| BackendError::NotAvailable(format!("{}: {}", self.url, e)))?;
            self.socket = Some(socket);
        }
        Ok(self.socket.as_mut().expect("connected"))
    }

    // A dropped connection is normal — camilladsp restarts. One retry, then report it.
    fn call(&mut self, request: Value) -> Result<Value, BackendError> {
        match self.round_trip(request.clone()) {
            Ok(v) => Ok(v),
            Err(e) => {
                self.socket = None;
                match self.round_trip(request) {
                    Ok(v) => Ok(v),
                    Err(_) => Err(e),
                }
            }
        }
    }

    fn round_trip(&mut self, request: Value) -> Result<Value, BackendError> {
        let socket = self.connect()?;

        socket
            .send(Message::text(request.to_string()))
            .map_err(|e| BackendError::Protocol(e.to_string()))?;

        loop {
            let msg = socket
                .read()
                .map_err(|e| BackendError::Protocol(e.to_string()))?;

            match msg {
                Message::Text(text) => {
                    return serde_json::from_str(&text)
                        .map_err(|e| BackendError::Protocol(e.to_string()))
                }
                Message::Close(_) => {
                    return Err(BackendError::Protocol("connection closed".into()))
                }
                _ => continue,
            }
        }
    }

    pub fn version(&mut self) -> Result<String, BackendError> {
        let reply = self.call(json!("GetVersion"))?;
        result_value(&reply, "GetVersion")?
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| BackendError::Protocol("GetVersion returned no version".into()))
    }

    fn fetch_config(&mut self) -> Result<Value, BackendError> {
        let reply = self.call(json!("GetConfigJson"))?;
        let text = result_value(&reply, "GetConfigJson")?
            .as_str()
            .ok_or_else(|| BackendError::Protocol("GetConfigJson returned no config".into()))?
            .to_string();

        serde_json::from_str(&text).map_err(|e| BackendError::Protocol(e.to_string()))
    }

    fn push_config(&mut self, cfg: &Value) -> Result<(), BackendError> {
        let reply = self.call(json!({ "SetConfigJson": cfg.to_string() }))?;
        result_value(&reply, "SetConfigJson").map(|_| ())
    }
}

impl Backend for CamillaBackend {
    fn name(&self) -> &str {
        "CamillaDSP"
    }

    fn apply(&mut self, snapshot: &Snapshot) -> Result<(), BackendError> {
        let mut cfg = match self.fetch_config() {
            Ok(cfg) => cfg,
            Err(e) => {
                self.socket = None;
                self.status = Status::offline(format!("CamillaDSP unreachable ({})", e));
                return Err(e);
            }
        };

        config::splice(&mut cfg, snapshot);

        if let Err(e) = self.push_config(&cfg) {
            self.socket = None;
            self.status = Status::offline(format!("CamillaDSP refused the config ({})", e));
            return Err(e);
        }

        self.status = Status::ok(format!("applied to {}", self.url));

        if let Some(path) = &self.config_path {
            if let Err(e) = fs::write(path, yaml::to_yaml(&cfg)) {
                self.status = Status::ok(format!("applied, but {} not written: {}", path.display(), e));
            }
        }

        Ok(())
    }

    fn status(&self) -> Status {
        self.status.clone()
    }
}

fn result_value<'a>(reply: &'a Value, command: &str) -> Result<&'a Value, BackendError> {
    let body = reply
        .get(command)
        .ok_or_else(|| BackendError::Protocol(format!("no reply to {}", command)))?;

    match body.get("result").and_then(Value::as_str) {
        Some("Ok") => Ok(body.get("value").unwrap_or(&Value::Null)),
        Some(other) => Err(BackendError::Protocol(format!("{}: {}", command, other))),
        None => Err(BackendError::Protocol(format!(
            "{}: malformed reply",
            command
        ))),
    }
}
