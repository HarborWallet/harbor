use arti_client::{TorAddr, TorClient, TorClientConfig};
use fedimint_core::util::SafeUrl;
use http_body_util::{BodyExt, Empty};
use hyper::body::Bytes;
use hyper::Request;
use hyper_util::rt::TokioIo;
use serde::de::DeserializeOwned;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_native_tls::native_tls::TlsConnector;

pub(crate) async fn make_get_request_tor<T>(url: &str) -> anyhow::Result<T>
where
    T: DeserializeOwned,
{
    log::debug!("Making get request to tor: {}", url);
    let tor_config = TorClientConfig::default();
    let tor_client = TorClient::create_bootstrapped(tor_config)
        .await?
        .isolated_client();

    log::debug!("Successfully created and bootstrapped the `TorClient`, for given `TorConfig`.");

    let safe_url = SafeUrl::parse(url)?;
    let https = safe_url.scheme() == "https";

    log::debug!("Successfully parsed the URL into a `SafeUrl`.");

    let host = safe_url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("Expected host str"))?;
    let port = safe_url
        .port_or_known_default()
        .ok_or_else(|| anyhow::anyhow!("Expected port number"))?;
    let tor_addr = TorAddr::from((host, port))
        .map_err(|e| anyhow::anyhow!("Invalid endpoint addr: {:?}: {e:#}", (host, port)))?;

    log::debug!("Successfully created `TorAddr` for given address (i.e. host and port)");

    let stream = if safe_url.is_onion_address() {
        let mut stream_prefs = arti_client::StreamPrefs::default();
        stream_prefs.connect_to_onion_services(arti_client::config::BoolOrAuto::Explicit(true));

        let anonymized_stream = tor_client
            .connect_with_prefs(tor_addr, &stream_prefs)
            .await?;

        log::debug!("Successfully connected to onion address `TorAddr`, and established an anonymized `DataStream`");
        anonymized_stream
    } else {
        let anonymized_stream = tor_client.connect(tor_addr).await?;

        log::debug!("Successfully connected to `Hostname`or `Ip` `TorAddr`, and established an anonymized `DataStream`");
        anonymized_stream
    };

    let res = if https {
        let cx = TlsConnector::builder().build()?;
        let cx = tokio_native_tls::TlsConnector::from(cx);
        let stream = cx.connect(host, stream).await?;
        make_request(&safe_url, stream).await?
    } else {
        make_request(&safe_url, stream).await?
    };

    Ok(res)
}

async fn make_request<T>(
    url: &SafeUrl,
    stream: impl AsyncRead + AsyncWrite + Unpin + Send + 'static,
) -> anyhow::Result<T>
where
    T: DeserializeOwned,
{
    let (mut request_sender, connection) =
        hyper::client::conn::http1::handshake(TokioIo::new(stream)).await?;

    // spawn a task to poll the connection and drive the HTTP state
    tokio::spawn(async move {
        connection.await.unwrap();
    });

    let req = Request::get(url.as_str())
        .header("Host", url.host_str().expect("already checked for host"))
        .body(Empty::<Bytes>::new())?;
    let mut resp = request_sender.send_request(req).await?;

    log::debug!("Successfully sent the request.");

    let len: usize = resp
        .headers()
        .get("content-length")
        .and_then(|h| h.to_str().ok().and_then(|s| s.parse().ok()))
        .unwrap_or(10_000);

    // if over 20MB, something is going wrong
    if len > 20000000 {
        return Err(anyhow::anyhow!(
            "Received too large of response, size: {len}"
        ));
    }

    let mut buf: Vec<u8> = Vec::with_capacity(len);
    while let Some(frame) = resp.body_mut().frame().await {
        let bytes = frame?.into_data().unwrap();
        buf.extend_from_slice(&bytes);
    }

    log::debug!("Successfully received the response body.");

    let text = String::from_utf8(buf)?;
    serde_json::from_str(&text).map_err(anyhow::Error::from)
}

pub(crate) async fn make_get_request_direct<T>(url: &str) -> anyhow::Result<T>
where
    T: DeserializeOwned,
{
    let response = reqwest::get(url).await?;
    let status = response.status();
    let text = response.text().await?;

    if !status.is_success() {
        return Err(anyhow::anyhow!(
            "Request failed with status {}: {}",
            status,
            text
        ));
    }

    serde_json::from_str(&text).map_err(anyhow::Error::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::FederationMetaConfig;

    #[tokio::test]
    async fn test_fetch_metadata() {
        let res =
            make_get_request_tor::<FederationMetaConfig>("https://meta.dev.fedibtc.com/meta.json")
                .await
                .unwrap();

        assert!(!res.federations.is_empty());
    }
}
