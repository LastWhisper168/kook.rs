# KOOK SDK 使用指南

## 1. 环境准备

### 1.1 安装 Rust
确保您的系统已安装 Rust 1.70+ 版本。

### 1.2 创建新项目
```bash
cargo new my-kook-bot
cd my-kook-bot
```

### 1.3 添加依赖
在 `Cargo.toml` 中添加：
```toml
[dependencies]
kook_sdk = "0.1.0"
tokio = { version = "1", features = ["full"] }
```

## 2. 获取 Bot Token

1. 访问 [KOOK 开发者平台](https://developer.kookapp.cn/)
2. 创建应用程序
3. 在机器人页面获取 Bot Token
4. 设置环境变量或直接在代码中使用

## 3. 基础用法

### 3.1 创建客户端

```rust
use kook_sdk::KookClient;

// 方式1：从环境变量读取 token
let client = KookClient::new()?;

// 方式2：直接提供 token
let client = KookClient::with_token("your_bot_token")?;
```

### 3.2 获取机器人信息

```rust
let user = client.get_me().await?;
println!("机器人名称: {}", user.username);
println!("机器人ID: {}", user.id);
```

### 3.3 发送消息

```rust
// 发送文本消息
let result = client.send_message(
    "频道ID",
    "消息内容",
    Some(1), // 消息类型：1=文本
    None     // 引用消息ID（可选）
).await?;

// 发送 Markdown 消息
let result = client.send_message(
    "频道ID",
    "**粗体文本** 和 *斜体文本*",
    Some(9), // 消息类型：9=Markdown
    None
).await?;
```

### 3.4 获取频道列表

```rust
use kook_sdk::PageParams;

let params = PageParams {
    page: Some(1),
    page_size: Some(20),
    sort: None,
};

let channels = client.get_channels(&params).await?;
for channel in channels.items {
    println!("频道: {} (ID: {})", channel.name, channel.id);
}
```

## 4. WebSocket 实时事件

### 4.1 创建事件处理器

```rust
use kook_sdk::{EventHandler, EventData, HelloData};

struct MyBot;

impl EventHandler for MyBot {
    fn on_event(&self, event: EventData) -> impl std::future::Future<Output = ()> + Send {
        async move {
            match event.r#type {
                1 => {
                    // 文本消息
                    println!("收到消息: {}", event.content);
                    
                    // 可以在这里处理命令逻辑
                    if event.content.starts_with("!ping") {
                        // 回复消息的逻辑
                    }
                }
                _ => {
                    println!("其他事件类型: {}", event.r#type);
                }
            }
        }
    }
    
    fn on_hello(&self, hello: HelloData) -> impl std::future::Future<Output = ()> + Send {
        async move {
            println!("WebSocket 连接成功，会话ID: {:?}", hello.session_id);
        }
    }
}
```

### 4.2 启动 WebSocket 连接

```rust
use kook_sdk::WebSocketClient;

let client = KookClient::with_token("your_bot_token")?;
let ws_client = WebSocketClient::new(client);
let bot = MyBot;

// 这将阻塞并保持连接
ws_client.connect(bot).await?;
```

## 5. Webhook 服务器

### 5.1 创建 Webhook 处理器

```rust
use kook_sdk::{WebhookHandler, WebhookEvent, WebhookChallenge, KookError};

#[derive(Clone)]
struct MyWebhookHandler {
    verify_token: String,
}

impl WebhookHandler for MyWebhookHandler {
    fn handle_event(&self, event: WebhookEvent) -> impl std::future::Future<Output = Result<(), KookError>> + Send {
        async move {
            println!("Webhook 事件: sn={}, type={}", event.sn, event.d.r#type);
            
            // 处理事件逻辑
            match event.d.r#type {
                1 => {
                    // 处理文本消息
                    println!("消息内容: {}", event.d.content);
                }
                _ => {
                    println!("其他事件类型");
                }
            }
            
            Ok(())
        }
    }
    
    fn handle_challenge(&self, challenge: WebhookChallenge) -> impl std::future::Future<Output = Result<String, KookError>> + Send {
        let verify_token = self.verify_token.clone();
        async move {
            if challenge.verify_token == verify_token {
                Ok(challenge.challenge)
            } else {
                Err(KookError::Auth("验证令牌不匹配".to_string()))
            }
        }
    }
}
```

### 5.2 启动 Webhook 服务器

```rust
use kook_sdk::{start_webhook_server, WebhookConfig};

let config = WebhookConfig {
    verify_token: "你的验证令牌".to_string(),
    path: "webhook".to_string(),
    port: 3000,
    decompress: true,
};

let handler = MyWebhookHandler {
    verify_token: "你的验证令牌".to_string(),
};

// 启动服务器（阻塞）
start_webhook_server(config, handler).await?;
```

## 6. 完整示例

### 6.1 简单的回声机器人

```rust
use kook_sdk::*;

struct EchoBot {
    client: KookClient,
}

impl EventHandler for EchoBot {
    fn on_event(&self, event: EventData) -> impl std::future::Future<Output = ()> + Send {
        let client = self.client.clone();
        async move {
            if event.r#type == 1 && event.content.starts_with("!echo ") {
                let echo_text = &event.content[6..]; // 去掉 "!echo "
                
                if let Err(e) = client.send_message(
                    &event.target_id,
                    echo_text,
                    Some(1),
                    None
                ).await {
                    eprintln!("发送消息失败: {}", e);
                }
            }
        }
    }
    
    fn on_hello(&self, hello: HelloData) -> impl std::future::Future<Output = ()> + Send {
        async move {
            println!("回声机器人已启动");
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = KookClient::with_token("your_bot_token")?;
    let ws_client = WebSocketClient::new(client.clone());
    
    let bot = EchoBot { client };
    ws_client.connect(bot).await?;
    
    Ok(())
}
```

## 7. 错误处理

```rust
use kook_sdk::KookError;

match client.send_message("channel_id", "content", Some(1), None).await {
    Ok(result) => println!("消息发送成功: {:?}", result),
    Err(KookError::Network(e)) => println!("网络错误: {}", e),
    Err(KookError::Auth(e)) => println!("认证错误: {}", e),
    Err(KookError::Generic(code, msg)) => println!("API错误 {}: {}", code, msg),
    Err(e) => println!("其他错误: {:?}", e),
}
```

## 8. 最佳实践

### 8.1 环境变量配置
```bash
export KOOK_BOT_TOKEN="your_bot_token"
export RUST_LOG="info"
```

### 8.2 日志记录
```rust
use log::{info, error};

info!("机器人启动");
error!("发生错误: {}", error_msg);
```

### 8.3 优雅关闭
```rust
use tokio::signal;

tokio::select! {
    _ = ws_client.connect(handler) => {},
    _ = signal::ctrl_c() => {
        println!("收到关闭信号，正在退出...");
    }
}
```

## 9. 故障排除

### 9.1 常见错误

- **认证失败**: 检查 Bot Token 是否正确
- **网络超时**: 检查网络连接和防火墙设置
- **JSON 解析错误**: 检查 API 响应格式是否符合预期

### 9.2 调试技巧

启用详细日志：
```bash
RUST_LOG=debug cargo run
```

### 9.3 性能优化

- 使用连接池复用 HTTP 连接
- 实现消息队列避免频繁API调用
- 使用异步处理提高并发性能

## 10. 参考资料

- [KOOK 官方文档](https://developer.kookapp.cn/)
- [Rust 异步编程指南](https://rust-lang.github.io/async-book/)
- [tokio 官方文档](https://tokio.rs/)
