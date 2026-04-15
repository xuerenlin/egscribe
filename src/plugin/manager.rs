use crate::plugin::process::PluginProcess;
use crate::plugin::protocol::{PluginEvent, PluginMessage, PluginRequest};
use crate::medit::Action;
use crate::medit::Trigger;
use crate::i18n::{tr, current_language, Language};
use serde::{Deserialize, Serialize, Deserializer};
use serde_json;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc};
use regex::Regex;

/// 日志类别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogCategory {
    /// 接收（从插件接收的消息）
    Receive,
    /// 发送（向插件发送的消息）
    Send,
    /// 信息（一般信息）
    Info,
    /// 错误
    Error,
    /// 警告
    #[allow(dead_code)]
    Warning,
}

impl LogCategory {
    /// 获取类别的显示名称（带图标）
    pub fn as_str(&self) -> String {
        match self {
            LogCategory::Receive => format!("← {}", tr("plugin.log.receive")),
            LogCategory::Send => format!("→ {}", tr("plugin.log.send")),
            LogCategory::Info => format!("i {}", tr("plugin.log.info")),
            LogCategory::Error => format!("X {}", tr("plugin.log.error")),
            LogCategory::Warning => format!("! {}", tr("plugin.log.warning")),
        }
    }
}

/// 多语言文本
/// 支持两种格式：
/// 1. 简单字符串（向后兼容）
/// 2. 对象格式：{ "zh-CN": "...", "en-US": "..." }
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LocalizedString {
    /// 简单字符串格式
    Simple(String),
    /// 多语言对象格式
    Localized {
        #[serde(rename = "zh-CN")]
        zh_cn: Option<String>,
        #[serde(rename = "en-US")]
        en_us: Option<String>,
    },
}

impl LocalizedString {
    /// 根据当前语言获取文本
    pub fn get(&self) -> String {
        match self {
            LocalizedString::Simple(s) => s.clone(),
            LocalizedString::Localized { zh_cn, en_us } => {
                match current_language() {
                    Language::ZhCn => zh_cn.clone().or_else(|| en_us.clone()).unwrap_or_default(),
                    Language::EnUs => en_us.clone().or_else(|| zh_cn.clone()).unwrap_or_default(),
                }
            }
        }
    }
    
    /// 根据指定语言获取文本
    #[allow(dead_code)]
    pub fn get_with_lang(&self, lang: Language) -> String {
        match self {
            LocalizedString::Simple(s) => s.clone(),
            LocalizedString::Localized { zh_cn, en_us } => {
                match lang {
                    Language::ZhCn => zh_cn.clone().or_else(|| en_us.clone()).unwrap_or_default(),
                    Language::EnUs => en_us.clone().or_else(|| zh_cn.clone()).unwrap_or_default(),
                }
            }
        }
    }
}

/// 日志记录
#[derive(Debug, Clone)]
struct LogRecord {
    category: LogCategory,
    message: String,
    timestamp: SystemTime,
}

impl LogRecord {
    /// 格式化时间为字符串 (YYYY-MM-DD HH:MM:SS.mmm)
    fn format_time(&self) -> String {
        let datetime = DateTime::<Utc>::from(self.timestamp);
        datetime.format("%Y-%m-%d %H:%M:%S%.3f").to_string()
    }
}

/// 触发器配置
/// 支持两种格式：
/// 1. 简单字符串（向后兼容）："line_changed"
/// 2. 配置对象：{ "trigger": "line_changed", "pattern": "^=" }
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TriggerConfig {
    /// 简单字符串格式（向后兼容）
    Simple(String),
    /// 配置对象格式
    Config {
        /// 触发器类型
        trigger: String,
        /// 正则表达式模式（可选，仅对 line_changed 有效）
        #[serde(default)]
        pattern: Option<String>,
    },
}

impl TriggerConfig {
    /// 获取触发器类型
    pub fn get_trigger(&self) -> Option<Trigger> {
        let trigger_str = match self {
            TriggerConfig::Simple(s) => s,
            TriggerConfig::Config { trigger, .. } => trigger,
        };
        Trigger::from_event_type(trigger_str)
    }
    
    /// 获取正则表达式模式（仅对 line_changed 有效）
    pub fn get_pattern(&self) -> Option<String> {
        match self {
            TriggerConfig::Simple(_) => None,
            TriggerConfig::Config { pattern, .. } => pattern.clone(),
        }
    }
    
