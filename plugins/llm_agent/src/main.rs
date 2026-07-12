use chrono::Local;
use plugin_sdk::{
    run_plugin, PluginApi, PluginEvent, PluginHandler, PluginMap, PluginMessage, PluginResponse,
};
use rig::client::{CompletionClient, ProviderClient};
use rig::completion::{Prompt, ToolDefinition};
use rig::providers::deepseek;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::sync::Semaphore;
use tokio::time;

const DEFAULT_PREAMBLE: &str = "你是一个专业的Markdown文档处理助手。";
const DEFAULT_PROMPT_FILE: &str = "llm_call_test_prompt.md";
const DESC_FILE: &str = "desc.json";
const DEFAULT_MAX_TURNS: usize = 8;
const DEFAULT_MAX_CONCURRENCY: usize = 2;

struct CommandConfig {
    preamble: String,
    prompt_file: String,
}

#[derive(Deserialize)]
struct TextStatsArgs {
    text: String,
}

#[derive(Debug, Deserialize)]
struct SetOutlineContentArgs {
    outline_path: String,
    content: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct TextStatsOutput {
    char_count: usize,
    line_count: usize,
    word_count: usize,
    non_whitespace_count: usize,
}

#[derive(Debug, thiserror::Error)]
#[error("text stats error")]
struct TextStatsError;

#[derive(Deserialize, Serialize)]
struct TextStatsTool;

impl Tool for TextStatsTool {
    const NAME: &'static str = "text_stats";
    type Error = TextStatsError;
    type Args = TextStatsArgs;
    type Output = TextStatsOutput;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "text_stats".to_string(),
            description: "Calculate text statistics from an input text".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "text": {"type": "string", "description": "text content to analyze"}
                },
                "required": ["text"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let text = args.text;
        Ok(TextStatsOutput {
            char_count: text.chars().count(),
            line_count: text.lines().count(),
            word_count: text.split_whitespace().count(),
            non_whitespace_count: text.chars().filter(|c| !c.is_whitespace()).count(),
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[error("set outline content error")]
struct SetOutlineContentError;

fn send_set_outline_content(
    api: &mut PluginApi,
    outline_path: &str,
    mut content: String,
) -> io::Result<()> {
    if !content.ends_with('\n') {
        content.push('\n');
    }
    let mut params = PluginMap::new();
    params.insert(
        "outline_path".to_string(),
        serde_json::Value::String(outline_path.to_string()),
    );
    params.insert("content".to_string(), serde_json::Value::String(content));
    api.send_command("set_outline_content", params)
}

#[derive(Deserialize, Serialize)]
struct SetOutlineContentTool;

impl Tool for SetOutlineContentTool {
    const NAME: &'static str = "set_outline_content";
    type Error = SetOutlineContentError;
    type Args = SetOutlineContentArgs;
    type Output = serde_json::Value;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "set_outline_content".to_string(),
            description: "Replace an outline section content in editor by outline path".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "outline_path": {
                        "type": "string",
                        "description": "Outline path, e.g. 一级标题 / 二级标题"
                    },
                    "content": {
                        "type": "string",
                        "description": "New markdown content for the matched outline section"
                    }
                },
                "required": ["outline_path", "content"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let mut api = PluginApi;
        send_set_outline_content(&mut api, &args.outline_path, args.content)
            .map_err(|_| SetOutlineContentError)?;

        Ok(serde_json::json!({
            "status": "sent",
            "command": "set_outline_content"
        }))
    }
}

struct LlmAgentPlugin {
    runtime: Runtime,
    exec_semaphore: Arc<Semaphore>,
}

impl LlmAgentPlugin {
    fn new() -> io::Result<Self> {
        let runtime = Runtime::new()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("create runtime failed: {e}")))?;
        let max_concurrency = Self::load_max_concurrency();
        let exec_semaphore = Arc::new(Semaphore::new(max_concurrency));
        Ok(Self {
            runtime,
            exec_semaphore,
        })
    }

    /// 从 `desc.json` 的 `config.max_concurrency`（或根级 `max_concurrency`）读取并发上限，至少为 1。
    fn load_max_concurrency() -> usize {
        let desc_path = Self::plugin_dir().join(DESC_FILE);
        let Ok(desc_text) = std::fs::read_to_string(desc_path) else {
            return DEFAULT_MAX_CONCURRENCY;
        };
        let Ok(parsed) = serde_json::from_str::<Value>(&desc_text) else {
            return DEFAULT_MAX_CONCURRENCY;
        };
        let n = parsed
            .get("config")
            .and_then(|c| c.get("max_concurrency"))
            .or_else(|| parsed.get("max_concurrency"))
            .and_then(|v| {
                v.as_u64()
                    .map(|u| u as usize)
                    .or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
            })
            .unwrap_or(DEFAULT_MAX_CONCURRENCY);
        n.max(1)
    }

    fn plugin_dir() -> PathBuf {
        let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
        exe.parent().unwrap_or(Path::new(".")).to_path_buf()
    }

    fn load_command_config(command: &str) -> Option<CommandConfig> {
        let desc_path = Self::plugin_dir().join(DESC_FILE);
        let desc_text = match std::fs::read_to_string(desc_path) {
            Ok(v) => v,
            Err(_) => return None,
        };

        let parsed: Value = match serde_json::from_str(&desc_text) {
            Ok(v) => v,
            Err(_) => return None,
        };

        let command_item = parsed
            .get("commands")
            .and_then(Value::as_array)
            .and_then(|commands| {
                commands.iter().find_map(|item| {
                    let cmd = item.get("command").and_then(Value::as_str)?;
                    if cmd != command {
                        return None;
                    }
                    Some(item)
                })
            });

        let Some(command_item) = command_item else {
            return None;
        };

        let preamble = command_item
            .get("preamble")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(DEFAULT_PREAMBLE)
            .to_string();

        let prompt_file = command_item
            .get("prompt_file")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(DEFAULT_PROMPT_FILE)
            .to_string();

        Some(CommandConfig {
            preamble,
            prompt_file,
        })
    }

    fn read_prompt_text(&self, prompt_file: &str) -> String {
        let path = Self::plugin_dir().join(prompt_file);
        std::fs::read_to_string(&path).unwrap_or_else(|_| "你好，这是一个测试prompt".to_string())
    }

    fn required_param<'a>(params: &'a PluginMap, key: &str) -> Result<&'a str, String> {
        params
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| format!("Missing param: {key}"))
    }

    fn build_user_query(&self, params: &PluginMap, prompt_file: &str) -> Result<String, String> {
        let outline_name = Self::required_param(params, "current_outline_name")?;
        let outline_path = Self::required_param(params, "current_outline_path")?;
        let outline_content = Self::required_param(params, "current_outline_content")?;
        let base_prompt = self.read_prompt_text(prompt_file);

        Ok(format!(
            "{base_prompt}\n\n【当前段落标题】\n- 名称: {outline_name}\n- 路径: {outline_path}\n\n【当前段落内容】\n{outline_content}"
        ))
    }

    async fn call_deepseek(user_query: String, preamble: String) -> Result<String, String> {
        let client = deepseek::Client::from_env();
        let agent = client
            .agent(deepseek::DEEPSEEK_CHAT)
            .preamble(&preamble)
            .default_max_turns(DEFAULT_MAX_TURNS)
            .tool(TextStatsTool)
            .tool(SetOutlineContentTool)
            .build();

        agent
            .prompt(&user_query)
            .await
            .map_err(|e| format!("LLM call failed: {e}"))
    }

    fn format_llm_error(err: &str) -> String {
        let lower = err.to_lowercase();
        if lower.contains("api key")
            || lower.contains("unauthorized")
            || lower.contains("authentication")
            || lower.contains("invalid key")
        {
            format!("{err}。请确认已设置 DeepSeek 环境变量（例如 DEEPSEEK_API_KEY）。")
        } else if lower.contains("maxturnerror") || lower.contains("max turn") {
            format!("{err}。可尝试提高 Agent 最大轮次配置。")
        } else {
            err.to_string()
        }
    }

    async fn run_llm_test_echo_async(
        outline_path: String,
        outline_content: String,
    ) -> Result<Value, io::Error> {
        time::sleep(Duration::from_secs(2)).await;
        let time_line = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let base = outline_content.trim_end();
        let new_content = if base.is_empty() {
            time_line.clone()
        } else {
            format!("{base}\n{time_line}")
        };
        let mut api = PluginApi;
        send_set_outline_content(&mut api, &outline_path, new_content)?;
        Ok(serde_json::json!({
            "command": "llm_test_echo",
            "appended_line": time_line,
        }))
    }

    fn send_stdout_message(msg: &PluginMessage) -> io::Result<()> {
        let line = serde_json::to_string(msg)?;
        println!("{line}");
        io::stdout().flush()
    }

    /// 若为 `llm_test_echo`：校验参数、异步执行并返回 `Ok(true)`；否则返回 `Ok(false)`。
    fn try_dispatch_llm_test_echo(
        &mut self,
        api: &mut PluginApi,
        id: String,
        command: &str,
        params: &PluginMap,
    ) -> io::Result<bool> {
        if command != "llm_test_echo" {
            return Ok(false);
        }

        let outline_path = match Self::required_param(params, "current_outline_path") {
            Ok(v) => v.to_string(),
            Err(e) => {
                api.send_err(id, e)?;
                return Ok(true);
            }
        };
        let outline_content = params
            .get("current_outline_content")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let request_id = id.clone();
        let sem = Arc::clone(&self.exec_semaphore);
        self.runtime.spawn(async move {
            let Ok(_permit) = sem.acquire_owned().await else {
                return;
            };
            match Self::run_llm_test_echo_async(outline_path, outline_content).await {
                Ok(data) => {
                    let _ = Self::send_stdout_message(&PluginMessage::Response(PluginResponse {
                        id: request_id,
                        success: true,
                        data: Some(data),
                        error: None,
                    }));
                }
                Err(e) => {
                    let _ = Self::send_stdout_message(&PluginMessage::Response(PluginResponse {
                        id: request_id,
                        success: false,
                        data: None,
                        error: Some(e.to_string()),
                    }));
                }
            }
        });
        Ok(true)
    }
}

