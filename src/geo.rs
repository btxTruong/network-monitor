//! Geo-IP location module using ip-api.com
//! Fetches public IP address and geographic location information.

use serde::Deserialize;
use thiserror::Error;

// Note: ip-api.com free tier only supports HTTP. HTTPS requires paid API key.
// This is acceptable as we only fetch public IP metadata (no sensitive data).
const API_URL: &str = "http://ip-api.com/json/?fields=status,message,country,countryCode,city,isp,query";

/// Geographic location information from IP lookup
#[derive(Debug, Clone, Deserialize)]
pub struct GeoInfo {
    /// Public IP address
    pub query: String,
    /// Country name (e.g., "Vietnam")
    pub country: String,
    /// ISO 3166-1 alpha-2 country code (e.g., "VN")
    #[serde(rename = "countryCode")]
    pub country_code: String,
    /// City name (e.g., "Ho Chi Minh City")
    pub city: String,
    /// Internet Service Provider name
    pub isp: String,
}

/// API response wrapper to handle success/error status
#[derive(Debug, Deserialize)]
struct ApiResponse {
    status: String,
    message: Option<String>,
    query: Option<String>,
    country: Option<String>,
    #[serde(rename = "countryCode")]
    country_code: Option<String>,
    city: Option<String>,
    isp: Option<String>,
}

/// Errors that can occur during geo-IP lookup
#[derive(Debug, Error)]
pub enum GeoError {
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("API error: {0}")]
    ApiError(String),
    #[error("Invalid response: missing fields")]
    InvalidResponse,
}

/// Fetches current geographic location based on public IP
pub async fn fetch_location() -> Result<GeoInfo, GeoError> {
    let client = reqwest::Client::new();

    let response: ApiResponse = client
        .get(API_URL)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?
        .json()
        .await?;

    if response.status == "fail" {
        return Err(GeoError::ApiError(
            response.message.unwrap_or_else(|| "Unknown error".to_string())
        ));
    }

    Ok(GeoInfo {
        query: response.query.ok_or(GeoError::InvalidResponse)?,
        country: response.country.ok_or(GeoError::InvalidResponse)?,
        country_code: response.country_code.ok_or(GeoError::InvalidResponse)?,
        city: response.city.ok_or(GeoError::InvalidResponse)?,
        isp: response.isp.ok_or(GeoError::InvalidResponse)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_location() {
        // Skip in CI environment without network
        if std::env::var("CI").is_ok() {
            return;
        }

        let result = fetch_location().await;
        assert!(result.is_ok(), "Failed to fetch location: {:?}", result.err());

        let info = result.unwrap();
        assert!(!info.query.is_empty());
        assert!(!info.country.is_empty());
        assert_eq!(info.country_code.len(), 2);
    }
}
