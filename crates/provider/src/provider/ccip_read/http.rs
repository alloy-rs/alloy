//! The default `reqwest`-backed [`CcipReadGateway`].

use super::{CcipReadGateway, CcipReadGatewayError, CcipReadRequest};
use alloy_primitives::Bytes;
use reqwest::{header::CONTENT_TYPE, Url};
use serde::{Deserialize, Serialize};

/// The default HTTP implementation of [`CcipReadGateway`].
///
/// Requests follow ERC-3668: URL templates containing `{data}` are fetched with `GET`, all
/// others receive a `POST` with a JSON body carrying `sender` and `data`. The URLs of a request
/// are tried in order; a `4xx` response aborts the request, while any other failure falls through
/// to the next URL.
///
/// # URL trust
///
/// Templates in [`CcipReadRequest::urls`] are chosen by the callee contract. This gateway only
/// checks that each URL uses `http` or `https` and will follow a URL to a private, link-local,
/// loopback, or cloud-metadata address. To constrain destinations, construct
/// [`HttpCcipReadGateway::new`] with a custom [`reqwest::Client`], or implement
/// [`CcipReadGateway`] with your own allowlist, blocklist, or DNS/IP policy.
#[derive(Clone, Debug)]
pub struct HttpCcipReadGateway {
    client: reqwest::Client,
}

impl HttpCcipReadGateway {
    /// Creates a gateway using an existing HTTP client, keeping its timeout and redirect policy.
    pub const fn new(client: reqwest::Client) -> Self {
        Self { client }
    }

    /// Sends one gateway request for `template`.
    async fn fetch(
        &self,
        template: &str,
        sender: &str,
        data: &str,
        max_response_size: usize,
    ) -> Attempt {
        let url = template.replace("{sender}", sender).replace("{data}", data);
        let url = match Url::parse(&url) {
            Ok(url) if matches!(url.scheme(), "http" | "https") => url,
            Ok(_) => {
                return Attempt::Retry(CcipReadGatewayError::new(
                    "CCIP Read gateway URL must use http or https",
                ))
            }
            Err(err) => {
                return Attempt::Retry(CcipReadGatewayError::new(format!(
                    "invalid CCIP Read gateway URL: {err}"
                )))
            }
        };

        let request = if template.contains("{data}") {
            self.client.get(url)
        } else {
            let body = serde_json::to_vec(&GatewayRequestBody { sender, data })
                .expect("serializing two strings cannot fail");
            self.client.post(url).header(CONTENT_TYPE, "application/json").body(body)
        };
        let response = match request.send().await {
            Ok(response) => response,
            Err(err) => {
                return Attempt::Retry(CcipReadGatewayError::new(format!(
                    "gateway request failed: {err}"
                )))
            }
        };

        let status = response.status().as_u16();
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string();
        match read_body(response, status, max_response_size).await {
            Ok(body) => classify_response(status, &content_type, &body),
            Err(error) => Attempt::failure(status, error),
        }
    }
}

impl Default for HttpCcipReadGateway {
    fn default() -> Self {
        #[cfg(not(target_family = "wasm"))]
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .expect("default CCIP Read HTTP client configuration is valid");
        #[cfg(target_family = "wasm")]
        let client = reqwest::Client::new();
        Self::new(client)
    }
}

#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
impl CcipReadGateway for HttpCcipReadGateway {
    async fn request(
        &self,
        request: &CcipReadRequest,
        max_response_size: usize,
    ) -> Result<Bytes, CcipReadGatewayError> {
        if request.urls.is_empty() {
            return Err(CcipReadGatewayError::new("OffchainLookup contained no gateway URLs"));
        }

        let sender = format!("{:#x}", request.sender);
        let data = request.data.to_string();
        let mut last_error = None;
        for template in &request.urls {
            match self.fetch(template, &sender, &data, max_response_size).await {
                Attempt::Data(data) => return Ok(data),
                Attempt::Fatal(error) => return Err(error),
                Attempt::Retry(error) => last_error = Some(error),
            }
        }
        Err(last_error.expect("at least one URL was attempted"))
    }
}

#[derive(Serialize)]
struct GatewayRequestBody<'a> {
    sender: &'a str,
    data: &'a str,
}

#[derive(Deserialize)]
struct GatewayResponse {
    data: Bytes,
}

/// Outcome of one gateway URL attempt.
#[derive(Debug)]
enum Attempt {
    /// The gateway returned data.
    Data(Bytes),
    /// The request failed with a client error, so the remaining URLs are not tried.
    Fatal(CcipReadGatewayError),
    /// The request failed; the next URL is tried.
    Retry(CcipReadGatewayError),
}

impl Attempt {
    /// Classifies a failed attempt by HTTP status: `4xx` responses are fatal per ERC-3668.
    fn failure(status: u16, error: CcipReadGatewayError) -> Self {
        if (400..500).contains(&status) {
            Self::Fatal(error)
        } else {
            Self::Retry(error)
        }
    }
}

