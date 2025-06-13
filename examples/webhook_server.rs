/// KOOK SDK Webhook 服务器示例
/// 
/// 本示例展示如何使用 KOOK SDK 创建 Webhook 服务器，实现：
/// - 接收 KOOK 平台推送的事件
/// - 验证 Webhook 签名
/// - 处理挑战验证
/// - 自定义事件处理逻辑

use kook_sdk::{
    WebhookConfig, WebhookHandler, WebhookEvent, WebhookChallenge, 
    KookError, EventData, start_webhook_server
};
use std::env;

/// 自定义 Webhook 事件处理器
#[derive(Clone)]
pub struct MyWebhookHandler {
    verify_token: String,
}

impl MyWebhookHandler {
    pub fn new(verify_token: String) -> Self {
        Self { verify_token }
    }
}

impl WebhookHandler for MyWebhookHandler {
    fn handle_event(
        &self,
        event: WebhookEvent,
    ) -> impl std::future::Future<Output = Result<(), KookError>> + Send {
        async move {
            println!("收到 Webhook 事件:");
            println!("  序列号: {}", event.sn);
            
            // 解析事件数据
            let event_data = &event.d;
            match event_data.r#type {
                1 => {
                    // 文本消息事件
                    println!("  类型: 文本消息");
                    println!("  发送者: {}", event_data.author_id);
                    println!("  频道: {}", event_data.target_id);
                    println!("  内容: {}", event_data.content);
                    println!("  消息ID: {}", event_data.msg_id);
                    println!("  时间戳: {}", event_data.msg_timestamp);
                    
                    // 这里可以添加自定义的消息处理逻辑
                    // 例如：关键词回复、命令处理、数据库存储等
                    if event_data.content.contains("帮助") {
                        println!("  检测到帮助请求，可以在这里实现自动回复");
                    }
                }
                2 => {
                    println!("  类型: 图片消息");
                    println!("  发送者: {}", event_data.author_id);
                    println!("  频道: {}", event_data.target_id);
                }
                8 => {
                    println!("  类型: 音频消息");
                    println!("  发送者: {}", event_data.author_id);
                    println!("  频道: {}", event_data.target_id);
                }
                9 => {
                    println!("  类型: 视频消息");
                    println!("  发送者: {}", event_data.author_id);
                    println!("  频道: {}", event_data.target_id);
                }
                255 => {
                    println!("  类型: 系统消息");
                    println!("  内容: {}", event_data.content);
                }
                _ => {
                    println!("  类型: 未知类型 ({})", event_data.r#type);
                    println!("  发送者: {}", event_data.author_id);
                    println!("  频道: {}", event_data.target_id);
                }
            }
            
            // 处理额外数据
            if !event_data.extra.is_null() {
                println!("  额外数据: {}", event_data.extra);
            }
            
            println!("  处理完成");
            println!("---");
            
            Ok(())
        }
    }

    fn handle_challenge(
        &self,
        challenge: WebhookChallenge,
    ) -> impl std::future::Future<Output = Result<String, KookError>> + Send {
        let verify_token = self.verify_token.clone();
        async move {
            println!("收到 Webhook 挑战验证:");
            println!("  挑战字符串: {}", challenge.challenge);
            println!("  验证令牌: {}", challenge.verify_token);
            
            // 验证令牌是否匹配
            if challenge.verify_token != verify_token {
                println!("  验证失败: 令牌不匹配");
                return Err(KookError::Auth("验证令牌不匹配".to_string()));
            }
            
            println!("  验证成功，返回挑战字符串");
            Ok(challenge.challenge)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志系统
    env_logger::init();

    println!("KOOK SDK Webhook 服务器示例");
    println!("===========================");
    println!("本示例将启动一个 HTTP 服务器来接收 KOOK Webhook 事件");
    println!();

    // 1. 从环境变量读取配置
    let verify_token = env::var("KOOK_VERIFY_TOKEN")
        .unwrap_or_else(|_| {
            println!("警告: 未设置 KOOK_VERIFY_TOKEN 环境变量，使用默认值");
            "default_verify_token".to_string()
        });

    let port = env::var("WEBHOOK_PORT")
        .unwrap_or_else(|_| "3030".to_string())
        .parse::<u16>()
        .unwrap_or(3030);

    let path = env::var("WEBHOOK_PATH")
        .unwrap_or_else(|_| "webhook".to_string());

    println!("服务器配置:");
    println!("  端口: {}", port);
    println!("  路径: /{}", path);
    println!("  验证令牌: {} (前4位)", &verify_token.chars().take(4).collect::<String>());
    println!();

    // 2. 创建 Webhook 配置
    let config = WebhookConfig {
        verify_token: verify_token.clone(),
        path,
        port,
        decompress: true, // 启用数据解压缩
    };

    // 3. 创建事件处理器
    let handler = MyWebhookHandler::new(verify_token);

    // 4. 启动服务器
    println!("正在启动 Webhook 服务器...");
    println!("服务器地址: http://127.0.0.1:{}/{}", config.port, config.path);
    println!("健康检查: http://127.0.0.1:{}/health", config.port);
    println!();
    println!("配置 KOOK 机器人 Webhook URL 为: http://您的服务器IP:{}/{}", config.port, config.path);
    println!("按 Ctrl+C 停止服务器");
    println!();

    // 启动服务器（这将阻塞直到服务器停止）
    start_webhook_server(config, handler).await?;

    println!("服务器已停止");
    Ok(())
}