    /// 检查是否匹配（对于 line_changed，如果配置了 pattern，则检查文本是否匹配）
    pub fn matches(&self, trigger: &Trigger, text: Option<&str>) -> bool {
        // 首先检查触发器类型是否匹配
        if let Some(config_trigger) = self.get_trigger() {
            if config_trigger != *trigger {
                return false;
            }
        } else {
            return false;
        }
        
        // 如果是 line_changed 且有正则表达式模式，检查文本是否匹配
        if *trigger == Trigger::LineChanged {
            if let Some(pattern) = self.get_pattern() {
                if let Some(text) = text {
                    if let Ok(re) = Regex::new(&pattern) {
                        return re.is_match(text);
                    } else {
                        // 正则表达式编译失败，记录警告但不阻止通知
                        log::warn!("Invalid regex pattern in trigger config: {}", pattern);
                        return true; // 正则表达式无效时，默认发送通知
                    }
                } else {
                    // 没有文本，不匹配
                    return false;
                }
            }
        }
        
        // 没有正则表达式限制，或者不是 line_changed，直接匹配
        true
    }
}

/// 插件交互日志
#[derive(Debug, Clone)]
pub struct PluginInteractionLog {
    /// 交互记录列表（保留最近N条）
    records: Arc<Mutex<Vec<LogRecord>>>,
    /// 最大记录数
    max_records: usize,
}

impl PluginInteractionLog {
    pub fn new(max_records: usize) -> Self {
        Self {
            records: Arc::new(Mutex::new(Vec::new())),
            max_records,
        }
    }
    
    /// 添加一条交互记录
    pub fn add(&self, category: LogCategory, message: String) {
        let mut records = self.records.lock().unwrap();
        records.push(LogRecord { 
            category, 
            message,
            timestamp: SystemTime::now(),
        });
        if records.len() > self.max_records {
            records.remove(0);
        }
    }
    
    /// 获取所有交互记录（带类别和时间）
    #[allow(dead_code)]
    pub fn get_all(&self) -> Vec<String> {
        let records = self.records.lock().unwrap();
        records.iter().map(|record| {
            format!("{} [{}] {}", record.format_time(), record.category.as_str(), record.message)
        }).collect()
    }
}

/// 主进程可按文件类型路由的插件命令（desc.json 中 `commands` 数组元素）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginCommandDef {
    /// 与插件 `on_execute` 中的命令名一致
    pub command: String,
    /// 适用的文件扩展名（不含点，大小写不敏感）
    #[serde(default)]
    pub extensions: Vec<String>,
}

impl PluginCommandDef {
    /// `command` 一致且路径扩展名在 `extensions` 中（`extensions` 为空则不匹配任何文件）
    pub fn matches_file(&self, command: &str, file_path: &str) -> bool {
        if self.command != command {
            return false;
        }
        let Some(file_ext) = Path::new(file_path)
            .extension()
            .and_then(|e| e.to_str())
        else {
            return false;
        };
        let file_ext = file_ext.to_lowercase();
        if self.extensions.is_empty() {
            return false;
        }
        self.extensions.iter().any(|ext| {
            let n = ext.trim_start_matches('.').trim().to_lowercase();
            !n.is_empty() && n == file_ext
        })
    }
}

/// 插件配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// 插件ID
    pub id: String,
    /// 插件名称（支持多语言）
    #[serde(deserialize_with = "deserialize_localized_string")]
    pub name: LocalizedString,
    /// 插件功能说明（支持多语言）
    #[serde(default = "default_description", deserialize_with = "deserialize_localized_string_optional")]
    pub description: LocalizedString,
    /// 插件可执行文件路径
    pub exe_path: String,
    /// 插件启动参数
    pub args: Vec<String>,
    /// 插件配置参数
    pub config: HashMap<String, serde_json::Value>,
    /// 是否启用
    pub enabled: bool,
    /// 自动启动
    pub auto_start: bool,
    /// 触发器列表（当这些事件发生时，会通知插件）
    /// 支持简单字符串格式（向后兼容）和配置对象格式
    #[serde(default, deserialize_with = "deserialize_triggers")]
    pub triggers: Vec<TriggerConfig>,
    /// 主进程可主动调用的命令及适用扩展名（如非文本文件转换）
    #[serde(default)]
    pub commands: Vec<PluginCommandDef>,
}

