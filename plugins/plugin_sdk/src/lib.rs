use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::io::{self, BufRead, Write};

pub type PluginMap = HashMap<String, Value>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PluginMessage {
    #[serde(rename = "request")]
    Request(PluginRequest),
    #[serde(rename = "response")]
    Response(PluginResponse),
    #[serde(rename = "event")]
    Event(PluginEvent),
    #[serde(rename = "error")]
    Error(PluginError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum PluginRequest {
    #[serde(rename = "init")]
    Init { id: String, config: PluginMap },
    #[serde(rename = "execute")]
    Execute {
        id: String,
        command: String,
        params: PluginMap,
    },
    #[serde(rename = "info")]
    Info { id: String },
    #[serde(rename = "shutdown")]
    Shutdown { id: String },
    #[serde(rename = "notify")]
    Notify {
        id: String,
        event_type: String,
        data: PluginMap,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResponse {
    pub id: String,
    pub success: bool,
    pub data: Option<Value>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event")]
pub enum PluginEvent {
    #[serde(rename = "ready")]
    Ready {
        name: String,
        version: String,
        capabilities: Vec<String>,
    },
    #[serde(rename = "notify")]
    Notify { level: String, message: String },
    #[serde(rename = "command")]
    Command { command: String, params: PluginMap },
    #[serde(rename = "data")]
    Data { data_type: String, content: Value },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginError {
    pub code: String,
    pub message: String,
    pub details: Option<Value>,
}

pub struct PluginApi;

impl PluginApi {
    pub fn send_message(&mut self, msg: &PluginMessage) -> io::Result<()> {
        let json = serde_json::to_string(msg)?;
        println!("{}", json);
        io::stdout().flush()?;
        Ok(())
    }

    pub fn send_response(
        &mut self,
        id: String,
        success: bool,
        data: Option<Value>,
        error: Option<String>,
    ) -> io::Result<()> {
        let response = PluginResponse {
            id,
            success,
            data,
            error,
        };
        self.send_message(&PluginMessage::Response(response))
    }

    pub fn send_ok(&mut self, id: String, data: Option<Value>) -> io::Result<()> {
        self.send_response(id, true, data, None)
    }

    pub fn send_err(&mut self, id: String, error: impl Into<String>) -> io::Result<()> {
        self.send_response(id, false, None, Some(error.into()))
    }

    pub fn send_event(&mut self, event: PluginEvent) -> io::Result<()> {
        self.send_message(&PluginMessage::Event(event))
    }

    pub fn send_ready(
        &mut self,
        name: impl Into<String>,
        version: impl Into<String>,
        capabilities: Vec<String>,
    ) -> io::Result<()> {
        self.send_event(PluginEvent::Ready {
            name: name.into(),
            version: version.into(),
            capabilities,
        })
    }

    pub fn notify(&mut self, level: impl Into<String>, message: impl Into<String>) -> io::Result<()> {
        self.send_event(PluginEvent::Notify {
            level: level.into(),
            message: message.into(),
        })
    }

    pub fn send_command(&mut self, command: impl Into<String>, params: PluginMap) -> io::Result<()> {
        self.send_event(PluginEvent::Command {
            command: command.into(),
            params,
        })
    }
}

pub trait PluginHandler {
    fn on_init(&mut self, api: &mut PluginApi, id: String, config: PluginMap) -> io::Result<()>;

    fn on_execute(
        &mut self,
        api: &mut PluginApi,
        id: String,
        command: String,
        params: PluginMap,
    ) -> io::Result<()>;

    fn on_info(&mut self, api: &mut PluginApi, id: String) -> io::Result<()> {
        api.send_ok(id, None)
    }

    fn on_shutdown(&mut self, api: &mut PluginApi, id: String) -> io::Result<()> {
        api.send_ok(id, None)
    }

    fn on_notify(
        &mut self,
        api: &mut PluginApi,
        id: String,
        event_type: String,
        _data: PluginMap,
    ) -> io::Result<()> {
        api.send_ok(
            id,
            Some(serde_json::json!({
                "event_type": event_type,
                "handled": false
            })),
        )
    }
}

fn handle_message<H: PluginHandler>(
    handler: &mut H,
    api: &mut PluginApi,
    msg: PluginMessage,
) -> io::Result<bool> {
    match msg {
        PluginMessage::Request(req) => match req {
            PluginRequest::Init { id, config } => {
                handler.on_init(api, id, config)?;
                Ok(false)
            }
            PluginRequest::Execute {
                id,
                command,
                params,
            } => {
                handler.on_execute(api, id, command, params)?;
                Ok(false)
            }
            PluginRequest::Info { id } => {
                handler.on_info(api, id)?;
                Ok(false)
            }
            PluginRequest::Shutdown { id } => {
                handler.on_shutdown(api, id)?;
                Ok(true)
            }
            PluginRequest::Notify {
                id,
                event_type,
                data,
            } => {
                handler.on_notify(api, id, event_type, data)?;
                Ok(false)
            }
        },
        _ => Ok(false),
    }
}

pub fn run_plugin<H: PluginHandler>(handler: &mut H) {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut api = PluginApi;
    let mut buffer = String::new();

    loop {
        buffer.clear();
        match input.read_line(&mut buffer) {
            Ok(0) => break,
            Ok(_) => {
                let line = buffer.trim();
                if line.is_empty() {
                    continue;
                }

                match serde_json::from_str::<PluginMessage>(line) {
                    Ok(msg) => {
                        match handle_message(handler, &mut api, msg) {
                            Ok(should_exit) => {
                                if should_exit {
                                    break;
                                }
                            }
                            Err(e) => {
                                eprintln!("Error handling message: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to parse message: {} - Line: {}", e, line);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading from stdin: {}", e);
                break;
            }
        }
    }
}
