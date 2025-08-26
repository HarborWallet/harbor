use anyhow::anyhow;
use arti_client::{TorAddr, TorClient};
use fedimint_core::util::SafeUrl;
use http_body_util::Empty;
use hyper::body::{Body, Bytes};
use hyper::header::LOCATION;
use hyper::{Request, Uri};
use hyper_rustls::HttpsConnectorBuilder;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use once_cell::sync::OnceCell;
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_rustls::rustls::RootCertStore;
use tor_rtcompat::PreferredRuntime;
use url::Url;

// Global TorClient singleton
static TOR_CLIENT: OnceCell<Arc<TorClient<PreferredRuntime>>> = OnceCell::new();

/// Initialize the Tor client if not already initialized
fn initialize_tor_client() -> anyhow::Result<Arc<TorClient<PreferredRuntime>>> {
    let client = TorClient::builder()
        .bootstrap_behavior(arti_client::BootstrapBehavior::OnDemand)
        .create_unbootstrapped()?;
    Ok(Arc::new(client))
}

/// Get or initialize the Tor client
fn get_tor_client() -> anyhow::Result<Arc<TorClient<PreferredRuntime>>> {
    match TOR_CLIENT.get() {
        Some(client) => Ok(client.clone()),
        _ => {
            let client = initialize_tor_client()?;
            // It's okay if another thread beat us to initialization
            let _ = TOR_CLIENT.set(client.clone());
            Ok(client)
        }
    }
}

const MAX_REDIRECTS: u8 = 5;
const MAX_RESPONSE_SIZE: usize = 10 * 1024 * 1024; // 10MB limit

/// Make a GET request using normal TCP with TLS.
///
/// This is the standard way to make HTTPS requests. It:
/// - Uses a connection pool for better performance
/// - Handles redirects automatically (up to `MAX_REDIRECTS`)
/// - Enforces a response size limit
/// - Returns deserialized JSON
pub async fn make_get_request_direct<T>(url: &str) -> anyhow::Result<T>
where
    T: DeserializeOwned + Send + 'static,
{
    make_get_request_direct_internal::<T>(url.to_string(), 0).await
}