/// 反序列化 LocalizedString（必需字段）
fn deserialize_localized_string<'de, D>(deserializer: D) -> Result<LocalizedString, D::Error>
where
    D: Deserializer<'de>,
{
    LocalizedString::deserialize(deserializer)
}

/// 反序列化 LocalizedString（可选字段，有默认值）
fn deserialize_localized_string_optional<'de, D>(deserializer: D) -> Result<LocalizedString, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<LocalizedString>::deserialize(deserializer)
        .map(|opt| opt.unwrap_or_else(|| LocalizedString::Simple(tr("plugin.config.default_description"))))
}

/// 反序列化触发器列表
/// 支持向后兼容：既支持简单的字符串数组，也支持配置对象数组
fn deserialize_triggers<'de, D>(deserializer: D) -> Result<Vec<TriggerConfig>, D::Error>
where
    D: Deserializer<'de>,
{
    // 首先尝试作为 TriggerConfig 数组反序列化
    let value: serde_json::Value = Deserialize::deserialize(deserializer)?;
    
    match value {
        serde_json::Value::Array(arr) => {
            let mut triggers = Vec::new();
            for item in arr {
                match item {
                    serde_json::Value::String(s) => {
                        // 简单字符串格式
                        triggers.push(TriggerConfig::Simple(s));
                    }
                    serde_json::Value::Object(obj) => {
                        // 配置对象格式
                        if let Ok(config) = TriggerConfig::deserialize(&serde_json::Value::Object(obj)) {
                            triggers.push(config);
                        }
                    }
                    _ => {
                        // 无效格式，跳过
                        continue;
                    }
                }
            }
            Ok(triggers)
        }
        _ => Ok(Vec::new()),
    }
}

/// 默认描述
fn default_description() -> LocalizedString {
    LocalizedString::Simple(tr("plugin.config.default_description"))
}

/// 插件状态
#[derive(Debug, Clone)]
pub enum PluginStatus {
    /// 未加载
    NotLoaded,
    /// 已加载
    #[allow(dead_code)]
    Loaded,
    /// 运行中
    Running,
    /// 错误
    #[allow(dead_code)]
    Error(String),
}

/// 插件实例
pub struct PluginInstance {
    /// 插件配置
    config: PluginConfig,
    /// 插件进程
    process: Option<PluginProcess>,
    /// 插件状态
    status: PluginStatus,
    /// 插件目录（用于解析相对路径）
    plugin_dir: PathBuf,
    /// 插件交互日志（记录所有交互信息）
    interaction_log: PluginInteractionLog,
}

impl PluginInstance {
    pub fn new(config: PluginConfig) -> Self {
        Self {
            config,
            process: None,
            status: PluginStatus::NotLoaded,
            plugin_dir: PathBuf::new(), // 将在启动时设置
            interaction_log: PluginInteractionLog::new(2048), 
        }
    }
    
    /// 获取交互日志
    pub fn interaction_log(&self) -> &PluginInteractionLog {
        &self.interaction_log
    }
    
    /// 设置插件目录
    pub fn set_plugin_dir(&mut self, plugin_dir: PathBuf) {
        self.plugin_dir = plugin_dir;
    }
    
