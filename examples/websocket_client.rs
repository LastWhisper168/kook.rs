/// KOOK SDK WebSocket 客户端示例
/// 
/// 本示例展示如何使用 KOOK SDK 建立 WebSocket 连接，实现：
/// - 实时接收消息和事件
/// - 自动心跳保持连接
/// - 断线自动重连
/// - 自定义事件处理

use kook_sdk::{KookClient, WebSocketClient, EventHandler, EventData, HelloData};
use std::sync::Arc;

/// 自定义事件处理器
#[derive(Clone)]
pub struct MyEventHandler {
    client: Arc<KookClient>,
}

impl MyEventHandler {
    pub fn new(client: KookClient) -> Self {
        Self {
            client: Arc::new(client),
        }
    }
}

impl EventHandler for MyEventHandler {
    fn on_event(&self, event: EventData) -> impl std::future::Future<Output = ()> + Send {
        let client = self.client.clone();
        async move {
            // 根据事件类型处理不同的消息
            match event.r#type {
                1 => {
                    // 文本消息
                    println!("收到文本消息:");
                    println!("  发送者: {}", event.author_id);
                    println!("  频道: {}", event.target_id);
                    println!("  内容: {}", event.content);
                    
                    // 如果消息内容是 "ping"，则回复 "pong"
                    if event.content.trim().to_lowercase() == "ping" {
                        println!("  检测到 ping 消息，回复 pong...");
                        if let Err(e) = client.send_message(
                            &event.target_id,
                            "pong",
                            Some(1), // 文本消息类型
                            Some(&event.msg_id), // 引用原消息
                        ).await {
                            eprintln!("  回复消息失败: {}", e);
                        } else {
                            println!("  回复成功");
                        }
                    }
                }
                2 => {
                    // 图片消息
                    println!("收到图片消息:");
                    println!("  发送者: {}", event.author_id);
                    println!("  频道: {}", event.target_id);
                }
                8 => {
                    // 音频消息
                    println!("收到音频消息:");
                    println!("  发送者: {}", event.author_id);
                    println!("  频道: {}", event.target_id);
                }
                9 => {
                    // 视频消息
                    println!("收到视频消息:");
                    println!("  发送者: {}", event.author_id);
                    println!("  频道: {}", event.target_id);
                }
                10 => {
                    // 文件消息
                    println!("收到文件消息:");
                    println!("  发送者: {}", event.author_id);
                    println!("  频道: {}", event.target_id);
                }
                _ => {
                    // 其他类型消息
                    println!("收到未知类型消息 (类型: {})", event.r#type);
                    println!("  发送者: {}", event.author_id);
                    println!("  频道: {}", event.target_id);
                }
            }
            
            println!("  消息ID: {}", event.msg_id);
            println!("  时间戳: {}", event.msg_timestamp);
            println!("---");
        }
    }

    fn on_hello(&self, hello: HelloData) -> impl std::future::Future<Output = ()> + Send {
        async move {
            println!("WebSocket 连接建立成功!");
            println!("  状态码: {}", hello.code);
            if let Some(session_id) = &hello.session_id {
                println!("  会话ID: {}", session_id);
            }
            println!("  开始监听事件...");
            println!("---");
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志系统
    env_logger::init();

    println!("KOOK SDK WebSocket 客户端示例");
    println!("=============================");
    println!("本示例将建立 WebSocket 连接并监听实时事件");
    println!("发送 'ping' 消息到任何频道，机器人将自动回复 'pong'");
    println!("按 Ctrl+C 退出程序");
    println!();

    // 1. 创建 HTTP 客户端
    println!("1. 创建 KOOK 客户端...");
    let client = KookClient::new()?;
    println!("   客户端创建成功");

    // 2. 验证客户端连接
    println!("\n2. 验证连接...");
    match client.get_me().await {
        Ok(user) => {
            println!("   机器人用户名: {}", user.username);
            println!("   机器人ID: {}", user.id);
        }
        Err(e) => {
            eprintln!("   连接验证失败: {}", e);
            return Err(e.into());
        }
    }

    // 3. 创建事件处理器
    println!("\n3. 创建事件处理器...");
    let event_handler = MyEventHandler::new(client.clone());

    // 4. 创建并启动 WebSocket 客户端
    println!("\n4. 启动 WebSocket 连接...");
    let ws_client = WebSocketClient::new(client);
    
    // 连接并开始监听（这将阻塞直到连接关闭）
    if let Err(e) = ws_client.connect(event_handler).await {
        eprintln!("WebSocket 连接失败: {}", e);
        return Err(e.into());
    }

    println!("WebSocket 连接已关闭");
    Ok(())
}