/// Helper function to monitor cancellation
async fn check_cancel(cancel_handle: Arc<AtomicBool>) {
    while !cancel_handle.load(Ordering::Relaxed) {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Make a GET request through the Tor network.
///
/// This provides enhanced privacy by:
/// - Routing all traffic through the Tor network
/// - Supporting .onion addresses
/// - Enforcing HTTPS-only connections
/// - Using fresh circuits for each request
///
/// The request can be cancelled at any time using the `cancel_handle`.
///
/// Note: This is slower than direct requests due to Tor routing.
pub async fn make_get_request_tor<T>(url: &str, cancel_handle: Arc<AtomicBool>) -> anyhow::Result<T>
where
    T: DeserializeOwned + Send + 'static,
{
    make_tor_request::<T, ()>(url, None, cancel_handle).await
}

/// Make a GET request through the Tor network.
///
/// This provides enhanced privacy by:
/// - Routing all traffic through the Tor network
/// - Supporting .onion addresses
/// - Enforcing HTTPS-only connections
/// - Using fresh circuits for each request
///
/// The request can be cancelled at any time using the `cancel_handle`.
///
/// Note: This is slower than direct requests due to Tor routing.
pub async fn make_tor_request<T, P>(
    url: &str,
    payload: Option<P>,
    cancel_handle: Arc<AtomicBool>,
) -> anyhow::Result<T>
where
    P: Serialize + Sized,
    T: DeserializeOwned + Send + 'static,
{
    log::debug!("Making get request to tor: {}", url);

    let safe_url = SafeUrl::parse(url)?;
    if safe_url.scheme() != "https" {
        return Err(anyhow!("Only HTTPS is supported"));
    }

    // Get a reference to the global TorClient
    let tor_client = get_tor_client()?;

    log::debug!("Starting bootstrap if needed");

    // Set a timeout for the bootstrap process
    let bootstrap_timeout = Duration::from_secs(60);

    // Use select! to handle cancellation during bootstrap
    let bootstrap_result = tokio::select! {
        biased;  // Check cancellation first
        () = check_cancel(cancel_handle.clone()) => {
            return Err(anyhow!("Request cancelled during bootstrap"));
        }
        result = tokio::time::timeout(bootstrap_timeout, tor_client.bootstrap()) => result,
    };

    match bootstrap_result {
        Ok(Ok(())) => log::debug!("Successfully bootstrapped Tor client"),
        Ok(Err(e)) => return Err(anyhow!("Failed to bootstrap Tor client: {:?}", e)),
        Err(_) => {
            return Err(anyhow!(
                "Tor client bootstrap timed out after {} seconds",
                bootstrap_timeout.as_secs()
            ));
        }
    }

    let tor_client = tor_client.isolated_client();

    let host = safe_url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("Expected host str"))?
        .to_string();
    let port = safe_url
        .port_or_known_default()
        .ok_or_else(|| anyhow::anyhow!("Expected port number"))?;

    // Parse the URL properly
    let parsed_url = Url::parse(url)?;
    // Get the path and query string
    let path = if let Some(query) = parsed_url.query() {
        format!("{}?{}", parsed_url.path(), query)
    } else {
        parsed_url.path().to_string()
    };
    let is_onion = safe_url.is_onion_address();

    let tor_addr = TorAddr::from((host.as_str(), port))
        .map_err(|e| anyhow::anyhow!("Invalid endpoint addr: {:?}: {e:#}", (&host, port)))?;

    log::debug!("Attempting to connect to {}:{} via Tor", &host, port);

    let connect_timeout = Duration::from_secs(30);
    let stream = if is_onion {
        let mut stream_prefs = arti_client::StreamPrefs::default();
        stream_prefs.connect_to_onion_services(arti_client::config::BoolOrAuto::Explicit(true));

        // Use select! to handle cancellation during onion connection
        let stream_result = tokio::select! {
            biased;
            () = check_cancel(cancel_handle.clone()) => {
                return Err(anyhow!("Request cancelled during onion connection"));
            }
            result = tokio::time::timeout(
                connect_timeout,
                tor_client.connect_with_prefs(tor_addr, &stream_prefs),
            ) => result,
        };

        match stream_result {
            Ok(Ok(stream)) => {
                log::debug!("Successfully connected to onion address");
                stream
            }
            Ok(Err(e)) => return Err(anyhow!("Failed to connect to onion service: {:?}", e)),
            Err(_) => {
                return Err(anyhow!(
                    "Connection to onion service timed out after {} seconds",
                    connect_timeout.as_secs()
                ));
            }
        }
    } else {
        // Use select! to handle cancellation during regular connection
        let stream_result = tokio::select! {
            biased;
            () = check_cancel(cancel_handle.clone()) => {
                return Err(anyhow!("Request cancelled during connection"));
            }
            result = tokio::time::timeout(connect_timeout, tor_client.connect(tor_addr)) => result,
        };

        match stream_result {
            Ok(Ok(stream)) => {
                log::debug!("Successfully connected to regular address");
                stream
            }
            Ok(Err(e)) => return Err(anyhow!("Failed to connect: {:?}", e)),
            Err(_) => {
                return Err(anyhow!(
                    "Connection timed out after {} seconds",
                    connect_timeout.as_secs()
                ));
            }
        }
    };

    // After getting the stream, wrap it in TLS
    let mut root_store = RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let config = tokio_rustls::rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let connector = tokio_rustls::TlsConnector::from(Arc::new(config));
    let server_name = rustls_pki_types::ServerName::try_from(host.as_str())
        .map_err(|_| anyhow!("Invalid DNS name: {}", host))?
        .to_owned();

    log::debug!("Starting TLS handshake with {}", host);
    let tls_timeout = Duration::from_secs(30);

    // Use select! to handle cancellation during TLS handshake
    let tls_result = tokio::select! {
        biased;
        () = check_cancel(cancel_handle.clone()) => {
            return Err(anyhow!("Request cancelled during TLS handshake"));
        }
        result = tokio::time::timeout(tls_timeout, connector.connect(server_name, stream)) => result,
    };

    let tls_stream = match tls_result {
        Ok(Ok(s)) => {
            log::debug!("TLS handshake successful");
            s
        }
        Ok(Err(e)) => return Err(anyhow!("TLS handshake failed: {:?}", e)),
        Err(_) => {
            return Err(anyhow!(
                "TLS handshake timed out after {} seconds",
                tls_timeout.as_secs()
            ));
        }
    };

    make_request_tor(host, path, payload, tls_stream, cancel_handle).await
}

async fn make_request_tor<T, S, P>(
    host: String,
    path: String,
    payload: Option<P>,
    stream: S,
    cancel_handle: Arc<AtomicBool>,
) -> anyhow::Result<T>
where
    P: Serialize + Sized,
    T: DeserializeOwned + Send + 'static,
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    // Check if cancelled before sending request
    if cancel_handle.load(Ordering::Relaxed) {
        return Err(anyhow!("Request cancelled"));
    }

    // Create a Hyper connection using the TLS-wrapped Tor stream
    log::debug!("Creating TokioIo wrapper for stream");
    let io = hyper_util::rt::TokioIo::new(stream);

    // For single connections, we need to use relative paths and set the Host header
    log::debug!("Building request for path: {} with host: {}", path, host);
    match payload {
        None => {
            log::debug!("Starting HTTP/1.1 handshake");
            let (sender, conn) = hyper::client::conn::http1::handshake(io).await?;
            log::debug!("HTTP/1.1 handshake successful");

            // Spawn the connection driver task
            tokio::spawn(async move {
                if let Err(err) = conn.await {
                    log::error!("Connection driver failed: {:?}", err);
                }
            });

            let request = build_get_request(path, Some(host))?;
            // Log the full request for debugging
            log::debug!(
                "Sending request: {} {} {:?}",
                request.method(),
                request.uri(),
                request.headers()
            );

            handle_http_request(request, sender).await
        }
        Some(payload) => {
            log::debug!("Starting HTTP/1.1 handshake");
            let (sender, conn) = hyper::client::conn::http1::handshake(io).await?;
            log::debug!("HTTP/1.1 handshake successful");

            // Spawn the connection driver task
            tokio::spawn(async move {
                if let Err(err) = conn.await {
                    log::error!("Connection driver failed: {:?}", err);
                }
            });

            let request = build_post_request(path, Some(host), payload)?;
            // Log the full request for debugging
            log::debug!(
                "Sending request: {} {} {:?}",
                request.method(),
                request.uri(),
                request.headers()
            );

            handle_http_request(request, sender).await
        }
    }
}