    /// 启动插件
    pub fn start(&mut self) -> Result<(), String> {
        if matches!(self.status, PluginStatus::Running) {
            return Ok(());
        }
        
        // 解析可执行文件路径
        let exe_path = if Path::new(&self.config.exe_path).is_absolute() {
            // 绝对路径，直接使用
            PathBuf::from(&self.config.exe_path)
        } else {
            // 相对路径：尝试多种解析方式
            let mut path = None;
            let exe_path_clean = self.config.exe_path.trim_start_matches("./");
            let is_system_command = !exe_path_clean.contains('/') && !exe_path_clean.contains('\\');
            
            // 策略1: 如果路径包含 "plugins/"，尝试相对于可执行文件目录
            if exe_path_clean.starts_with("plugins/") {
                if let Ok(exe_dir) = std::env::current_exe() {
                    if let Some(exe_parent) = exe_dir.parent() {
                        let candidate = exe_parent.join(exe_path_clean);
                        if candidate.exists() {
                            path = Some(candidate);
                        }
                    }
                }
            }
            
            // 策略2: 如果路径只是文件名（不包含路径分隔符），尝试在插件目录中查找
            if path.is_none() && is_system_command {
                let candidate = self.plugin_dir.join(exe_path_clean);
                if candidate.exists() {
                    path = Some(candidate);
                }
            }
            
            // 策略3: 尝试相对于插件目录
            if path.is_none() {
                let candidate = self.plugin_dir.join(exe_path_clean);
                if candidate.exists() {
                    path = Some(candidate);
                }
            }
            
            // 策略4: 尝试相对于可执行文件目录
            if path.is_none() {
                if let Ok(exe_dir) = std::env::current_exe() {
                    if let Some(exe_parent) = exe_dir.parent() {
                        let candidate = exe_parent.join(exe_path_clean);
                        if candidate.exists() {
                            path = Some(candidate);
                        }
                    }
                }
            }
            
            // 策略5: 尝试相对于当前工作目录
            if path.is_none() {
                let candidate = PathBuf::from(exe_path_clean);
                if candidate.exists() {
                    path = Some(candidate.canonicalize().unwrap_or(candidate));
                }
            }
            
            // 策略6: 如果是系统命令（不包含路径分隔符），尝试在系统 PATH 中查找
            if path.is_none() && is_system_command {
                // 使用 which 命令查找（Unix/Linux）或 where 命令（Windows）
                #[cfg(unix)]
                {
                    if let Ok(output) = std::process::Command::new("which")
                        .arg(&exe_path_clean)
                        .output()
                    {
                        if output.status.success() {
                            if let Ok(found_path) = String::from_utf8(output.stdout) {
                                let found_path = found_path.trim();
                                if !found_path.is_empty() {
                                    path = Some(PathBuf::from(found_path));
                                }
                            }
                        }
                    }
                }
                
                #[cfg(windows)]
                {
                    if let Some(found) =
                        crate::util::win_exec::find_executable_in_path(exe_path_clean)
                    {
                        path = Some(found);
                    }
                }
            }
            
            // 如果仍然找不到，但对于系统命令，我们允许直接使用命令名
            // （让 std::process::Command 在执行时查找 PATH）
            if path.is_none() && is_system_command {
                // 对于系统命令，直接使用命令名，不进行文件存在性检查
                // std::process::Command 会在执行时查找 PATH
                PathBuf::from(&self.config.exe_path)
            } else {
                path.unwrap_or_else(|| PathBuf::from(&self.config.exe_path))
            }
        };
        
        // 对于系统命令（不包含路径分隔符），跳过文件存在性检查
        // 因为 std::process::Command 会在执行时查找 PATH
        let is_system_command = !exe_path.to_string_lossy().contains('/') 
            && !exe_path.to_string_lossy().contains('\\');
        
        if !is_system_command && !exe_path.exists() {
            return Err(format!("{}: {} (resolved paths: plugin_dir={:?}, exe_dir={:?})", 
                tr("plugin.error.executable_not_found"),
                self.config.exe_path, 
                self.plugin_dir,
                std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.to_path_buf()))
            ));
        }
        

        
        let exe_path_str = exe_path.to_string_lossy().to_string();
        let plugin_id = self.config.id.clone();
        
        self.interaction_log.add(LogCategory::Info, format!("Starting plugin '{}' from {}", plugin_id, exe_path_str));
        
        let mut process = PluginProcess::start(
            plugin_id.clone(),
            &exe_path_str,
            self.config.args.clone(),
            Some(self.plugin_dir.clone()),
        )?;
        
        // 发送初始化请求
        let init_id = format!("init_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos());
        let init_request = PluginRequest::Init {
            id: init_id.clone(),
            config: self.config.config.clone(),
        };
        
        self.interaction_log.add(LogCategory::Send, format!("Sending init request to plugin '{}'", plugin_id));
        
        let init_msg = PluginMessage::Request(init_request);
        let response = process.request(init_msg, Duration::from_secs(5))?;
        
        if !response.success {
            let error = response.error.unwrap_or_else(|| "Unknown error".to_string());
            self.interaction_log.add(LogCategory::Error, format!("Plugin '{}' init failed: {}", plugin_id, error));
            return Err(error);
        }
        
        self.interaction_log.add(LogCategory::Info, format!("Plugin '{}' initialized successfully", plugin_id));
        self.process = Some(process);
        self.status = PluginStatus::Running;
        
        Ok(())
    }
    
    /// 停止插件
    pub fn stop(&mut self) -> Result<(), String> {
        if let Some(ref mut process) = self.process {
            process.stop()?;
        }
        self.process = None;
        self.status = PluginStatus::NotLoaded;
        Ok(())
    }
    
    /// 处理插件消息
    pub fn handle_messages(&mut self) -> Vec<Action> {
        let mut commands = Vec::new();
        
        if let Some(ref mut process) = self.process {
            // 处理所有待处理的消息
            while let Ok(msg) = process.try_recv() {
                let log = format!("Receive Plugin Message: {:?}", msg);
                self.interaction_log.add(LogCategory::Receive, log);
                match msg {
                    PluginMessage::Event(event) => {
                        match event {
                            PluginEvent::Command { command, params } => {
                                // 将插件命令转换为应用命令
                                let cmd = Self::convert_plugin_command(&command, &params);
                                if let Some(cmd) = cmd {
                                    commands.push(cmd);
                                }
                            }
                            PluginEvent::Notify { level: _, message: _ } => {
                            }
                            PluginEvent::Ready { name: _, version: _, capabilities: _ } => {
                            }
                            PluginEvent::Data { data_type: _, content: _ } => {
                            }
                        }
                    }
                    PluginMessage::Response(_resp) => {
                    }
                    PluginMessage::Error(err) => {
                        self.status = PluginStatus::Error(err.message.clone());
                    }
                    PluginMessage::Request(_) => {
                        // 插件不应该发送请求
                        log::warn!("Plugin {} sent unexpected request", self.config.id);
                    }
                }
            }
            
            // 检查进程状态
            if !process.is_running() {
                self.status = PluginStatus::Error("Process exited".to_string());
                self.process = None;
            }
        }
        
        commands
    }
    
    /// 转换插件命令为应用命令
    fn convert_plugin_command(command: &str, params: &HashMap<String, serde_json::Value>) -> Option<Action> {
        // 使用 Action::from_command 动态创建 Action（带验证）
        match Action::from_command(command, params) {
            Ok(action) => Some(action),
            Err(e) => {
                log::warn!("Failed to convert plugin command '{}': {}", command, e);
                None
            }
        }
    }
    
    /// 执行插件命令
    #[allow(dead_code)]
    pub fn execute(&mut self, command: String, params: HashMap<String, serde_json::Value>) -> Result<serde_json::Value, String> {
        if let Some(ref mut process) = self.process {
            let request_id = format!("exec_{}", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos());
            
            self.interaction_log.add(LogCategory::Send, format!("Execute: {} ({:?})", command, params));
            
            let request = PluginRequest::Execute {
                id: request_id.clone(),
                command: command.clone(),
                params: params.clone(),
            };
            
            let request_msg = PluginMessage::Request(request);
            let response = process.request(request_msg, Duration::from_secs(10))?;
            
            if response.success {
                let result = response.data.unwrap_or(serde_json::Value::Null);
                self.interaction_log.add(LogCategory::Receive, format!("Execute result: {} -> Success ({:?})", command, result));
                Ok(result)
            } else {
                let error = response.error.unwrap_or_else(|| "Unknown error".to_string());
                self.interaction_log.add(LogCategory::Error, format!("Execute result: {} -> Error: {}", command, error));
                Err(error)
            }
        } else {
            let error = tr("plugin.error.not_running");
            self.interaction_log.add(LogCategory::Error, format!("Execute failed: {} - {}", command, error));
            Err(error)
        }
    }
    
    /// 获取插件状态
    #[allow(dead_code)]
    pub fn status(&self) -> &PluginStatus {
        &self.status
    }
    
    /// 获取插件配置
    #[allow(dead_code)]
    pub fn config(&self) -> &PluginConfig {
        &self.config
    }
    
    /// 通知插件事件（如果插件配置了相应的触发器）
    #[allow(dead_code)]
    pub fn notify_event(&mut self, trigger: Trigger, event_data: HashMap<String, serde_json::Value>) -> Result<(), String> {
        // 检查插件是否配置了该触发器，并检查正则表达式匹配
        let mut should_notify = false;
        let mut matched_config: Option<&TriggerConfig> = None;
        
        for trigger_config in &self.config.triggers {
            // 对于 line_changed，需要检查正则表达式匹配
            if trigger == Trigger::LineChanged {
                if let Some(line_text) = event_data.get("line_text").and_then(|v| v.as_str()) {
                    if trigger_config.matches(&trigger, Some(line_text)) {
                        should_notify = true;
                        matched_config = Some(trigger_config);
                        break;
                    }
                } else {
                    // 没有 line_text，不发送通知
                    continue;
                }
            } else {
                // 其他触发器，只检查类型匹配
                if trigger_config.matches(&trigger, None) {
                    should_notify = true;
                    matched_config = Some(trigger_config);
                    break;
                }
            }
        }
        
        if !should_notify {
            return Ok(()); // 插件未配置此触发器或正则表达式不匹配，直接返回
        }
        
        // 检查插件是否正在运行
        if let Some(ref mut process) = self.process {
            let event_type = trigger.to_event_type();
            
            let request_id = format!("notify_{}_{}", event_type, std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos());
            
            let notify_request = PluginRequest::Notify {
                id: request_id.clone(),
                event_type: event_type.to_string(),
                data: event_data,
            };
            
            let notify_msg = PluginMessage::Request(notify_request);
            
            // 发送通知（不等待响应）
            if let Err(e) = process.send(notify_msg.clone()) {
                self.interaction_log.add(LogCategory::Error, format!("Failed to notify event {}: {}", event_type, e));
                return Err(e);
            }
            
            // 记录日志，如果使用了正则表达式，也记录一下
            let log_msg = if let Some(config) = matched_config {
                if let Some(pattern) = config.get_pattern() {
                    format!("Send notify event: {} (pattern: {}), {:?}", event_type, pattern, notify_msg)
                } else {
                    format!("Send notify event: {}, {:?}", event_type, notify_msg)
                }
            } else {
                format!("Send notify event: {}, {:?}", event_type, notify_msg)
            };
            self.interaction_log.add(LogCategory::Send, log_msg);
            Ok(())
        } else {
            // 插件未运行，忽略通知
            Ok(())
        }
    }
}

