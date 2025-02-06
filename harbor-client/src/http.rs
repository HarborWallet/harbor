use anyhow::anyhow;
use arti_client::{TorAddr, TorClient};
use fedimint_core::util::SafeUrl;
use rustls_pki_types::ServerName;
use serde::de::DeserializeOwned;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_rustls::rustls::{ClientConfig, RootCertStore};
use tokio_rustls::TlsConnector;

pub(crate) async fn make_get_request_tor<T>(
    url: &str,
    cancel_handle: Arc<AtomicBool>,
) -> anyhow::Result<T>
where
    T: DeserializeOwned,
{
    log::debug!("Making get request to tor: {}", url);

    // Check if cancelled before starting
    if cancel_handle.load(Ordering::Relaxed) {
        return Err(anyhow!("Request cancelled"));
    }

    let tor_client = TorClient::builder()
        .bootstrap_behavior(arti_client::BootstrapBehavior::OnDemand)
        .create_unbootstrapped()
        .map_err(|e| anyhow!("Failed to create Tor client: {:?}", e))?;

    log::debug!("Successfully created Tor client, starting bootstrap");

    // Set a timeout for the bootstrap process
    let bootstrap_timeout = tokio::time::Duration::from_secs(30);
    let bootstrap_result = tokio::time::timeout(bootstrap_timeout, tor_client.bootstrap()).await;

    match bootstrap_result {
        Ok(Ok(_)) => log::debug!("Successfully bootstrapped Tor client"),
        Ok(Err(e)) => return Err(anyhow!("Failed to bootstrap Tor client: {:?}", e)),
        Err(_) => {
            return Err(anyhow!(
                "Tor client bootstrap timed out after {} seconds",
                bootstrap_timeout.as_secs()
            ))
        }
    }

    let tor_client = tor_client.isolated_client();

    // Check if cancelled after client creation
    if cancel_handle.load(Ordering::Relaxed) {
        return Err(anyhow!("Request cancelled"));
    }

    let safe_url = SafeUrl::parse(url)?;
    let https = safe_url.scheme() == "https";

    log::debug!("Successfully parsed the URL into a `SafeUrl`: {}", safe_url);

    let host = safe_url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("Expected host str"))?
        .to_string();
    let port = safe_url
        .port_or_known_default()
        .ok_or_else(|| anyhow::anyhow!("Expected port number"))?;
    let path = safe_url.path().to_string();
    let is_onion = safe_url.is_onion_address();

    let tor_addr = TorAddr::from((host.as_str(), port))
        .map_err(|e| anyhow::anyhow!("Invalid endpoint addr: {:?}: {e:#}", (&host, port)))?;

    // Check if cancelled before connection
    if cancel_handle.load(Ordering::Relaxed) {
        return Err(anyhow!("Request cancelled"));
    }

    log::debug!("Attempting to connect to {}:{} via Tor", &host, port);

    let connect_timeout = tokio::time::Duration::from_secs(30);
    let stream = if is_onion {
        let mut stream_prefs = arti_client::StreamPrefs::default();
        stream_prefs.connect_to_onion_services(arti_client::config::BoolOrAuto::Explicit(true));

        match tokio::time::timeout(
            connect_timeout,
            tor_client.connect_with_prefs(tor_addr, &stream_prefs),
        )
        .await
        {
            Ok(Ok(stream)) => {
                log::debug!("Successfully connected to onion address");
                stream
            }
            Ok(Err(e)) => return Err(anyhow!("Failed to connect to onion service: {:?}", e)),
            Err(_) => {
                return Err(anyhow!(
                    "Connection to onion service timed out after {} seconds",
                    connect_timeout.as_secs()
                ))
            }
        }
    } else {
        match tokio::time::timeout(connect_timeout, tor_client.connect(tor_addr)).await {
            Ok(Ok(stream)) => {
                log::debug!("Successfully connected to regular address");
                stream
            }
            Ok(Err(e)) => return Err(anyhow!("Failed to connect: {:?}", e)),
            Err(_) => {
                return Err(anyhow!(
                    "Connection timed out after {} seconds",
                    connect_timeout.as_secs()
                ))
            }
        }
    };

    // Check if cancelled before making request
    if cancel_handle.load(Ordering::Relaxed) {
        return Err(anyhow!("Request cancelled"));
    }

    let res = if https {
        log::debug!("Setting up TLS connection");

        let mut root_store = RootCertStore::empty();
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

        let config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        let connector = TlsConnector::from(Arc::new(config));

        // Parse the hostname into a ServerName
        let server_name = ServerName::try_from(host.to_string())
            .map_err(|_| anyhow!("Invalid DNS name: {}", &host))?;

        log::debug!("Attempting TLS handshake with {}", &host);
        let tls_timeout = tokio::time::Duration::from_secs(30);
        let stream =
            match tokio::time::timeout(tls_timeout, connector.connect(server_name, stream)).await {
                Ok(Ok(s)) => {
                    log::debug!("TLS handshake successful");
                    s
                }
                Ok(Err(e)) => return Err(anyhow!("TLS handshake failed: {:?}", e)),
                Err(_) => {
                    return Err(anyhow!(
                        "TLS handshake timed out after {} seconds",
                        tls_timeout.as_secs()
                    ))
                }
            };

        let response = make_request(
            host.clone(),
            path.clone(),
            stream,
            cancel_handle,
            https,
            port,
        )
        .await?;
        match response {
            RequestResult::Success(data) => data,
            RequestResult::Redirect(_) => {
                return Err(anyhow!("Redirects not supported for Tor requests"))
            }
        }
    } else {
        let response = make_request(
            host.clone(),
            path.clone(),
            stream,
            cancel_handle,
            https,
            port,
        )
        .await?;
        match response {
            RequestResult::Success(data) => data,
            RequestResult::Redirect(_) => {
                return Err(anyhow!("Redirects not supported for Tor requests"))
            }
        }
    };

    Ok(res)
}

