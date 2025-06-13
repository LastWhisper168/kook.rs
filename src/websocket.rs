use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};
use serde_json::Value;
use std::time::Duration;
use tokio::time::{sleep, timeout, Instant};
use std::collections::HashMap;
use flate2::read::ZlibDecoder;
use std::io::Read;
use crate::models::*;
use crate::client::KookClient;

/// WebSocket 客户端，实现完整的 KOOK WebSocket 协议
pub struct KookWebSocketClient {
    client: KookClient,
    session_id: Option<String>,
    current_sn: i64,
    message_buffer: HashMap<i64, Signal>,
    compress: bool,
}

/// WebSocket 事件处理器
pub trait EventHandler: Send + Sync {
    fn on_event(&self, event: EventData) -> impl std::future::Future<Output = ()> + Send;
    fn on_hello(&self, hello: HelloData) -> impl std::future::Future<Output = ()> + Send;
    fn on_reconnect(&self, code: i32, message: String) -> impl std::future::Future<Output = ()> + Send;
    fn on_resume_ack(&self, session_id: String) -> impl std::future::Future<Output = ()> + Send;
}

impl KookWebSocketClient {
    /// 创建新的 WebSocket 客户端
    pub fn new(client: KookClient, compress: bool) -> Self {
        Self {
            client,
            session_id: None,
            current_sn: 0,
            message_buffer: HashMap::new(),
            compress,
        }
    }

    /// 启动 WebSocket 连接和事件循环
    pub async fn connect<H: EventHandler + 'static>(&mut self, handler: H) -> Result<(), KookError> {
        let mut retry_count = 0;
        let max_retries = 3;

