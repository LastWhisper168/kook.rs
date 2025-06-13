# KOOK SDK for Rust

一个功能完整的 KOOK（原 Kaiheila）机器人开发框架，使用 Rust 语言编写。

## 项目简介

KOOK SDK for Rust 是一个高性能、类型安全的 KOOK 机器人开发库，支持 HTTP API、WebSocket 实时连接和 Webhook 事件处理。该库完全按照 KOOK 官方 API 文档规范实现，提供了完整的错误处理、自动重连、事件分发等功能。

## 主要特性

- **完整的 API 支持**: 支持 KOOK 所有官方 API 接口
- **WebSocket 实时连接**: 支持消息实时推送和事件处理
- **Webhook 服务器**: 内置 HTTP 服务器处理 Webhook 事件
- **类型安全**: 完整的类型定义，编译时错误检查
- **异步支持**: 基于 tokio 的高性能异步运行时
- **自动重连**: WebSocket 连接断开时自动重连
- **错误处理**: 完善的错误类型和处理机制
- **分页支持**: 内置分页参数和结果处理
- **压缩支持**: 支持 gzip/zlib 数据压缩

## 安装依赖

在您的 `Cargo.toml` 文件中添加以下依赖：

```toml
[dependencies]
kook_sdk = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
```

## 快速开始

### 1. 设置机器人令牌

```bash
# Windows (PowerShell)
$env:KOOK_BOT_TOKEN = "您的机器人令牌"

# Linux/macOS
export KOOK_BOT_TOKEN="您的机器人令牌"
```

### 2. 基本 API 调用

```rust
use kook_sdk::KookClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建客户端（自动从环境变量读取令牌）
    let client = KookClient::new()?;
    
    // 获取当前用户信息
    let user = client.get_me().await?;
    println!("机器人用户名: {}", user.username);
    
    // 发送消息
    client.send_message("频道ID", "你好，KOOK！", None, None).await?;
    
    Ok(())
}
```

### 3. WebSocket 实时连接

```rust
use kook_sdk::{KookClient, WebSocketClient, EventHandler, EventData, HelloData};

struct MyEventHandler;

impl EventHandler for MyEventHandler {
    fn on_event(&self, event: EventData) -> impl std::future::Future<Output = ()> + Send {
        async move {
            println!("收到消息: {}", event.content);
        }
    }

    fn on_hello(&self, hello: HelloData) -> impl std::future::Future<Output = ()> + Send {
        async move {
            println!("WebSocket 连接成功");
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = KookClient::new()?;
    let ws_client = WebSocketClient::new(client);
    let handler = MyEventHandler;
    
    // 启动 WebSocket 连接
    ws_client.connect(handler).await?;
    
    Ok(())
}
```

### 4. Webhook 服务器

```rust
use kook_sdk::{WebhookConfig, DefaultWebhookHandler, start_webhook_server};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = WebhookConfig {
        verify_token: "您的验证令牌".to_string(),
        path: "webhook".to_string(),
        port: 3000,
        decompress: true,
    };
    
    let handler = DefaultWebhookHandler::new(config.verify_token.clone());
    
    // 启动 Webhook 服务器
    start_webhook_server(config, handler).await?;
    
    Ok(())
}
```

## API 文档

### 客户端 (KookClient)

```rust
impl KookClient {
    // 创建客户端
    pub fn new() -> Result<Self, KookError>;
    pub fn with_token(token: &str) -> Result<Self, KookError>;
    
    // 用户相关
    pub async fn get_me(&self) -> Result<User, KookError>;
    
    // 消息相关
    pub async fn send_message(
        &self,
        target_id: &str,
        content: &str,
        message_type: Option<i32>,
        quote: Option<&str>,
    ) -> Result<serde_json::Value, KookError>;
    
    // 频道相关
    pub async fn get_channels(&self, params: &PageParams) -> Result<PagedResponse<Channel>, KookError>;
    
    // 服务器相关
    pub async fn get_guilds(&self, params: &PageParams) -> Result<PagedResponse<Guild>, KookError>;
    
    // WebSocket Gateway
    pub async fn get_gateway(&self, compress: bool) -> Result<Gateway, KookError>;
    
    // 通用 API 请求
    pub async fn api_request<T: serde::de::DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        query: Option<&[(&str, &str)]>,
        body: Option<&Value>,
    ) -> Result<T, KookError>;
}
```

### WebSocket 客户端 (WebSocketClient)

```rust
impl WebSocketClient {
    pub fn new(client: KookClient) -> Self;
    pub async fn connect<H: EventHandler>(&self, handler: H) -> Result<(), KookError>;
}
```

### Webhook 服务器

```rust
// 启动 Webhook 服务器
pub async fn start_webhook_server<H: WebhookHandler + Clone + 'static>(
    config: WebhookConfig,
    handler: H,
) -> Result<(), KookError>;
```

## 错误处理

库定义了统一的错误类型 `KookError`：

```rust
pub enum KookError {
    Generic(i32, String),     // API 错误码和消息
    Network(String),          // 网络连接错误
    Json(String),            // JSON 解析错误
    WebSocket(String),       // WebSocket 连接错误
    Auth(String),            // 认证错误
}
```

## 环境要求

- Rust 1.70 或更高版本
- tokio 异步运行时
- 稳定的网络连接

## 开发与贡献

### 构建项目

```bash
# 克隆项目
git clone https://github.com/your-username/kook-sdk-rust.git
cd kook-sdk-rust

# 构建项目
cargo build

# 运行测试
cargo test

# 构建文档
cargo doc --open
```

### 项目结构

```
src/
├── lib.rs          # 库入口文件
├── client.rs       # HTTP API 客户端
├── websocket.rs    # WebSocket 客户端
├── webhook.rs      # Webhook 服务器
├── models.rs       # 数据模型定义
├── api.rs          # API 接口封装
└── utils.rs        # 工具函数

examples/
├── basic_usage.rs      # 基本使用示例
├── websocket_bot.rs    # WebSocket 机器人示例
└── webhook_server.rs   # Webhook 服务器示例
```

## 许可证

本项目采用 MIT 许可证。详情请参阅 [LICENSE](LICENSE) 文件。

## 相关链接

- [KOOK 官方文档](https://developer.kookapp.cn/)
- [KOOK 社区](https://kook.vip/qenXk1)
- [GitHub 仓库](https://github.com/your-username/kook-sdk-rust)

## 版本历史

### v0.1.0
- 初始版本发布
- 支持完整的 KOOK API
- WebSocket 实时连接支持
- Webhook 服务器支持
- 完整的错误处理机制
