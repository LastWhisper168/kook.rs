use serde::{Deserialize, Serialize};

/// API 通用响应结构 (按照官方文档规范)
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    /// 错误码，0代表成功，非0代表失败
    pub code: i32,
    /// 错误消息，会根据Accept-Language返回
    pub message: String,
    /// 具体的数据，可能为null
    pub data: Option<T>,
}

/// 分页信息
#[derive(Debug, Serialize, Deserialize)]
pub struct Meta {
    /// 页码
    pub page: i32,
    /// 总页数
    pub page_total: i32,
    /// 每一页的数据
    pub page_size: i32,
    /// 总数据量
    pub total: i32,
}

/// 分页列表响应
#[derive(Debug, Serialize, Deserialize)]
pub struct PagedResponse<T> {
    /// 数据列表
    pub items: Vec<T>,
    /// 分页信息
    pub meta: Meta,
    /// 排序信息
    pub sort: Option<serde_json::Value>,
}

/// 用户信息
#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub identify_num: String,
    pub online: bool,
    pub bot: bool,
    pub status: i32,
    pub avatar: String,
    pub vip_avatar: Option<String>,
    pub nickname: String,
    pub roles: Vec<i32>,
    pub is_vip: bool,
    pub vip_amp: bool,
    pub tag_info: Option<serde_json::Value>,
}

/// 服务器信息
#[derive(Debug, Serialize, Deserialize)]
pub struct Guild {
    pub id: String,
    pub name: String,
    pub topic: String,
    pub user_id: String,
    pub icon: String,
    pub notify_type: i32,
    pub region: String,
    pub enable_open: bool,
    pub open_id: String,
    pub default_channel_id: String,
    pub welcome_channel_id: String,
}

/// 频道信息
#[derive(Debug, Serialize, Deserialize)]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub user_id: String,
    pub guild_id: String,
    pub topic: String,
    pub is_category: bool,
    pub parent_id: String,
    pub level: i32,
    pub slow_mode: i32,
    pub r#type: i32,
    pub permission_overwrites: Vec<serde_json::Value>,
    pub permission_users: Vec<serde_json::Value>,
    pub permission_sync: i32,
    pub has_password: bool,
}

/// Gateway 响应
#[derive(Debug, Serialize, Deserialize)]
pub struct Gateway {
    pub url: String,
}

/// WebSocket 信令
#[derive(Debug, Serialize, Deserialize)]
pub struct Signal {
    /// 信令类型
    pub s: i32,
    /// 数据字段
    pub d: serde_json::Value,
    /// 序列号 (仅在s=0时存在)
    pub sn: Option<i64>,
}

/// WebSocket Hello 响应
#[derive(Debug, Serialize, Deserialize)]
pub struct HelloData {
    pub code: i32,
    pub session_id: Option<String>,
}

/// 事件数据
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EventData {
    pub channel_type: String,
    pub r#type: i32,
    pub target_id: String,
    pub author_id: String,
    pub content: String,
    pub msg_id: String,
    pub msg_timestamp: i64,
    pub nonce: String,
    pub extra: serde_json::Value,
}

/// KOOK 错误码枚举
#[derive(Debug)]
pub enum KookError {
    /// 通用错误
    Generic(i32, String),
    /// 网络错误
    Network(String),
    /// JSON 解析错误
    Json(String),
    /// WebSocket 错误
    WebSocket(String),
    /// 认证错误
    Auth(String),
    /// 参数错误
    Params(String),
}

impl std::fmt::Display for KookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KookError::Generic(code, msg) => write!(f, "KOOK API Error {}: {}", code, msg),
            KookError::Network(msg) => write!(f, "Network Error: {}", msg),
            KookError::Json(msg) => write!(f, "JSON Error: {}", msg),
            KookError::WebSocket(msg) => write!(f, "WebSocket Error: {}", msg),
            KookError::Auth(msg) => write!(f, "Auth Error: {}", msg),
            KookError::Params(msg) => write!(f, "Params Error: {}", msg),
        }
    }
}

impl std::error::Error for KookError {}

impl KookError {
    /// 根据错误码创建错误
    pub fn from_code(code: i32, message: String) -> Self {
        match code {
            40100..=40199 => KookError::Auth(format!("Authentication failed ({}): {}", code, message)),
            _ => KookError::Generic(code, message),
        }
    }
}
