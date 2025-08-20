use std::fmt::Display;

use base64_url::encode;
use mime::Mime;
use percent_encoding::{NON_ALPHANUMERIC, percent_encode};
pub use reqwest::Error;
use reqwest::{Client, header::CONTENT_TYPE};

/// Data URL 结构体，表示一个符合 RFC 2397 标准的数据 URL
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataUrl {
    /// 媒体类型 (MIME type)
    pub media_type: String,
    /// 是否是 base64 编码
    pub base64_encoded: bool,
    /// 数据内容
    pub data: Vec<u8>,
}

impl DataUrl {
    /// 创建一个新的 DataUrl
    pub fn new(media_type: impl Into<String>, data: Vec<u8>, base64_encoded: bool) -> Self {
        Self {
            media_type: media_type.into(),
            base64_encoded,
            data,
        }
    }
}

/// 将 DataUrl 转换为字符串表示形式
impl Display for DataUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let encoding = if self.base64_encoded { ";base64" } else { "" };
        let data = if self.base64_encoded {
            encode(&self.data)
        } else {
            // 对于非 base64 编码，需要确保数据是 URL 安全的
            percent_encode(&self.data, NON_ALPHANUMERIC).to_string()
        };
        write!(f, "data:{}{},{}", self.media_type, encoding, data)
    }
}

/// HTTP 到 Data URL 转换器
#[derive(Debug, Clone)]
pub struct GetDataUrl {
    client: Client,
}

impl Default for GetDataUrl {
    fn default() -> Self {
        Self::new()
    }
}

impl GetDataUrl {
    /// 创建一个新的转换器实例
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// 使用自定义 HTTP 客户端创建转换器实例
    pub fn with_client(client: Client) -> Self {
        Self { client }
    }

    /// 从 URL 获取资源并转换为 DataUrl
    pub async fn fetch(&self, url: &str) -> Result<DataUrl, reqwest::Error> {
        let response = self.client.get(url).send().await?;
        println!("{:?}", response);
        self.response_to_data_url(response).await
    }

    /// 将 HTTP 响应转换为 DataUrl
    pub async fn response_to_data_url(
        &self,
        response: reqwest::Response,
    ) -> Result<DataUrl, Error> {
        // 获取内容类型
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<Mime>().ok())
            .map(|mime| mime.to_string())
            .unwrap_or_else(|| "application/octet-stream".to_string());

        // 读取响应字节
        let bytes = response.bytes().await?.to_vec();

        // 创建 DataUrl (总是使用 base64 编码以确保数据安全)
        Ok(DataUrl::new(content_type, bytes, true))
    }
}

/// 便捷函数：从 URL 获取资源并转换为 Data URL 字符串
pub async fn url_to_data_url(url: &str) -> Result<String, Error> {
    let converter = GetDataUrl::new();
    let data_url = converter.fetch(url).await?;
    Ok(data_url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_data_url_creation() {
        let data = DataUrl::new("text/plain".to_string(), b"Hello, World!".to_vec(), true);

        assert_eq!(data.media_type, "text/plain");
        assert!(data.base64_encoded);
        assert_eq!(data.data, b"Hello, World!");

        let expected_string = "data:text/plain;base64,SGVsbG8sIFdvcmxkIQ";
        assert_eq!(data.to_string(), expected_string);
    }

    #[tokio::test]
    async fn test_fetch_data_url() {
        let mock_server = MockServer::start().await;

        // 设置模拟响应
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string("Hello, World!"))
            .mount(&mock_server)
            .await;

        let converter = GetDataUrl::new();
        let result = converter.fetch(&mock_server.uri()).await;

        assert!(result.is_ok());

        let data_url = result.unwrap();
        assert_eq!(data_url.media_type, "text/plain");
        assert!(data_url.base64_encoded);
        assert_eq!(data_url.data, b"Hello, World!");
    }

    #[tokio::test]
    async fn test_url_to_data_url_convenience() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(r#"{"message": "Hello, World!"}"#),
            )
            .mount(&mock_server)
            .await;

        let result = url_to_data_url(&mock_server.uri()).await;
        assert!(result.is_ok());

        let data_url_str = result.unwrap();
        println!("{}", data_url_str);
        assert!(data_url_str.starts_with("data:application/json;base64,"));
    }

    #[tokio::test]
    async fn test_invalid_url() {
        let converter = GetDataUrl::new();
        let result = converter.fetch("not_a_valid_url").await;
        assert!(result.is_err());
    }
}