async fn make_request<T>(
    host: String,
    path: String,
    mut stream: impl AsyncRead + AsyncWrite + Unpin + Send + 'static,
    cancel_handle: Arc<AtomicBool>,
    https: bool,
    port: u16,
) -> anyhow::Result<RequestResult<T>>
where
    T: DeserializeOwned,
{
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    // This is a minimal HTTP client implementation specifically designed for making
    // GET requests that return JSON responses. It does not support:
    // - POST/PUT/DELETE requests
    // - Streaming responses
    // - Keep-alive connections
    // - Compressed responses
    // - WebSocket connections
    // It does handle:
    // - Basic redirects (301/302)
    // - Response size limits
    // - Timeouts
    // - Cancellation

    const MAX_RESPONSE_SIZE: usize = 10 * 1024 * 1024; // 10MB limit

    // Check if cancelled before sending request
    if cancel_handle.load(Ordering::Relaxed) {
        return Err(anyhow!("Request cancelled"));
    }

    log::debug!("Preparing request to {}{}", host, path);
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: harbor-client/0.1.0\r\nConnection: close\r\n\r\n",
        path,
        host
    );

    log::debug!("Sending request");
    stream.write_all(request.as_bytes()).await?;

    // IMPORTANT: Make sure the request was written
    stream.flush().await?;
    log::debug!("Request sent and flushed");

    // Read the response with a timeout
    let body_timeout = tokio::time::Duration::from_secs(30);
    let read_result = tokio::time::timeout(body_timeout, async {
        let mut buf = Vec::new();
        let mut chunk = [0u8; 8192];

        loop {
            if buf.len() > MAX_RESPONSE_SIZE {
                return Err(anyhow!(
                    "Response too large, exceeded {} bytes",
                    MAX_RESPONSE_SIZE
                ));
            }

            match stream.read(&mut chunk).await? {
                0 => break, // EOF
                n => {
                    buf.extend_from_slice(&chunk[..n]);
                    log::debug!("Read {} bytes", n);
                }
            }
        }

        Ok::<_, anyhow::Error>(buf)
    })
    .await;

    let buf = match read_result {
        Ok(Ok(buf)) => {
            log::debug!("Successfully read response, size: {} bytes", buf.len());
            buf
        }
        Ok(Err(e)) => return Err(anyhow!("Failed to read response: {:?}", e)),
        Err(_) => {
            return Err(anyhow!(
                "Reading response timed out after {} seconds",
                body_timeout.as_secs()
            ))
        }
    };

    // Parse the HTTP response
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut resp = httparse::Response::new(&mut headers);

    match resp.parse(buf.as_slice()) {
        Ok(httparse::Status::Complete(offset)) => {
            let status = resp.code.unwrap_or(500);

            // Handle redirects
            if status == 301 || status == 302 {
                // Find the Location header
                if let Some(location) = headers
                    .iter()
                    .find(|h| h.name.eq_ignore_ascii_case("location"))
                {
                    if let Ok(redirect_url) = std::str::from_utf8(location.value) {
                        log::debug!("Following redirect to: {}", redirect_url);

                        // Handle relative URLs by constructing the full URL
                        let full_redirect_url = if redirect_url.starts_with('/') {
                            // It's a relative URL, construct the full URL using original scheme and port
                            let scheme = if https { "https" } else { "http" };
                            format!("{}://{}:{}{}", scheme, host, port, redirect_url)
                        } else {
                            // It's already a full URL
                            redirect_url.to_string()
                        };

                        log::debug!("Full redirect URL: {}", full_redirect_url);
                        return Ok(RequestResult::Redirect(full_redirect_url));
                    }
                }
                return Err(anyhow!("Redirect response missing Location header"));
            }

            if status != 200 {
                return Err(anyhow!("HTTP request failed with status: {}", status));
            }

            // Find the response body after headers
            let body = &buf[offset..];
            log::debug!("Parsing response body as JSON");
            let parsed = serde_json::from_slice(body).map_err(anyhow::Error::from)?;
            Ok(RequestResult::Success(parsed))
        }
        _ => Err(anyhow!("Failed to parse HTTP response")),
    }
}

