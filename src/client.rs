use reqwest::{Client, Method, header::HeaderMap};
use std::env;
use std::time::Duration;
use serde_json::Value;
use crate::models::*;

/// 核心客户端，管理 HTTP 客户端和 Bot Token
pub struct KookClient {
    client: Client,
    bot_token: String,
    base_url: String,
}

/// 分页参数
#[derive(Debug, Default)]
pub struct PageParams {
    pub page: Option<i32>,
    pub page_size: Option<i32>,
    pub sort: Option<String>,
}

impl KookClient {
    /// 使用指定的 token 创建客户端
    pub fn with_token(token: &str) -> Result<Self, KookError> {
        let mut headers = HeaderMap::new();
        headers.insert("User-Agent", "KookSDK/0.1.0".parse().unwrap());
        
        let client = Client::builder()
            .https_only(true)
            .timeout(Duration::from_secs(30))
            .default_headers(headers)
            .build()
            .map_err(|e| KookError::Network(e.to_string()))?;
            
        Ok(KookClient {
            client,
            bot_token: token.to_string(),
            base_url: "https://www.kookapp.cn/api".to_string(),
        })
    }

    /// 从环境变量初始化客户端
    pub fn new() -> Result<Self, KookError> {
        let bot_token = env::var("KOOK_BOT_TOKEN")
            .map_err(|_| KookError::Auth("KOOK_BOT_TOKEN environment variable not set".to_string()))?;
        Self::with_token(&bot_token)
    }

    /// 获取 Bot Token (用于 WebSocket 连接)
    pub fn get_token(&self) -> &str {
        &self.bot_token
    }

    /// 通用 API 请求方法，包含完整的错误处理
    pub async fn api_request<T: serde::de::DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        query: Option<&[(&str, &str)]>,
        body: Option<&Value>,
    ) -> Result<T, KookError> {
        let url = if path.starts_with('/') {
            format!("{}{}", self.base_url, path)
        } else {
            format!("{}/{}", self.base_url, path)
        };

        let mut req = self.client
            .request(method, &url)
            .header("Authorization", format!("Bot {}", self.bot_token))
            .header("Content-Type", "application/json");

        if let Some(q) = query {
            req = req.query(q);
        }
        
        if let Some(b) = body {
            req = req.json(b);
        }

        let resp = req.send().await
            .map_err(|e| KookError::Network(e.to_string()))?;

        let status = resp.status();
        let response_text = resp.text().await
            .map_err(|e| KookError::Network(e.to_string()))?;

        // 处理 HTTP 错误状态
        if !status.is_success() {
            return Err(KookError::Network(format!("HTTP {}: {}", status, response_text)));
        }

        // 解析 JSON 响应
        let api_resp: ApiResponse<T> = serde_json::from_str(&response_text)
            .map_err(|e| KookError::Json(format!("Failed to parse response: {} - Response: {}", e, response_text)))?;

        // 检查 API 错误码
        if api_resp.code != 0 {
            return Err(KookError::from_code(api_resp.code, api_resp.message));
        }

        // 返回数据
        api_resp.data.ok_or_else(|| KookError::Json("Response data is null".to_string()))
    }

    /// 分页请求方法
    pub async fn paged_request<T: serde::de::DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        params: &PageParams,
        extra_query: Option<&[(&str, &str)]>,
    ) -> Result<PagedResponse<T>, KookError> {
        let mut query_params = Vec::new();
        
        if let Some(page) = params.page {
            query_params.push(("page", page.to_string()));
        }
        if let Some(page_size) = params.page_size {
            query_params.push(("page_size", page_size.to_string()));
        }
        if let Some(sort) = &params.sort {
            query_params.push(("sort", sort.clone()));
        }
        
        if let Some(extra) = extra_query {
            for (k, v) in extra {
                query_params.push((k, v.to_string()));
            }
        }

        let query_refs: Vec<(&str, &str)> = query_params.iter()
            .map(|(k, v)| (k.as_ref(), v.as_ref()))
            .collect();

        self.api_request(method, path, Some(&query_refs), None).await
    }

    /// 获取当前用户信息
    pub async fn get_me(&self) -> Result<User, KookError> {
        self.api_request(Method::GET, "/v3/user/me", None, None).await
    }

    /// 获取 WebSocket Gateway
    pub async fn get_gateway(&self, compress: bool) -> Result<Gateway, KookError> {
        let compress_param = if compress { "1" } else { "0" };
        let query = [("compress", compress_param)];
        self.api_request(Method::GET, "/v3/gateway/index", Some(&query), None).await
    }

    /// 获取频道列表
    pub async fn get_channels(&self, params: &PageParams) -> Result<PagedResponse<Channel>, KookError> {
        self.paged_request(Method::GET, "/v3/channel/list", params, None).await
    }

    /// 获取服务器列表
    pub async fn get_guilds(&self, params: &PageParams) -> Result<PagedResponse<Guild>, KookError> {
        self.paged_request(Method::GET, "/v3/guild/list", params, None).await
    }

    /// 发送消息
    pub async fn send_message(
        &self,
        target_id: &str,
        content: &str,
        message_type: Option<i32>,
        quote: Option<&str>,
    ) -> Result<serde_json::Value, KookError> {
        let mut body = serde_json::json!({
            "target_id": target_id,
            "content": content,
        });

        if let Some(msg_type) = message_type {
            body["type"] = msg_type.into();
        }

        if let Some(quote_id) = quote {
            body["quote"] = quote_id.into();
        }

        self.api_request(Method::POST, "/v3/message/create", None, Some(&body)).await
    }
}