/// 插件操作类型
#[derive(Debug, Clone)]
pub enum PluginAction {
    /// 启动或停止插件 (plugin_id, start)
    StartStop(String, bool),
    /// 显示插件日志 (plugin_id)
    ShowLog(String),
}

/// 插件管理器
pub struct PluginManager {
    /// 插件实例
    plugins: HashMap<String, PluginInstance>,
    /// 插件目录
    plugin_dir: PathBuf,
}

impl PluginManager {
    /// 创建插件管理器
    pub fn new(plugin_dir: PathBuf) -> Self {
        Self {
            plugins: HashMap::new(),
            plugin_dir,
        }
    }
    
    /// 加载插件配置
    pub fn load_plugins(&mut self) -> Result<(), String> {
        // 扫描插件目录下的所有子目录
        let entries = std::fs::read_dir(&self.plugin_dir)
            .map_err(|e| format!("{}: {}", tr("plugin.error.read_directory"), e))?;
        
        for entry in entries {
            let entry = entry.map_err(|e| format!("{}: {}", tr("plugin.error.read_entry"), e))?;
            let path = entry.path();
            
            // 只处理目录
            if !path.is_dir() {
                continue;
            }
            
            // 在每个插件目录中查找 desc.json
            let plugin_config_file = path.join("desc.json");
            
            if !plugin_config_file.exists() {
                // 如果不存在 desc.json，跳过该插件
                continue;
            }
            
            // 读取并解析插件配置
            let json = std::fs::read_to_string(&plugin_config_file)
                .map_err(|e| format!("{} {:?}: {}", tr("plugin.error.read_config"), plugin_config_file, e))?;
            
            // 解析为插件配置对象
            let config: PluginConfig = serde_json::from_str(&json)
                .map_err(|e| format!("{} {:?}: {}", tr("plugin.error.parse_config"), plugin_config_file, e))?;
            
            println!("{:?}", config);
            
            if config.enabled {
                let mut instance = PluginInstance::new(config.clone());
                // 设置插件目录为各自的插件目录，用于解析相对路径
                instance.set_plugin_dir(path.clone());
                
                if config.auto_start {
                    if let Err(e) = instance.start() {
                        log::error!("Failed to start plugin {}: {}", config.id, e);
                    }
                }
                
                self.plugins.insert(config.id.clone(), instance);
            }
        }
        
        Ok(())
    }
    