impl PluginHandler for LlmAgentPlugin {
    fn on_init(&mut self, api: &mut PluginApi, id: String, config: PluginMap) -> io::Result<()> {
        api.send_ready(
            "LLM Agent Plugin",
            "0.1.0",
            vec![
                "llm_call_test".to_string(),
                "llm_call_coscat_jiangsu".to_string(),
                "llm_test_echo".to_string(),
            ],
        )?;
        api.send_ok(
            id,
            Some(serde_json::json!({
                "message": "llm_agent initialized",
                "provider": "deepseek",
                "model": "deepseek_chat",
                "config": config
            })),
        )
    }

    fn on_execute(
        &mut self,
        api: &mut PluginApi,
        id: String,
        command: String,
        params: PluginMap,
    ) -> io::Result<()> {
        if self.try_dispatch_llm_test_echo(api, id.clone(), command.as_str(), &params)? {
            return Ok(());
        }

        let command_cfg = match Self::load_command_config(command.as_str()) {
            Some(cfg) => cfg,
            None => {
                return api.send_err(
                    id,
                    format!("Unknown command: {command}. Please check {DESC_FILE}."),
                );
            }
        };

        let user_query = match self.build_user_query(&params, &command_cfg.prompt_file) {
            Ok(v) => v,
            Err(e) => return api.send_err(id, e),
        };

        let request_id = id.clone();
        let sem = Arc::clone(&self.exec_semaphore);
        self.runtime.spawn(async move {
            let Ok(_permit) = sem.acquire_owned().await else {
                return;
            };
            let llm_out = Self::call_deepseek(user_query, command_cfg.preamble.clone()).await;
            match llm_out {
                Ok(output) => {
                    let _ = Self::send_stdout_message(&PluginMessage::Event(PluginEvent::Notify {
                        level: "info".to_string(),
                        message: "LLM 调用成功".to_string(),
                    }));
                    let _ = Self::send_stdout_message(&PluginMessage::Response(PluginResponse {
                        id: request_id,
                        success: true,
                        data: Some(serde_json::json!({
                            "provider": "deepseek",
                            "model": "deepseek_chat",
                            "preamble": command_cfg.preamble,
                            "prompt_file": command_cfg.prompt_file,
                            "output": output
                        })),
                        error: None,
                    }));
                }
                Err(e) => {
                    let _ = Self::send_stdout_message(&PluginMessage::Response(PluginResponse {
                        id: request_id,
                        success: false,
                        data: None,
                        error: Some(Self::format_llm_error(&e)),
                    }));
                }
            }
        });
        Ok(())
    }

    fn on_shutdown(&mut self, api: &mut PluginApi, id: String) -> io::Result<()> {
        api.notify("info", "llm_agent is shutting down")?;
        api.send_ok(id, None)
    }
}

fn main() {
    let mut plugin = match LlmAgentPlugin::new() {
        Ok(plugin) => plugin,
        Err(e) => {
            eprintln!("failed to init llm_agent: {e}");
            return;
        }
    };
    run_plugin(&mut plugin);
}