/// Reads the response body, rejecting bodies larger than `max_response_size`.
#[cfg(not(target_family = "wasm"))]
async fn read_body(
    mut response: reqwest::Response,
    status: u16,
    max_response_size: usize,
) -> Result<Vec<u8>, CcipReadGatewayError> {
    if response.content_length().is_some_and(|length| length > max_response_size as u64) {
        return Err(size_limit_error(status));
    }
    let mut body = Vec::new();
    loop {
        match response.chunk().await {
            Ok(Some(chunk)) => {
                if body.len() + chunk.len() > max_response_size {
                    return Err(size_limit_error(status));
                }
                body.extend_from_slice(&chunk);
            }
            Ok(None) => return Ok(body),
            Err(err) => {
                return Err(CcipReadGatewayError::new(format!(
                    "failed reading gateway response: {err}"
                )))
            }
        }
    }
}

/// Reads the response body, rejecting bodies larger than `max_response_size`.
#[cfg(target_family = "wasm")]
async fn read_body(
    response: reqwest::Response,
    status: u16,
    max_response_size: usize,
) -> Result<Vec<u8>, CcipReadGatewayError> {
    if response.content_length().is_some_and(|length| length > max_response_size as u64) {
        return Err(size_limit_error(status));
    }
    let body = response.bytes().await.map_err(|err| {
        CcipReadGatewayError::new(format!("failed reading gateway response: {err}"))
    })?;
    if body.len() > max_response_size {
        return Err(size_limit_error(status));
    }
    Ok(body.to_vec())
}

fn size_limit_error(status: u16) -> CcipReadGatewayError {
    CcipReadGatewayError::http(status, "gateway response exceeded configured size limit")
}

/// Classifies a complete gateway response by status, content type, and body.
fn classify_response(status: u16, content_type: &str, body: &[u8]) -> Attempt {
    if (400..500).contains(&status) {
        return Attempt::Fatal(CcipReadGatewayError::http(status, response_message(body)));
    }
    if !(200..300).contains(&status) {
        return Attempt::Retry(CcipReadGatewayError::http(status, response_message(body)));
    }
    let is_json = content_type
        .split(';')
        .next()
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("application/json"));
    if !is_json {
        return Attempt::Retry(CcipReadGatewayError::http(
            status,
            "gateway response was not application/json",
        ));
    }
    match serde_json::from_slice::<GatewayResponse>(body) {
        Ok(response) => Attempt::Data(response.data),
        Err(err) => Attempt::Retry(CcipReadGatewayError::http(
            status,
            format!("invalid gateway response: {err}"),
        )),
    }
}

/// Returns the beginning of an error response body as a message.
fn response_message(body: &[u8]) -> String {
    const LIMIT: usize = 1_024;
    String::from_utf8_lossy(&body[..body.len().min(LIMIT)]).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, bytes};

    fn message(attempt: &Attempt) -> &str {
        match attempt {
            Attempt::Data(_) => "",
            Attempt::Fatal(error) | Attempt::Retry(error) => &error.message,
        }
    }

    #[test]
    fn classifies_gateway_responses() {
        let json_ok = br#"{"data":"0xdead"}"#;
        match classify_response(200, "application/json; charset=utf-8", json_ok) {
            Attempt::Data(data) => assert_eq!(data, bytes!("dead")),
            other => panic!("expected data, got {other:?}"),
        }

        let attempt = classify_response(404, "text/plain", b"missing");
        assert!(matches!(&attempt, Attempt::Fatal(error) if error.status == Some(404)));
        assert_eq!(message(&attempt), "missing");

        let attempt = classify_response(503, "application/json", json_ok);
        assert!(matches!(&attempt, Attempt::Retry(error) if error.status == Some(503)));

        let attempt = classify_response(200, "text/plain", json_ok);
        assert!(matches!(attempt, Attempt::Retry(_)));
        assert!(message(&attempt).contains("application/json"));

        for body in [&b"{\"data\":"[..], b"not json", br#"{"data":"zz"}"#] {
            let attempt = classify_response(200, "application/json", body);
            assert!(matches!(attempt, Attempt::Retry(_)), "{attempt:?}");
            assert!(message(&attempt).contains("invalid gateway response"));
        }
    }

    #[tokio::test]
    async fn rejects_empty_and_invalid_urls() {
        let gateway = HttpCcipReadGateway::default();
        let sender = address!("1111111111111111111111111111111111111111");
        let request = |urls: Vec<String>| CcipReadRequest { sender, urls, data: Bytes::new() };

        let error = gateway.request(&request(vec![]), 1024).await.unwrap_err();
        assert!(error.message.contains("no gateway URLs"));

        let error = gateway
            .request(&request(vec!["ftp://example.test/{data}".into()]), 1024)
            .await
            .unwrap_err();
        assert!(error.message.contains("http or https"));

        let error = gateway.request(&request(vec!["not a url".into()]), 1024).await.unwrap_err();
        assert!(error.message.contains("invalid CCIP Read gateway URL"));
    }
}