    /// 处理所有插件的消息
    pub fn handle_messages(&mut self) -> Vec<Action> {
        let mut all_commands = Vec::new();
        
        for (_plugin_id, plugin) in self.plugins.iter_mut() {
            let commands = plugin.handle_messages();
            all_commands.extend(commands);
        }
        
        all_commands
    }
    
    /// 启动插件
    pub fn start_plugin(&mut self, plugin_id: &str) -> Result<(), String> {
        if let Some(plugin) = self.plugins.get_mut(plugin_id) {
            plugin.start()
        } else {
            Err(format!("{}: {}", tr("plugin.error.not_found"), plugin_id))
        }
    }
    
    /// 停止插件
    pub fn stop_plugin(&mut self, plugin_id: &str) -> Result<(), String> {
        if let Some(plugin) = self.plugins.get_mut(plugin_id) {
            plugin.stop()
        } else {
            Err(format!("{}: {}", tr("plugin.error.not_found"), plugin_id))
        }
    }
    
    /// 执行插件命令
    #[allow(dead_code)]
    pub fn execute_plugin_command(&mut self, plugin_id: &str, command: String, params: HashMap<String, serde_json::Value>) -> Result<serde_json::Value, String> {
        if let Some(plugin) = self.plugins.get_mut(plugin_id) {
            plugin.execute(command, params)
        } else {
            Err(format!("{}: {}", tr("plugin.error.not_found"), plugin_id))
        }
    }