pub(crate) async fn make_get_request_direct<T>(url: &str) -> anyhow::Result<T>
where
    T: DeserializeOwned + Send + 'static,
{
    make_get_request_direct_internal::<T>(url.to_string(), 0).await
}

fn make_get_request_direct_internal<T>(
    url: String,
    redirect_count: u8,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<T>> + Send>>
where
    T: DeserializeOwned + Send + 'static,
{
    Box::pin(async move {
        const MAX_REDIRECTS: u8 = 5;
        if redirect_count >= MAX_REDIRECTS {
            return Err(anyhow!("Too many redirects (max {})", MAX_REDIRECTS));
        }

        log::debug!("Making direct get request to: {}", url);

        let safe_url = SafeUrl::parse(&url)?;
        let https = safe_url.scheme() == "https";

        log::debug!("Successfully parsed the URL into a `SafeUrl`: {}", safe_url);

        let host = safe_url
            .host_str()
            .ok_or_else(|| anyhow::anyhow!("Expected host str"))?
            .to_string();
        let port = safe_url
            .port_or_known_default()
            .ok_or_else(|| anyhow::anyhow!("Expected port number"))?;
        let path = safe_url.path().to_string();

        // Connect with timeout
        let connect_timeout = tokio::time::Duration::from_secs(30);
        let addr = format!("{}:{}", host, port);
        log::debug!("Attempting to connect to {}", addr);

        let stream = match tokio::time::timeout(
            connect_timeout,
            tokio::net::TcpStream::connect(&addr),
        )
        .await
        {
            Ok(Ok(stream)) => {
                log::debug!("Successfully connected to {}", addr);
                stream
            }
            Ok(Err(e)) => return Err(anyhow!("Failed to connect: {:?}", e)),
            Err(_) => {
                return Err(anyhow!(
                    "Connection timed out after {} seconds",
                    connect_timeout.as_secs()
                ))
            }
        };

        let response = if https {
            log::debug!("Setting up TLS connection");

            let mut root_store = RootCertStore::empty();
            root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

            let config = ClientConfig::builder()
                .with_root_certificates(root_store)
                .with_no_client_auth();

            let connector = TlsConnector::from(Arc::new(config));

            // Parse the hostname into a ServerName
            let server_name = ServerName::try_from(host.to_string())
                .map_err(|_| anyhow!("Invalid DNS name: {}", &host))?;

            log::debug!("Attempting TLS handshake with {}", &host);
            let tls_timeout = tokio::time::Duration::from_secs(30);
            let stream =
                match tokio::time::timeout(tls_timeout, connector.connect(server_name, stream))
                    .await
                {
                    Ok(Ok(s)) => {
                        log::debug!("TLS handshake successful");
                        s
                    }
                    Ok(Err(e)) => return Err(anyhow!("TLS handshake failed: {:?}", e)),
                    Err(_) => {
                        return Err(anyhow!(
                            "TLS handshake timed out after {} seconds",
                            tls_timeout.as_secs()
                        ))
                    }
                };

            make_request(
                host,
                path,
                stream,
                Arc::new(AtomicBool::new(false)),
                https,
                port,
            )
            .await
        } else {
            make_request(
                host,
                path,
                stream,
                Arc::new(AtomicBool::new(false)),
                https,
                port,
            )
            .await
        }?;

        match response {
            RequestResult::Success(data) => Ok(data),
            RequestResult::Redirect(url) => {
                make_get_request_direct_internal::<T>(url, redirect_count + 1).await
            }
        }
    })
}

enum RequestResult<T> {
    Success(T),
    Redirect(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::FederationMetaConfig;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[tokio::test]
    async fn test_fetch_metadata() {
        init();
        log::debug!("Starting test_fetch_metadata");

        match make_get_request_tor::<FederationMetaConfig>(
            "https://meta.dev.fedibtc.com/meta.json",
            Arc::new(AtomicBool::new(false)),
        )
        .await
        {
            Ok(res) => {
                log::debug!("Fetched metadata: {:?}", res);
                assert!(!res.federations.is_empty());
            }
            Err(e) => {
                log::error!("Failed to fetch metadata: {:?}", e);
                panic!("Failed to fetch metadata: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_direct_fetch_metadata() {
        init();
        log::debug!("Starting test_direct_fetch_metadata");

        match make_get_request_direct::<FederationMetaConfig>(
            "https://meta.dev.fedibtc.com/meta.json",
        )
        .await
        {
            Ok(res) => {
                log::debug!("Fetched metadata: {:?}", res);
                assert!(!res.federations.is_empty());
            }
            Err(e) => {
                log::error!("Failed to fetch metadata: {:?}", e);
                panic!("Failed to fetch metadata: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_direct_fetch_nonexistent() {
        init();
        log::debug!("Starting test_direct_fetch_nonexistent");

        // This domain should not exist
        let result = make_get_request_direct::<FederationMetaConfig>(
            "https://this-domain-should-not-exist-harbor-test.com/meta.json",
        )
        .await;

        assert!(result.is_err(), "Expected error for non-existent domain");
        if let Err(e) = result {
            log::debug!("Got expected error: {:?}", e);
            // The exact error message might vary by platform/DNS resolver
            // so we just check that we got an error
        }
    }

    #[tokio::test]
    async fn test_direct_fetch_redirect() {
        init();
        log::debug!("Starting test_direct_fetch_redirect");

        // httpbin will redirect to /get
        let result =
            make_get_request_direct::<serde_json::Value>("http://httpbin.org/redirect/1").await;

        match result {
            Ok(res) => {
                log::debug!("Followed redirect successfully: {:?}", res);
                // The /get endpoint returns a JSON object with request info
                assert!(
                    res.is_object(),
                    "Expected JSON object response after redirect"
                );
                assert!(res.get("url").is_some(), "Expected 'url' field in response");
            }
            Err(e) => {
                log::error!("Failed to follow redirect: {:?}", e);
                panic!("Failed to follow redirect: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_direct_fetch_too_many_redirects() {
        init();
        log::debug!("Starting test_direct_fetch_too_many_redirects");

        // httpbin will try to redirect 10 times, which should exceed our MAX_REDIRECTS
        let result =
            make_get_request_direct::<serde_json::Value>("http://httpbin.org/redirect/10").await;

        assert!(result.is_err(), "Expected error for too many redirects");
        if let Err(e) = result {
            log::debug!("Got expected error: {:?}", e);
            // We should hit our MAX_REDIRECTS limit before completing all 10 redirects
        }
    }
}