// Create a new Hyper client with TLS support and reasonable defaults
fn create_https_client() -> Client<
    hyper_rustls::HttpsConnector<hyper_util::client::legacy::connect::HttpConnector>,
    Empty<Bytes>,
> {
    let https = HttpsConnectorBuilder::new()
        .with_webpki_roots()
        .https_only() // Enforce HTTPS for all connections
        .enable_http1()
        .build();

    Client::builder(TokioExecutor::new())
        .pool_idle_timeout(Duration::from_secs(30))
        .pool_max_idle_per_host(1)
        .build(https)
}

/// Common response handling logic
async fn handle_response<T, B, E>(
    response: hyper::Response<B>,
    redirect_count: u8,
    original_url: Option<&str>,
) -> anyhow::Result<T>
where
    T: DeserializeOwned + Send + 'static,
    E: std::error::Error + Send + Sync + 'static,
    B: http_body_util::BodyExt<Data = Bytes, Error = E>,
{
    let status = response.status();

    if status.is_redirection() {
        if let Some(location) = response.headers().get(LOCATION) {
            let location_str = location.to_str()?;
            log::debug!("Got redirect to: {}", location_str);

            // Handle relative redirects
            let redirect_url = if location_str.starts_with('/') {
                if let Some(base_url) = original_url {
                    let base = SafeUrl::parse(base_url)?;
                    format!(
                        "{}://{}{}",
                        base.scheme(),
                        base.host_str().unwrap_or_default(),
                        location_str
                    )
                } else {
                    return Err(anyhow!(
                        "Cannot handle relative redirect without original URL"
                    ));
                }
            } else {
                location_str.to_string()
            };

            log::debug!("Following redirect to: {}", redirect_url);
            return make_get_request_direct_internal::<T>(redirect_url, redirect_count + 1).await;
        }
        return Err(anyhow!("Redirect response missing Location header"));
    }

    if !status.is_success() {
        // Read and log the error response body
        let body_bytes = http_body_util::BodyExt::collect(response.into_body())
            .await?
            .to_bytes();
        let body_str = String::from_utf8_lossy(&body_bytes);

        log::error!(
            "HTTP request failed\nStatus: {}\nResponse body: {}",
            status,
            body_str.trim()
        );
        return Err(anyhow!("HTTP request failed with status: {}", status));
    }

    // Read the response body with size limit
    let body_bytes = http_body_util::BodyExt::collect(response.into_body())
        .await?
        .to_bytes();

    if body_bytes.len() > MAX_RESPONSE_SIZE {
        return Err(anyhow!(
            "Response too large, exceeded {} bytes",
            MAX_RESPONSE_SIZE
        ));
    }

    // Parse the JSON response
    let parsed = serde_json::from_slice(&body_bytes)?;
    Ok(parsed)
}