        loop {
            match self.try_connect(&handler).await {
                Ok(_) => {
                    log::info!("WebSocket connection completed successfully");
                    return Ok(());
                }
                Err(e) => {
                    retry_count += 1;
                    if retry_count >= max_retries {
                        return Err(e);
                    }
                    
                    log::warn!("WebSocket connection failed (attempt {}): {}", retry_count, e);
                    let delay = 2_u64.pow(retry_count as u32);
                    sleep(Duration::from_secs(delay)).await;
                }
            }
        }
    }

    /// 尝试建立 WebSocket 连接
    async fn try_connect<H: EventHandler>(&mut self, handler: &H) -> Result<(), KookError> {
        // 1. 获取 Gateway
        log::info!("获取 WebSocket Gateway...");
        let gateway = self.client.get_gateway(self.compress).await?;
        
        // 2. 构建连接 URL
        let mut ws_url = format!("{}?token={}", gateway.url, self.client.get_token());
        if self.compress {
            ws_url.push_str("&compress=1");
        }

        // 3. 建立 WebSocket 连接
        log::info!("连接到 WebSocket: {}", ws_url);
        let (ws_stream, _) = connect_async(&ws_url).await
            .map_err(|e| KookError::WebSocket(format!("连接失败: {}", e)))?;

        let (write, mut read) = ws_stream.split();

        // 4. 等待 Hello 包
        let hello_timeout = timeout(Duration::from_secs(6), read.next()).await
            .map_err(|_| KookError::WebSocket("等待 Hello 包超时".to_string()))?;

        if let Some(Ok(msg)) = hello_timeout {
            let signal = self.parse_message(msg)?;
            if signal.s == 1 {
                let hello_data: HelloData = serde_json::from_value(signal.d)
                    .map_err(|e| KookError::Json(format!("解析 Hello 数据失败: {}", e)))?;
                
                if hello_data.code == 0 {
                    self.session_id = hello_data.session_id.clone();
                    log::info!("WebSocket 握手成功, session_id: {:?}", self.session_id);
                    handler.on_hello(hello_data).await;
                } else {
                    return Err(KookError::Auth(format!("WebSocket 握手失败: {}", hello_data.code)));
                }
            } else {
                return Err(KookError::WebSocket("期望 Hello 包但收到其他信令".to_string()));
            }
        } else {
            return Err(KookError::WebSocket("未收到 Hello 包".to_string()));
        }

        // 5. 启动心跳和消息处理
        self.start_event_loop(write, read, handler).await
    }

    /// 启动事件循环
    async fn start_event_loop<H: EventHandler>(
        &mut self,
        mut write: futures_util::stream::SplitSink<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>, Message>,
        mut read: futures_util::stream::SplitStream<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>,
        handler: &H,
    ) -> Result<(), KookError> {
        let mut last_ping = Instant::now();
        let ping_interval = Duration::from_secs(30);
        let pong_timeout = Duration::from_secs(6);
        let mut waiting_for_pong = false;

        loop {
            // 检查是否需要发送心跳
            if last_ping.elapsed() >= ping_interval && !waiting_for_pong {
                let ping_msg = serde_json::json!({
                    "s": 2,
                    "sn": self.current_sn
                });
                
                let ping_text = serde_json::to_string(&ping_msg)
                    .map_err(|e| KookError::Json(e.to_string()))?;
                
                write.send(Message::Text(ping_text)).await
                    .map_err(|e| KookError::WebSocket(format!("发送心跳失败: {}", e)))?;
                
                log::debug!("发送心跳: sn={}", self.current_sn);
                last_ping = Instant::now();
                waiting_for_pong = true;
            }

            // 等待消息，设置超时
            let message_timeout = if waiting_for_pong { pong_timeout } else { Duration::from_secs(1) };
            
            match timeout(message_timeout, read.next()).await {
                Ok(Some(Ok(msg))) => {
                    let signal = self.parse_message(msg)?;
                    
                    match signal.s {
                        0 => {
                            // 事件消息
                            if let Some(sn) = signal.sn {
                                self.handle_event_message(signal, sn, handler).await?;
                            }
                        }
                        3 => {
                            // Pong 心跳响应
                            log::debug!("收到心跳响应");
                            waiting_for_pong = false;
                        }
                        5 => {
                            // 重连请求
                            let reconnect_data: Value = signal.d;
                            let code = reconnect_data.get("code").and_then(|c| c.as_i64()).unwrap_or(0) as i32;
                            let message = reconnect_data.get("err").and_then(|m| m.as_str()).unwrap_or("Unknown").to_string();
                            
                            log::warn!("服务器要求重连: {} - {}", code, message);
                            handler.on_reconnect(code, message).await;
                            
                            // 清空状态并重连
                            self.session_id = None;
                            self.current_sn = 0;
                            self.message_buffer.clear();
                            return Err(KookError::WebSocket("服务器要求重连".to_string()));
                        }
                        6 => {
                            // Resume ACK
                            let ack_data: Value = signal.d;
                            if let Some(session_id) = ack_data.get("session_id").and_then(|s| s.as_str()) {
                                log::info!("Resume 成功: {}", session_id);
                                handler.on_resume_ack(session_id.to_string()).await;
                            }
                        }
                        _ => {
                            log::warn!("收到未知信令: {}", signal.s);
                        }
                    }
                }
                Ok(Some(Err(e))) => {
                    return Err(KookError::WebSocket(format!("WebSocket 错误: {}", e)));
                }
                Ok(None) => {
                    return Err(KookError::WebSocket("WebSocket 连接关闭".to_string()));
                }
                Err(_) => {
                    if waiting_for_pong {
                        log::warn!("心跳超时，准备重连");
                        return Err(KookError::WebSocket("心跳超时".to_string()));
                    }
                    // 普通超时，继续循环
                }
            }
        }
    }

    /// 处理事件消息，包括序列号管理
    async fn handle_event_message<H: EventHandler>(
        &mut self,
        signal: Signal,
        sn: i64,
        handler: &H,
    ) -> Result<(), KookError> {
        // 检查是否是重复消息
        if sn <= self.current_sn {
            log::debug!("丢弃重复消息: sn={}", sn);
            return Ok(());
        }

        // 检查是否是下一条消息
        if sn == self.current_sn + 1 {
            // 处理消息
            self.process_event_signal(signal, handler).await?;
            self.current_sn = sn;

            // 处理缓冲区中的后续消息
            while let Some(buffered_signal) = self.message_buffer.remove(&(self.current_sn + 1)) {
                self.process_event_signal(buffered_signal, handler).await?;
                self.current_sn += 1;
            }
        } else {
            // 消息乱序，放入缓冲区
            log::debug!("消息乱序，缓存: sn={}, 期望={}", sn, self.current_sn + 1);
            self.message_buffer.insert(sn, signal);
        }

        Ok(())
    }

    /// 处理具体的事件信令
    async fn process_event_signal<H: EventHandler>(
        &self,
        signal: Signal,
        handler: &H,
    ) -> Result<(), KookError> {
        let event_data: EventData = serde_json::from_value(signal.d)
            .map_err(|e| KookError::Json(format!("解析事件数据失败: {}", e)))?;
        
        handler.on_event(event_data).await;
        Ok(())
    }

    /// 解析 WebSocket 消息
    fn parse_message(&self, msg: Message) -> Result<Signal, KookError> {
        match msg {
            Message::Text(text) => {
                serde_json::from_str(&text)
                    .map_err(|e| KookError::Json(format!("解析文本消息失败: {}", e)))
            }
            Message::Binary(data) => {
                if self.compress {
                    // 解压缩数据
                    let mut decoder = ZlibDecoder::new(&data[..]);
                    let mut decompressed = String::new();
                    decoder.read_to_string(&mut decompressed)
                        .map_err(|e| KookError::WebSocket(format!("解压缩失败: {}", e)))?;
                    
                    serde_json::from_str(&decompressed)
                        .map_err(|e| KookError::Json(format!("解析压缩消息失败: {}", e)))
                } else {
                    // 直接解析二进制数据为 JSON
                    serde_json::from_slice(&data)
                        .map_err(|e| KookError::Json(format!("解析二进制消息失败: {}", e)))
                }
            }
            _ => Err(KookError::WebSocket("不支持的消息类型".to_string()))
        }
    }
}
