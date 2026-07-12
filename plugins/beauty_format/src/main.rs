use plugin_sdk::{run_plugin, PluginApi, PluginHandler, PluginMap};
use std::io;

fn format_json(text: &str) -> Result<String, String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err("未选中文本".into());
    }
    let value: serde_json::Value =
        serde_json::from_str(trimmed).map_err(|e| format!("不是有效的 JSON: {e}"))?;
    serde_json::to_string_pretty(&value).map_err(|e| format!("格式化失败: {e}"))
}

struct BeautyFormatPlugin;

impl PluginHandler for BeautyFormatPlugin {
    fn on_init(&mut self, api: &mut PluginApi, id: String, _config: PluginMap) -> io::Result<()> {
        api.send_ready("JSON Formatter", "0.1.0", vec!["format_json".to_string()])?;
        api.send_ok(id, None)
    }

    fn on_execute(
        &mut self,
        api: &mut PluginApi,
        id: String,
        command: String,
        params: PluginMap,
    ) -> io::Result<()> {
        if command != "format_json" {
            return api.send_err(id, format!("Unknown command: {command}"));
        }

        let selected = params
            .get("selected_text")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        match format_json(selected) {
            Ok(formatted) => {
                let mut cmd_params = PluginMap::new();
                cmd_params.insert(
                    "text".to_string(),
                    serde_json::Value::String(formatted),
                );
                api.send_command("insert_text", cmd_params)?;
                api.send_ok(id, Some(serde_json::json!({ "formatted": true })))
            }
            Err(e) => {
                let _ = api.notify("warn", &e);
                api.send_err(id, e)
            }
        }
    }
}

fn main() {
    run_plugin(&mut BeautyFormatPlugin);
}
