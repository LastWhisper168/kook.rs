use warp::Filter;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use flate2::read::ZlibDecoder;
use std::io::Read;
use crate::models::*;

/// Webhook 事件结构 (按照官方规范)
#[derive(Deserialize, Debug, Clone)]
pub struct WebhookEvent {
    /// 消息序列号
    pub sn: i64,
    /// 事件数据
    pub d: EventData,
}

/// Webhook 验证信息
#[derive(Deserialize, Debug)]
pub struct WebhookChallenge {
    pub challenge: String,
    pub verify_token: String,
}

/// Webhook 处理器特征
pub trait WebhookHandler: Send + Sync + Clone + 'static {
    fn handle_event(
        &self,
        event: WebhookEvent,
    ) -> impl std::future::Future<Output = Result<(), KookError>> + Send;
    
    fn handle_challenge(
        &self,
        challenge: WebhookChallenge,
    ) -> impl std::future::Future<Output = Result<String, KookError>> + Send;
}

/// 默认的 Webhook 处理器
#[derive(Clone)]
pub struct DefaultWebhookHandler {
    verify_token: String,
    processed_sns: Arc<RwLock<HashMap<i64, bool>>>,
}

impl DefaultWebhookHandler {
    pub fn new(verify_token: String) -> Self {
        Self {
            verify_token,
            processed_sns: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 检查是否为重复事件
    async fn is_duplicate(&self, sn: i64) -> bool {
        let sns = self.processed_sns.read().await;
        sns.contains_key(&sn)
    }

    /// 标记事件为已处理
    async fn mark_processed(&self, sn: i64) {
        let mut sns = self.processed_sns.write().await;
        sns.insert(sn, true);
        
        // 清理旧的序列号 (保留最近1000个)
        if sns.len() > 1000 {
            let min_sn = sn - 1000;
            sns.retain(|&k, _| k > min_sn);
        }
    }
}

impl WebhookHandler for DefaultWebhookHandler {
    fn handle_event(&self, event: WebhookEvent) -> impl std::future::Future<Output = Result<(), KookError>> + Send {
        let self_clone = self.clone();
        async move {
            // 检查重复事件
            if self_clone.is_duplicate(event.sn).await {
                log::debug!("丢弃重复事件: sn={}", event.sn);
                return Ok(());
            }

            // 处理事件 (这里只是打印，实际应用中应该实现具体逻辑)
            log::info!("处理 Webhook 事件: sn={}, type={}", event.sn, event.d.r#type);
            println!("事件内容: {:?}", event);

            // 标记为已处理
            self_clone.mark_processed(event.sn).await;
            Ok(())
        }
    }

    fn handle_challenge(&self, challenge: WebhookChallenge) -> impl std::future::Future<Output = Result<String, KookError>> + Send {
        let verify_token = self.verify_token.clone();
        async move {
            if challenge.verify_token != verify_token {
                return Err(KookError::Auth("验证令牌不匹配".to_string()));
            }
            Ok(challenge.challenge)
        }
    }
}

/// Webhook 服务器配置
pub struct WebhookConfig {
    pub verify_token: String,
    pub path: String,
    pub port: u16,
    pub decompress: bool,
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            verify_token: String::new(),
            path: "webhook".to_string(),
            port: 3000,
            decompress: true,
        }
    }
}

/// 启动 Webhook 服务器
pub async fn start_webhook_server<H: WebhookHandler + Clone + 'static>(
    config: WebhookConfig,
    handler: H,
) -> Result<(), KookError> {
    let handler_filter = warp::any().map(move || handler.clone());
    let decompress = config.decompress;
    let path = config.path.clone();
    let port = config.port;
    
    // 创建 Webhook 路由
    let webhook_route = warp::path(path)
        .and(warp::post())
        .and(warp::header::optional::<String>("content-encoding"))
        .and(warp::body::bytes())
        .and(handler_filter)
        .and_then(move |encoding: Option<String>, body: bytes::Bytes, handler: H| {
            handle_webhook_request(encoding, body, handler, decompress)
        });

    // 健康检查路由
    let health_route = warp::path("health")
        .and(warp::get())
        .map(|| Box::new(warp::reply::with_status("OK", warp::http::StatusCode::OK)) as Box<dyn warp::Reply>);

    let routes = webhook_route.or(health_route);

    log::info!("启动 Webhook 服务器: http://127.0.0.1:{}/{}", port, config.path);
    
    warp::serve(routes)
        .run(([127, 0, 0, 1], port))
        .await;

    Ok(())
}

/// 处理 Webhook 请求
async fn handle_webhook_request<H: WebhookHandler>(
    encoding: Option<String>,
    body: bytes::Bytes,
    handler: H,
    decompress: bool,
) -> Result<Box<dyn warp::Reply>, warp::Rejection> {
    // 解压缩数据 (如果需要)
    let json_str = if decompress && encoding.as_deref() == Some("gzip") {
        // 解压缩
        let mut decoder = ZlibDecoder::new(&body[..]);
        let mut decompressed = String::new();
        if let Err(e) = decoder.read_to_string(&mut decompressed) {
            log::error!("解压缩 Webhook 数据失败: {}", e);
            return Ok(Box::new(warp::reply::with_status(
                "Decompression failed",
                warp::http::StatusCode::BAD_REQUEST,
            )));
        }
        decompressed
    } else {
        match String::from_utf8(body.to_vec()) {
            Ok(s) => s,
            Err(e) => {
                log::error!("解析 Webhook 数据为 UTF-8 失败: {}", e);
                return Ok(Box::new(warp::reply::with_status(
                    "Invalid UTF-8",
                    warp::http::StatusCode::BAD_REQUEST,
                )));
            }
        }
    };

    // 首先尝试解析为挑战
    if let Ok(challenge) = serde_json::from_str::<WebhookChallenge>(&json_str) {
        match handler.handle_challenge(challenge).await {
            Ok(response) => {
                return Ok(Box::new(warp::reply::html(response)));
            }
            Err(e) => {
                log::error!("处理 Webhook 挑战失败: {}", e);
                return Ok(Box::new(warp::reply::with_status(
                    "Challenge failed",
                    warp::http::StatusCode::UNAUTHORIZED,
                )));
            }
        }
    }

    // 然后尝试解析为事件
    match serde_json::from_str::<WebhookEvent>(&json_str) {
        Ok(event) => {
            match handler.handle_event(event).await {
                Ok(_) => Ok(Box::new(warp::reply::with_status(
                    "OK",
                    warp::http::StatusCode::OK,
                ))),
                Err(e) => {
                    log::error!("处理 Webhook 事件失败: {}", e);
                    Ok(Box::new(warp::reply::with_status(
                        "Event processing failed",
                        warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                    )))
                }
            }
        }
        Err(e) => {
            log::error!("解析 Webhook JSON 失败: {}", e);
            log::debug!("原始数据: {}", json_str);
            Ok(Box::new(warp::reply::with_status(
                "Invalid JSON",
                warp::http::StatusCode::BAD_REQUEST,
            )))
        }
    }
}
