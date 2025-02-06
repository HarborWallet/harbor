use crate::http::{make_get_request_direct, make_get_request_tor};
use lnurl::{
    lightning_address::LightningAddress,
    lnurl::LnUrl,
    pay::{LnURLPayInvoice, PayResponse},
};
use std::str::FromStr;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub fn parse_lightning_address(address: &str) -> anyhow::Result<LightningAddress> {
    let ln_address = LightningAddress::from_str(address)?;
    Ok(ln_address)
}

pub async fn get_invoice(
    pay: &PayResponse,
    msats: u64,
    tor_enabled: bool,
    cancel_handle: Arc<AtomicBool>,
) -> anyhow::Result<LnURLPayInvoice> {
    if msats < pay.min_sendable || msats > pay.max_sendable {
        return Err(anyhow::anyhow!("Invalid amount"));
    }

    let symbol = if pay.callback.contains('?') { "&" } else { "?" };
    let url = format!("{}{}amount={}", pay.callback, symbol, msats);

    if tor_enabled {
        make_get_request_tor(&url, cancel_handle).await
    } else {
        make_get_request_direct(&url).await
    }
}

pub async fn make_lnurl_request(
    lnurl: &LnUrl,
    tor_enabled: bool,
    cancel_handle: Arc<AtomicBool>,
) -> anyhow::Result<PayResponse> {
    let lnurlp = lnurl.url.clone();
    log::info!("Making lnurl request: {lnurlp}, tor_enabled: {tor_enabled}");

    if tor_enabled {
        make_get_request_tor(&lnurlp, cancel_handle).await
    } else {
        make_get_request_direct(&lnurlp).await
    }
}