/// Common HTTP request/response handling logic for single connections
async fn handle_http_request<T, P>(
    request: Request<P>,
    mut sender: hyper::client::conn::http1::SendRequest<P>,
) -> anyhow::Result<T>
where
    P: Body + 'static,
    T: DeserializeOwned + Send + 'static,
{
    log::debug!("Sending request to server");
    let response = sender.send_request(request).await?;
    log::debug!(
        "Got response: {} {:?}",
        response.status(),
        response.headers()
    );
    handle_response(response, 0, None).await
}

/// Use what Chrome puts for User Agent for better privacy, copied from: `https://www.whatismybrowser.com/guides/the-latest-user-agent/chrome`
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36";

/// Build a GET request with common headers
fn build_get_request(
    uri: impl AsRef<str>,
    host: Option<String>,
) -> anyhow::Result<Request<Empty<Bytes>>> {
    let uri_str = uri.as_ref();
    let mut builder = Request::builder()
        .uri(uri_str)
        .header("User-Agent", USER_AGENT);

    // Only set Host header if we're not using an absolute URL
    if let Some(host) = host {
        if !uri_str.starts_with("http://") && !uri_str.starts_with("https://") {
            builder = builder.header("Host", host);
        }
    }

    builder
        .body(Empty::<Bytes>::new())
        .map_err(|e| anyhow!("Failed to build request: {}", e))
}

/// Build a POST request with common headers
fn build_post_request<P: Serialize + Sized>(
    uri: impl AsRef<str>,
    host: Option<String>,
    payload: P,
) -> anyhow::Result<Request<String>> {
    let uri_str = uri.as_ref();
    let mut builder = Request::builder()
        .uri(uri_str)
        .header("User-Agent", USER_AGENT)
        .header("Content-Type", "application/json")
        .method("POST");

    // Only set Host header if we're not using an absolute URL
    if let Some(host) = host {
        if !uri_str.starts_with("http://") && !uri_str.starts_with("https://") {
            builder = builder.header("Host", host);
        }
    }

    let body = serde_json::to_string(&payload)?;
    builder
        .body(body)
        .map_err(|e| anyhow!("Failed to build request: {}", e))
}

fn make_get_request_direct_internal<T>(
    url: String,
    redirect_count: u8,
) -> std::pin::Pin<Box<dyn Future<Output = anyhow::Result<T>> + Send>>
where
    T: DeserializeOwned + Send + 'static,
{
    Box::pin(async move {
        if redirect_count >= MAX_REDIRECTS {
            return Err(anyhow!("Too many redirects (max {})", MAX_REDIRECTS));
        }

        // Parse and validate URL
        let parsed_url = Url::parse(&url)?;
        if parsed_url.scheme() != "https" {
            return Err(anyhow!("Only HTTPS is supported"));
        }

        log::debug!("Making direct get request to: {}", url);

        let client = create_https_client();
        let uri: Uri = url
            .parse()
            .map_err(|e| anyhow!("Invalid URL '{}': {}", url, e))?;

        let request = build_get_request(uri.to_string(), None)?;

        let response = client.request(request).await.map_err(|e| {
            if e.to_string().contains("rustls") {
                anyhow!("TLS error while connecting: {}", e)
            } else {
                anyhow!("HTTP request failed: {}", e)
            }
        })?;

        handle_response(response, redirect_count, Some(&url)).await
    })
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
                log::error!("Failed to fetch metadata: {e:?}");
                panic!("Failed to fetch metadata: {e:?}");
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
                log::error!("Failed to fetch metadata: {e:?}");
                panic!("Failed to fetch metadata: {e:?}");
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

        // Use httpbin's HTTPS endpoint which redirects to /get
        let result =
            make_get_request_direct::<serde_json::Value>("https://httpbin.org/redirect/1").await;

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
                log::error!("Failed to follow redirect: {e:?}");
                panic!("Failed to follow redirect: {e:?}");
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
