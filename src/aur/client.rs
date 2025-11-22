use crate::aur::package::{AurPackage, AurResponse};
use crate::error::{KhazaurError, Result};
use reqwest::Client;
use std::time::Duration;

const AUR_RPC_URL: &str = "https://aur.archlinux.org/rpc/v5";
const AUR_URL: &str = "https://aur.archlinux.org";

/// AUR RPC API client
pub struct AurClient {
    client: Client,
    rate_limiter: super::rate_limit::RateLimiter,
}

impl AurClient {
    /// Create a new AUR client
    pub fn new() -> Result<Self> {
        Self::with_rate_limit(10, 100) // Default: 10 concurrent, 100ms delay
    }

    /// Create AUR client with custom rate limiting
    pub fn with_rate_limit(max_concurrent: usize, delay_ms: u64) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(format!("khazaur/{}", env!("CARGO_PKG_VERSION")))
            .build()?;
        
        Ok(Self { 
            client,
            rate_limiter: super::rate_limit::RateLimiter::new(max_concurrent, delay_ms),
        })
    }

    /// Search for packages matching a query
    pub async fn search(&self, query: &str) -> Result<Vec<AurPackage>> {
        if query.len() < 2 {
            return Err(KhazaurError::AurApi(
                "Search query must be at least 2 characters".to_string(),
            ));
        }

        // Acquire rate limit
        let _guard = self.rate_limiter.acquire().await;

        let url = format!("{}/search/{}", AUR_RPC_URL, query);
        let retry_config = super::retry::RetryConfig::default();
        
        let response = super::retry::retry_request(
            || {
                let client = self.client.clone();
                let url = url.clone();
                async move {
                    client.get(&url).send().await
                }
            },
            &retry_config,
        )
        .await
        .map_err(|e| KhazaurError::AurApi(format!("Search failed after retries: {}", e)))?;

        let aur_response = response.json::<AurResponse>().await.map_err(|e| {
            KhazaurError::AurApi(format!("Failed to parse AUR response: {}", e))
        })?;

        if aur_response.is_error() {
            let error_msg = aur_response.error.unwrap_or_else(|| "Unknown error".to_string());
            return Err(KhazaurError::AurApi(format!("AUR search failed: {}", error_msg)));
        }

        Ok(aur_response.results)
    }

    /// Get information about a single package
    pub async fn info(&self, package_name: &str) -> Result<AurPackage> {
        // Acquire rate limit
        let _guard = self.rate_limiter.acquire().await;

        let url = format!("{}/info/{}", AUR_RPC_URL, package_name);
        let retry_config = super::retry::RetryConfig::default();
        
        let response = super::retry::retry_request(
            || {
                let client = self.client.clone();
                let url = url.clone();
                async move {
                    client.get(&url).send().await
                }
            },
            &retry_config,
        )
        .await
        .map_err(|e| KhazaurError::AurApi(format!("Info query failed after retries: {}", e)))?;

        let aur_response = response.json::<AurResponse>().await.map_err(|e| {
            KhazaurError::AurApi(format!("Failed to parse AUR response: {}", e))
        })?;

        if aur_response.is_error() {
            let error_msg = aur_response.error.unwrap_or_else(|| "Unknown error".to_string());
            return Err(KhazaurError::AurApi(format!("AUR info query failed: {}", error_msg)));
        }

        if aur_response.resultcount == 0 {
            return Err(KhazaurError::PackageNotFound(package_name.to_string()));
        }

        Ok(aur_response.first()?.clone())
    }

    /// Get information about multiple packages

    /// Get the snapshot URL for a package
    pub fn snapshot_url(&self, package_name: &str) -> String {
        format!("{}/cgit/aur.git/snapshot/{}.tar.gz", AUR_URL, package_name)
    }

    /// Download package bytes (tarball)
    pub async fn download_snapshot(&self, package_name: &str) -> Result<Vec<u8>> {
        let url = self.snapshot_url(package_name);
        
        let retry_config = super::retry::RetryConfig::default();
        
        let response = super::retry::retry_request(
            || {
                let client = self.client.clone();
                let url = url.clone();
                async move {
                    client.get(&url).send().await
                }
            },
            &retry_config,
        )
        .await
        .map_err(|e| KhazaurError::DownloadFailed(
            format!("Failed to download {} after retries: {}", package_name, e),
        ))?;

        if !response.status().is_success() {
            return Err(KhazaurError::DownloadFailed(
                format!("Failed to download {}: HTTP {}", package_name, response.status()),
            ));
        }

        let bytes = response.bytes().await?.to_vec();
        Ok(bytes)
    }
}

impl Default for AurClient {
    fn default() -> Self {
        Self::new().expect("Failed to create AUR client")
    }
}