    /// 根据 `desc.json` 中的 `commands` 声明，查找能处理该路径的插件 id
    pub fn find_plugin_for_command_and_file(&self, command: &str, file_path: &str) -> Option<String> {
        for (plugin_id, instance) in &self.plugins {
            for def in &instance.config().commands {
                if def.matches_file(command, file_path) {
                    return Some(plugin_id.clone());
                }
            }
        }
        None
    }

    /// 解析插件、必要时启动进程后执行命令（用于 `hex_file_to_md` 等按扩展名路由的场景）
    pub fn execute_file_command_with_auto_start(
        &mut self,
        command: &str,
        file_path: &str,
        params: HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        let plugin_id = self.find_plugin_for_command_and_file(command, file_path).ok_or_else(|| {
            format!(
                "未找到可处理该文件的插件：命令「{}」，请在对应插件 desc.json 的 commands 中声明扩展名",
                command
            )
        })?;

        let need_start = self
            .plugins
            .get(&plugin_id)
            .map(|p| !matches!(p.status(), PluginStatus::Running))
            .unwrap_or(true);

        if need_start {
            self.start_plugin(&plugin_id)?;
        }

        self.execute_plugin_command(&plugin_id, command.to_string(), params)
    }
    
    /// 获取所有插件
    #[allow(dead_code)]
    pub fn plugins(&self) -> &HashMap<String, PluginInstance> {
        &self.plugins
    }
    
    /// 获取插件目录
    #[allow(dead_code)]
    pub fn plugin_dir(&self) -> &Path {
        &self.plugin_dir
    }
    
    /// 获取插件的所有日志记录
    pub fn get_plugin_logs(&self, plugin_id: &str) -> Option<Vec<String>> {
        self.plugins.get(plugin_id)
            .map(|plugin| plugin.interaction_log().get_all())
    }
    
    /// 通知所有配置了相应触发器的插件
    #[allow(dead_code)]
    pub fn notify_event(&mut self, trigger: Trigger, event_data: HashMap<String, serde_json::Value>) {
        for (_plugin_id, plugin) in self.plugins.iter_mut() {
            if let Err(e) = plugin.notify_event(trigger.clone(), event_data.clone()) {
                log::warn!("Failed to notify plugin {}: {}", plugin.config().id, e);
            }
        }
    }
    
