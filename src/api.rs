//! 可选：可为常用接口提供包装，如发送消息
use serde_json::json;
use crate::models::KookError;

impl crate::client::KookClient {
    /// 发送频道消息
    pub async fn send_channel_message(
        &self,
        channel_id: &str,
        content: &str,
    ) -> Result<crate::models::ApiResponse<serde_json::Value>, KookError> {
        let path = "/v3/message/create";
        let body = json!({
            "type": 1,
            "target_id": channel_id,
            "content": content
        });
        self.api_request(reqwest::Method::POST, &path, None, Some(&body)).await
    }
}
