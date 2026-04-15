use plugin_sdk::{run_plugin, PluginApi, PluginHandler, PluginMap};
use std::io;

struct SimplePlugin;

impl PluginHandler for SimplePlugin {
    fn on_init(&mut self, api: &mut PluginApi, id: String, config: PluginMap) -> io::Result<()> {
        api.send_ready(
            "Simple Plugin",
            "0.1.0",
            vec!["echo".to_string(), "greet".to_string()],
        )?;

        let response_data = serde_json::json!({
            "message": "Plugin initialized successfully",
            "config": config
        });
        api.send_ok(id, Some(response_data))
    }

    fn on_execute(
        &mut self,
        api: &mut PluginApi,
        id: String,
        command: String,
        params: PluginMap,
    ) -> io::Result<()> {
        match command.as_str() {
            "echo" => {
                let text = params
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Hello from plugin!");
                api.send_ok(id, Some(serde_json::json!({ "echoed": text })))
            }
            "greet" => {
                let name = params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("World");
                let greeting = format!("Hello, {}! This is a simple plugin demo.", name);
                api.notify("info", greeting.clone())?;
                api.send_ok(id, Some(serde_json::json!({ "greeting": greeting })))
            }
            "get_info" => api.send_ok(
                id,
                Some(serde_json::json!({
                    "name": "Simple Plugin",
                    "version": "0.1.0",
                    "description": "A simple plugin demo for egscribe",
                    "author": "egscribe",
                    "capabilities": ["echo", "greet", "get_info"]
                })),
            ),
            _ => api.send_err(id, format!("Unknown command: {}", command)),
        }
    }

    fn on_info(&mut self, api: &mut PluginApi, id: String) -> io::Result<()> {
        api.send_ok(
            id,
            Some(serde_json::json!({
                "name": "Simple Plugin",
                "version": "0.1.0",
                "description": "A simple plugin demo for egscribe",
                "capabilities": ["echo", "greet", "get_info"]
            })),
        )
    }

    fn on_shutdown(&mut self, api: &mut PluginApi, id: String) -> io::Result<()> {
        api.notify("info", "Plugin is shutting down...")?;
        api.send_ok(id, None)
    }

    fn on_notify(
        &mut self,
        api: &mut PluginApi,
        id: String,
        event_type: String,
        data: PluginMap,
    ) -> io::Result<()> {
        match event_type.as_str() {
            "line_changed" => {
                let line_no = data.get("line_no").and_then(|v| v.as_u64()).unwrap_or(0);
                let line_text = data
                    .get("line_text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let preview = if line_text.len() > 50 {
                    format!("{}...", &line_text[..50])
                } else {
                    line_text
                };
                api.notify(
                    "info",
                    format!("Received line_changed event: line {} = '{}'", line_no, preview),
                )?;
            }
            "file_opened" => {
                let file_path = data
                    .get("file_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                api.notify("info", format!("File opened: {}", file_path))?;
            }
            "file_saved" => {
                let file_path = data
                    .get("file_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                api.notify("info", format!("File saved: {}", file_path))?;
            }
            "file_closed" => {
                let file_path = data
                    .get("file_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                api.notify("info", format!("File closed: {}", file_path))?;
            }
            "cursor_changed" => {
                let line_no = data.get("line_no").and_then(|v| v.as_u64()).unwrap_or(0);
                let column = data.get("column").and_then(|v| v.as_u64()).unwrap_or(0);
                #[cfg(debug_assertions)]
                {
                    let _ = api.notify(
                        "debug",
                        format!("Cursor moved to line {}, column {}", line_no, column),
                    );
                }
            }
            "selection_changed" => {
                let selected_text = data
                    .get("selected_text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if !selected_text.is_empty() {
                    let preview = if selected_text.len() > 30 {
                        format!("{}...", &selected_text[..30])
                    } else {
                        selected_text
                    };
                    api.notify("info", format!("Selection changed: '{}'", preview))?;
                }
            }
            _ => {
                api.notify("warning", format!("Received unknown event type: {}", event_type))?;
            }
        }

        api.send_ok(
            id,
            Some(serde_json::json!({
                "event_type": event_type,
                "handled": true
            })),
        )
    }
}

fn main() {
    let mut plugin = SimplePlugin;
    run_plugin(&mut plugin);
}

