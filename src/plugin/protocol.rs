use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 插件消息类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PluginMessage {
    /// 请求消息（主程序 -> 插件）
    #[serde(rename = "request")]
    Request(PluginRequest),
    
    /// 响应消息（插件 -> 主程序）
    #[serde(rename = "response")]
    Response(PluginResponse),
    
    /// 事件消息（插件 -> 主程序）
    #[serde(rename = "event")]
    Event(PluginEvent),
    
    /// 错误消息
    #[serde(rename = "error")]
    Error(PluginError),
}

/// 插件请求类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum PluginRequest {
    /// 初始化插件
    #[serde(rename = "init")]
    Init {
        /// 请求ID
        id: String,
        /// 插件配置
        config: HashMap<String, serde_json::Value>,
    },
    
    /// 执行命令
    #[serde(rename = "execute")]
    Execute {
        /// 请求ID
        id: String,
        /// 命令名称
        command: String,
        /// 命令参数
        params: HashMap<String, serde_json::Value>,
    },
    
    /// 获取插件信息
    #[serde(rename = "info")]
    Info {
        /// 请求ID
        id: String,
    },
    
    /// 停止插件
    #[serde(rename = "shutdown")]
    Shutdown {
        /// 请求ID
        id: String,
    },
    
    /// 通知插件事件（主程序 -> 插件）
    #[serde(rename = "notify")]
    Notify {
        /// 请求ID
        id: String,
        /// 事件类型
        event_type: String,
        /// 事件数据
        data: HashMap<String, serde_json::Value>,
    },
}

/// 插件响应类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginResponse {
    /// 请求ID
    pub id: String,
    /// 是否成功
    pub success: bool,
    /// 响应数据
    pub data: Option<serde_json::Value>,
    /// 错误消息（如果失败）
    pub error: Option<String>,
}

/// 插件事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event")]
pub enum PluginEvent {
    /// 插件就绪
    #[serde(rename = "ready")]
    Ready {
        /// 插件名称
        name: String,
        /// 插件版本
        version: String,
        /// 插件能力
        capabilities: Vec<String>,
    },
    
    /// 通知消息
    #[serde(rename = "notify")]
    Notify {
        /// 通知类型
        level: String, // "info", "warning", "error"
        /// 通知消息
        message: String,
    },
    
    /// 命令请求（插件 -> 主程序）
    #[serde(rename = "command")]
    Command {
        /// 命令名称
        command: String,
        /// 命令参数
        params: HashMap<String, serde_json::Value>,
    },
    
    /// 数据更新
    #[serde(rename = "data")]
    Data {
        /// 数据类型
        data_type: String,
        /// 数据内容
        content: serde_json::Value,
    },
}

/// 插件错误
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginError {
    /// 错误代码
    pub code: String,
    /// 错误消息
    pub message: String,
    /// 错误详情
    pub details: Option<serde_json::Value>,
}

impl PluginMessage {
    /// 将消息序列化为单行JSON字符串
    pub fn to_json_line(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
    
    /// 从单行JSON字符串反序列化消息
    pub fn from_json_line(line: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(line)
    }
}

impl PluginResponse {
    /// 创建成功响应
    #[allow(dead_code)]
    pub fn success(id: String, data: Option<serde_json::Value>) -> Self {
        Self {
            id,
            success: true,
            data,
            error: None,
        }
    }
    
    /// 创建失败响应
    #[allow(dead_code)]
    pub fn error(id: String, error: String) -> Self {
        Self {
            id,
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

