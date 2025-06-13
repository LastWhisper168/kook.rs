/// 基本使用示例
/// 
/// 本示例展示 KOOK SDK 的基本使用方法，包括：
/// - 创建客户端
/// - 获取用户信息
/// - 发送消息
/// - 获取频道和服务器列表

use kook_sdk::{KookClient, PageParams};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志系统
    env_logger::init();
    
    println!("=== KOOK SDK 基本使用示例 ===");
    
    // 创建客户端（自动从环境变量读取 KOOK_BOT_TOKEN）
    println!("\n1. 创建客户端...");
    let client = KookClient::new()?;
    println!("客户端创建成功");
    
    // 获取机器人信息
    println!("\n2. 获取机器人信息...");
    let user = client.get_me().await?;
    println!("机器人用户名: {}", user.username);
    println!("机器人 ID: {}", user.id);
    println!("是否为机器人: {}", user.bot);
    println!("在线状态: {}", if user.online { "在线" } else { "离线" });
    
    // 获取 WebSocket Gateway
    println!("\n3. 获取 WebSocket Gateway...");
    match client.get_gateway(false).await {
        Ok(gateway) => {
            println!("Gateway URL: {}", gateway.url);
        }
        Err(e) => {
            println!("获取 Gateway 失败: {}", e);
        }
    }
    
    // 获取频道列表（分页）
    println!("\n4. 获取频道列表...");
    let page_params = PageParams {
        page: Some(1),
        page_size: Some(10),
        sort: None,
    };
    
    match client.get_channels(&page_params).await {
        Ok(channels) => {
            println!("频道列表获取成功:");
            println!("  总数: {}", channels.meta.total);
            println!("  当前页: {}/{}", channels.meta.page, channels.meta.page_total);
            println!("  页面大小: {}", channels.meta.page_size);
            println!("  本页频道数: {}", channels.items.len());
            
            for (i, channel) in channels.items.iter().take(5).enumerate() {
                println!("  {}. {} (ID: {})", i + 1, channel.name, channel.id);
            }
        }
        Err(e) => {
            println!("获取频道列表失败: {}", e);
            println!("这通常是因为机器人未加入任何服务器或没有权限");
        }
    }
    
    // 获取服务器列表
    println!("\n5. 获取服务器列表...");
    match client.get_guilds(&page_params).await {
        Ok(guilds) => {
            println!("服务器列表获取成功:");
            println!("  总数: {}", guilds.meta.total);
            println!("  本页服务器数: {}", guilds.items.len());
            
            for (i, guild) in guilds.items.iter().take(3).enumerate() {
                println!("  {}. {} (ID: {})", i + 1, guild.name, guild.id);
            }
        }
        Err(e) => {
            println!("获取服务器列表失败: {}", e);
            println!("这通常是因为机器人未加入任何服务器");
        }
    }
    
    // 发送消息示例 (需要有效的频道 ID)
    println!("\n6. 发送消息示例...");
    
    // 注意: 这里需要替换为实际的频道 ID
    // 在生产环境中，应该从频道列表中获取 ID
    let test_channel_id = "your_channel_id_here";
    
    if test_channel_id != "your_channel_id_here" {
        match client.send_message(
            test_channel_id,
            "你好！这是来自 KOOK Rust SDK 的测试消息。",
            None, // 消息类型 (None 表示普通文本消息)
            None, // 引用消息 ID
        ).await {
            Ok(_) => {
                println!("消息发送成功");
            }
            Err(e) => {
                println!("消息发送失败: {}", e);
            }
        }
    } else {
        println!("跳过消息发送 - 请设置有效的频道 ID");
        println!("在代码中将 'your_channel_id_here' 替换为实际的频道 ID");
    }
    
    println!("\n=== 基本使用示例完成 ===");
    println!("\n下一步:");
    println!("1. 尝试运行 WebSocket 机器人示例: cargo run --example websocket_bot");
    println!("2. 尝试运行 Webhook 服务器示例: cargo run --example webhook_server");
    println!("3. 查看 API 文档了解更多功能");
    
    Ok(())
}
