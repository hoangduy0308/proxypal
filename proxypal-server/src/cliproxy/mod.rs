pub mod config_gen;
pub mod process;

pub use config_gen::{
    build_proxy_config_yaml, generate_proxy_config, load_server_config, save_server_config,
    RateLimits, ServerConfig,
};
pub use process::{LocalProxyProcessManager, MockProxyProcessManager, ProxyProcessManager};

use async_trait::async_trait;
use axum::body::Bytes;
use http::{HeaderMap, Method};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyProviderStatus {
    pub name: String,
    pub status: String,
    pub accounts_count: i64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyHealthResponse {
    pub running: bool,
    pub uptime_seconds: Option<u64>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthStartResponse {
    pub auth_url: String,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthStatusResponse {
    pub completed: bool,
}

#[derive(Debug, Clone)]
pub struct ProxyResponse {
    pub status: u16,
    pub headers: HeaderMap,
    pub body: Bytes,
}

#[async_trait]
pub trait ProxyManagementClient: Send + Sync {
    async fn health_check(&self) -> anyhow::Result<ProxyHealthResponse>;
    async fn list_provider_statuses(&self) -> anyhow::Result<Vec<ProxyProviderStatus>>;
    async fn get_provider_status(&self, provider: &str) -> anyhow::Result<ProxyProviderStatus>;
    async fn start_oauth(&self, provider: &str, is_webui: bool) -> anyhow::Result<(String, String)>;
    async fn check_oauth_status(&self, state: &str) -> anyhow::Result<bool>;
    async fn sync_provider(&self, provider: &str) -> anyhow::Result<()>;
    async fn remove_provider(&self, provider: &str) -> anyhow::Result<()>;
    async fn forward_request(
        &self,
        path: &str,
        method: Method,
        headers: HeaderMap,
        body: Bytes,
    ) -> anyhow::Result<ProxyResponse>;
}

pub struct HttpProxyManagementClient {
    base_url: String,
    management_key: String,
    client: reqwest::Client,
}

impl HttpProxyManagementClient {
    pub fn new(base_url: String, management_key: String) -> Self {
        Self {
            base_url,
            management_key,
            client: reqwest::Client::new(),
        }
    }

    pub fn from_env() -> anyhow::Result<Self> {
        let base_url = std::env::var("PROXY_MANAGEMENT_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8317".to_string());
        let management_key = std::env::var("MANAGEMENT_KEY")
            .unwrap_or_else(|_| "proxypal-mgmt-key".to_string());
        Ok(Self::new(base_url, management_key))
    }

    fn auth_header(&self) -> (&'static str, &str) {
        ("X-Management-Key", &self.management_key)
    }
}

#[async_trait]
impl ProxyManagementClient for HttpProxyManagementClient {
    async fn health_check(&self) -> anyhow::Result<ProxyHealthResponse> {
        let url = format!("{}/v0/management/health", self.base_url);
        let (header_name, header_value) = self.auth_header();
        let resp = self
            .client
            .get(&url)
            .header(header_name, header_value)
            .send()
            .await?
            .error_for_status()?
            .json::<ProxyHealthResponse>()
            .await?;
        Ok(resp)
    }

    async fn list_provider_statuses(&self) -> anyhow::Result<Vec<ProxyProviderStatus>> {
        let url = format!("{}/v0/management/providers", self.base_url);
        let (header_name, header_value) = self.auth_header();
        let resp = self
            .client
            .get(&url)
            .header(header_name, header_value)
            .send()
            .await?
            .error_for_status()?
            .json::<Vec<ProxyProviderStatus>>()
            .await?;
        Ok(resp)
    }

    async fn get_provider_status(&self, provider: &str) -> anyhow::Result<ProxyProviderStatus> {
        let url = format!("{}/v0/management/providers/{}", self.base_url, provider);
        let (header_name, header_value) = self.auth_header();
        let resp = self
            .client
            .get(&url)
            .header(header_name, header_value)
            .send()
            .await?
            .error_for_status()?
            .json::<ProxyProviderStatus>()
            .await?;
        Ok(resp)
    }

    async fn start_oauth(&self, provider: &str, is_webui: bool) -> anyhow::Result<(String, String)> {
        let url = format!(
            "{}/v0/management/{}-auth-url?is_webui={}",
            self.base_url, provider, is_webui
        );
        let (header_name, header_value) = self.auth_header();
        let resp = self
            .client
            .get(&url)
            .header(header_name, header_value)
            .send()
            .await?
            .error_for_status()?
            .json::<OAuthStartResponse>()
            .await?;
        Ok((resp.auth_url, resp.state))
    }

    async fn check_oauth_status(&self, state: &str) -> anyhow::Result<bool> {
        let url = format!(
            "{}/v0/management/get-auth-status?state={}",
            self.base_url, state
        );
        let (header_name, header_value) = self.auth_header();
        let resp = self
            .client
            .get(&url)
            .header(header_name, header_value)
            .send()
            .await?
            .error_for_status()?
            .json::<AuthStatusResponse>()
            .await?;
        Ok(resp.completed)
    }

    async fn sync_provider(&self, _provider: &str) -> anyhow::Result<()> {
        let url = format!("{}/v0/management/reload", self.base_url);
        let (header_name, header_value) = self.auth_header();
        self.client
            .post(&url)
            .header(header_name, header_value)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    async fn remove_provider(&self, provider: &str) -> anyhow::Result<()> {
        let url = format!(
            "{}/v0/management/auth-files?provider={}",
            self.base_url, provider
        );
        let (header_name, header_value) = self.auth_header();
        self.client
            .delete(&url)
            .header(header_name, header_value)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    async fn forward_request(
        &self,
        path: &str,
        method: Method,
        headers: HeaderMap,
        body: Bytes,
    ) -> anyhow::Result<ProxyResponse> {
        let url = format!("{}{}", self.base_url, path);
        
        let mut req = self.client.request(
            reqwest::Method::from_bytes(method.as_str().as_bytes())?,
            &url,
        );
        
        for (key, value) in headers.iter() {
            if key != http::header::HOST && key != http::header::CONNECTION {
                req = req.header(key.as_str(), value.to_str().unwrap_or(""));
            }
        }
        
        req = req.body(body);
        
        let resp = req.send().await?;
        let status = resp.status().as_u16();
        let resp_headers = resp.headers().clone();
        let resp_body = resp.bytes().await?;
        
        let mut header_map = HeaderMap::new();
        for (key, value) in resp_headers.iter() {
            if let Ok(name) = http::header::HeaderName::from_bytes(key.as_str().as_bytes()) {
                if let Ok(val) = http::header::HeaderValue::from_bytes(value.as_bytes()) {
                    header_map.insert(name, val);
                }
            }
        }
        
        Ok(ProxyResponse {
            status,
            headers: header_map,
            body: resp_body,
        })
    }
}

#[derive(Default)]
pub struct MockProxyManagementClient {
    pub health_response: std::sync::Mutex<Option<ProxyHealthResponse>>,
    pub provider_statuses: std::sync::Mutex<Vec<ProxyProviderStatus>>,
    pub oauth_start_response: std::sync::Mutex<Option<(String, String)>>,
    pub oauth_status: std::sync::Mutex<bool>,
    pub call_log: std::sync::Mutex<Vec<String>>,
    pub forward_response: std::sync::Mutex<Option<ProxyResponse>>,
}

impl MockProxyManagementClient {
    fn log_call(&self, call: &str) {
        self.call_log.lock().unwrap().push(call.to_string());
    }
}

#[async_trait]
impl ProxyManagementClient for MockProxyManagementClient {
    async fn health_check(&self) -> anyhow::Result<ProxyHealthResponse> {
        self.log_call("health_check");
        self.health_response
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| anyhow::anyhow!("No mock health response configured"))
    }

    async fn list_provider_statuses(&self) -> anyhow::Result<Vec<ProxyProviderStatus>> {
        self.log_call("list_provider_statuses");
        Ok(self.provider_statuses.lock().unwrap().clone())
    }

    async fn get_provider_status(&self, provider: &str) -> anyhow::Result<ProxyProviderStatus> {
        self.log_call(&format!("get_provider_status:{}", provider));
        self.provider_statuses
            .lock()
            .unwrap()
            .iter()
            .find(|s| s.name == provider)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Provider not found: {}", provider))
    }

    async fn start_oauth(&self, provider: &str, is_webui: bool) -> anyhow::Result<(String, String)> {
        self.log_call(&format!("start_oauth:{}:{}", provider, is_webui));
        self.oauth_start_response
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| anyhow::anyhow!("No mock OAuth response configured"))
    }

    async fn check_oauth_status(&self, state: &str) -> anyhow::Result<bool> {
        self.log_call(&format!("check_oauth_status:{}", state));
        Ok(*self.oauth_status.lock().unwrap())
    }

    async fn sync_provider(&self, provider: &str) -> anyhow::Result<()> {
        self.log_call(&format!("sync_provider:{}", provider));
        Ok(())
    }

    async fn remove_provider(&self, provider: &str) -> anyhow::Result<()> {
        self.log_call(&format!("remove_provider:{}", provider));
        Ok(())
    }

    async fn forward_request(
        &self,
        path: &str,
        method: Method,
        _headers: HeaderMap,
        _body: Bytes,
    ) -> anyhow::Result<ProxyResponse> {
        self.log_call(&format!("forward_request:{}:{}", method, path));
        self.forward_response
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| anyhow::anyhow!("No mock forward response configured"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_client_records_calls() {
        let mock = MockProxyManagementClient::default();
        *mock.health_response.lock().unwrap() = Some(ProxyHealthResponse {
            running: true,
            uptime_seconds: Some(120),
            version: Some("1.0.0".to_string()),
        });
        *mock.oauth_start_response.lock().unwrap() =
            Some(("https://auth.example.com".to_string(), "state123".to_string()));

        let _ = mock.health_check().await;
        let _ = mock.list_provider_statuses().await;
        let _ = mock.start_oauth("google", true).await;
        let _ = mock.check_oauth_status("state123").await;
        let _ = mock.sync_provider("google").await;
        let _ = mock.remove_provider("google").await;

        let calls = mock.call_log.lock().unwrap().clone();
        assert_eq!(
            calls,
            vec![
                "health_check",
                "list_provider_statuses",
                "start_oauth:google:true",
                "check_oauth_status:state123",
                "sync_provider:google",
                "remove_provider:google",
            ]
        );
    }

    #[test]
    fn http_client_formats_request_correctly() {
        let client = HttpProxyManagementClient::new(
            "http://localhost:8317".to_string(),
            "test-key".to_string(),
        );

        assert_eq!(client.base_url, "http://localhost:8317");
        assert_eq!(client.management_key, "test-key");

        let (header_name, header_value) = client.auth_header();
        assert_eq!(header_name, "X-Management-Key");
        assert_eq!(header_value, "test-key");
    }

    #[test]
    fn from_env_uses_defaults() {
        std::env::remove_var("PROXY_MANAGEMENT_URL");
        std::env::remove_var("MANAGEMENT_KEY");

        let client = HttpProxyManagementClient::from_env().unwrap();
        assert_eq!(client.base_url, "http://127.0.0.1:8317");
        assert_eq!(client.management_key, "proxypal-mgmt-key");
    }

    #[tokio::test]
    async fn mock_get_provider_status_returns_matching_provider() {
        let mock = MockProxyManagementClient::default();
        *mock.provider_statuses.lock().unwrap() = vec![
            ProxyProviderStatus {
                name: "google".to_string(),
                status: "healthy".to_string(),
                accounts_count: 2,
                last_error: None,
            },
            ProxyProviderStatus {
                name: "azure".to_string(),
                status: "unhealthy".to_string(),
                accounts_count: 0,
                last_error: Some("Auth failed".to_string()),
            },
        ];

        let status = mock.get_provider_status("google").await.unwrap();
        assert_eq!(status.name, "google");
        assert_eq!(status.status, "healthy");
        assert_eq!(status.accounts_count, 2);

        let status = mock.get_provider_status("azure").await.unwrap();
        assert_eq!(status.status, "unhealthy");
        assert!(status.last_error.is_some());

        let result = mock.get_provider_status("nonexistent").await;
        assert!(result.is_err());
    }
}