    /// 显示插件卡片内容
    fn show_plugin_card_content(
        ui: &mut eframe::egui::Ui,
        plugin_id: &str,
        config: &PluginConfig,
        status: &PluginStatus,
        actions: &mut Vec<PluginAction>,
    ) {
        use eframe::egui::Color32;
        
        // 插件名称（根据当前语言显示）
        let name = config.name.get();
        ui.label(
            eframe::egui::RichText::new(&name)
                .size(16.0)
                .strong()
        );
        
        ui.add_space(5.0);
        
        // 功能说明（只显示一行，超长时显示省略号）
        let description = config.description.get();
        let description = if description.is_empty() {
            tr("plugin.config.default_description")
        } else {
            description
        };
        
        // 使用 Label 的 truncate 功能实现单行显示
        let label = eframe::egui::Label::new(&description).truncate();
        ui.add(label);
        
        // 状态显示
        let (status_text, status_color) = match status {
            PluginStatus::NotLoaded => (tr("plugin.status.not_loaded"), Color32::GRAY),
            PluginStatus::Loaded => (tr("plugin.status.loaded"), Color32::from_rgb(200, 200, 0)),
            PluginStatus::Running => (tr("plugin.status.running"), Color32::from_rgb(0, 200, 0)),
            PluginStatus::Error(e) => {
                ui.horizontal(|ui| {
                    ui.label(tr("plugin.status.label"));
                    ui.colored_label(Color32::RED, format!("{}: {}", tr("plugin.status.error"), e));
                });
                return;
            }
        };
        
        ui.horizontal(|ui| {
            ui.label(tr("plugin.status.label"));
            ui.colored_label(status_color, &status_text);
            match status {
                PluginStatus::Running => {
                    if ui.button(tr("plugin.action.stop")).clicked() {
                        actions.push(PluginAction::StartStop(plugin_id.to_string(), false));
                    }
                }
                _ => {
                    if ui.button(tr("plugin.action.start")).clicked() {
                        actions.push(PluginAction::StartStop(plugin_id.to_string(), true));
                    }
                }
            }
            if ui.button(tr("plugin.action.log")).clicked() {
                actions.push(PluginAction::ShowLog(plugin_id.to_string()));
            }
        });
    }
    
    /// 显示单个插件卡片
    fn show_plugin_card(
        ui: &mut eframe::egui::Ui,
        plugin_id: &str,
        config: &PluginConfig,
        status: &PluginStatus,
        available_width: f32,
        actions: &mut Vec<PluginAction>,
    ) {
        use eframe::egui::{Frame, Id};
        
        // 从 memory 中读取上一帧的悬停状态
        let hover_id = Id::new(("plugin_card_hover", plugin_id));
        let is_hovered = ui.memory(|mem| {
            mem.data.get_temp::<bool>(hover_id).unwrap_or(false)
        });
        
        // 根据悬停状态设置填充颜色
        let fill_color = if is_hovered {
            ui.style().visuals.faint_bg_color
        } else {
            ui.style().visuals.panel_fill
        };
        
        // 插件卡片框架
        let frame = Frame::default()
            .fill(fill_color)
            .corner_radius(3.0)
            .inner_margin(5.0);
        
        // 使用 allocate_ui 分配固定宽度，使所有插件卡片宽度一致
        ui.allocate_ui_with_layout(
            eframe::egui::Vec2::new(available_width, 0.0),
            eframe::egui::Layout::top_down(eframe::egui::Align::Min),
            |ui| {
                // 设置宽度为可用宽度，确保填充整个区域
                ui.set_width(available_width);
                
                // 显示 Frame 并获取 response 来检测悬停
                let frame_response = frame.show(ui, |ui| {
                    ui.vertical(|ui| {
                        // 设置内部宽度也为可用宽度（减去内边距）
                        ui.set_width(available_width - 10.0); // 减去左右内边距
                        
                        // 显示插件卡片内容
                        Self::show_plugin_card_content(ui, plugin_id, config, status, actions);
                    });
                });
                
                // 检测鼠标悬停并保存到 memory 中（供下一帧使用）
                // 使用 contains_pointer 来检测，这样在点击时也能正确检测到
                let is_hovered_now = frame_response.response.hovered() 
                    || frame_response.response.contains_pointer();
                ui.memory_mut(|mem| {
                    mem.data.insert_temp(hover_id, is_hovered_now);
                });
            });
    }
    
    /// 显示插件管理 UI
    pub fn show_ui(&mut self, ui: &mut eframe::egui::Ui) -> Vec<PluginAction> {
        let mut actions = Vec::new();
        
        if self.plugins.is_empty() {
            ui.label(tr("plugin.manager.no_plugins"));
            return actions;
        }
        
        // 获取可用宽度，使插件卡片填充整个界面
        let available_width = ui.available_width();
        
        // 按插件 ID 排序显示
        let mut plugin_ids: Vec<String> = self.plugins.keys().cloned().collect();
        plugin_ids.sort();
        
        for plugin_id in plugin_ids {
            if let Some(plugin) = self.plugins.get(&plugin_id) {
                let config = plugin.config();
                let status = plugin.status();
                Self::show_plugin_card(ui, &plugin_id, config, status, available_width, &mut actions);
            }
        }
        
        actions
    }
}

impl Drop for PluginManager {
    fn drop(&mut self) {
        // 停止所有插件
        for (_, plugin) in self.plugins.iter_mut() {
            let _ = plugin.stop();
        }
    }
}

