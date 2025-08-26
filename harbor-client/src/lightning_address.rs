use crate::http::{make_get_request_direct, make_get_request_tor};
use lnurl::{
    lightning_address::LightningAddress,
    lnurl::LnUrl,
    pay::{LnURLPayInvoice, PayResponse},
};
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

pub fn parse_lnurl(address: &str) -> anyhow::Result<LnUrl> {
    match LightningAddress::from_str(address) {
        Ok(lightning_address) => Ok(lightning_address.lnurl()),
        Err(_) => LnUrl::from_str(address)
            .map_err(|_| anyhow::anyhow!("Invalid lightning address or lnurl")),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    async fn try_with_tor_mode(lnurl: &LnUrl, tor_enabled: bool) -> anyhow::Result<()> {
        log::debug!(
            "Starting lightning address test with tor_enabled={}",
            tor_enabled
        );
        let cancel_handle = Arc::new(AtomicBool::new(false));

        // Get the LNURL data
        log::debug!("LNURL decoded: {}", lnurl.url);

        log::debug!("Making LNURL request to get payment details");
        let pay_response = make_lnurl_request(lnurl, tor_enabled, cancel_handle.clone()).await?;
        log::debug!(
            "Got payment details - min_sendable: {}, max_sendable: {}, callback: {}",
            pay_response.min_sendable,
            pay_response.max_sendable,
            pay_response.callback
        );

        // Verify the pay response
        assert!(
            pay_response.min_sendable > 0,
            "min_sendable should be greater than 0"
        );
        assert!(
            pay_response.max_sendable > pay_response.min_sendable,
            "max_sendable should be greater than min_sendable"
        );

        log::debug!("Requesting invoice for {} msats", pay_response.min_sendable);
        // Try to get an invoice for the minimum amount
        let invoice = get_invoice(
            &pay_response,
            pay_response.min_sendable,
            tor_enabled,
            cancel_handle.clone(),
        )
        .await?;

        // Verify we got a valid invoice
        assert!(!invoice.pr.is_empty(), "Invoice should not be empty");
        log::info!(
            "Successfully got invoice with tor_enabled={}: {}",
            tor_enabled,
            invoice.pr
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_lightning_address_flow() -> anyhow::Result<()> {
        init();
        log::debug!("Starting test_lightning_address_flow");

        // Test parsing the lightning address
        let address = "refund@lnurl.mutinynet.com";
        log::debug!("Attempting to parse lightning address: {}", address);
        let ln_address = parse_lnurl(address)?;
        log::debug!("Successfully parsed lightning address");

        // Always test without Tor first
        log::debug!("Starting non-Tor test");
        try_with_tor_mode(&ln_address, false).await?;
        log::debug!("Non-Tor test completed successfully");

        // Test with Tor, but don't fail the whole test if Tor fails
        log::debug!("Starting Tor test");
        match try_with_tor_mode(&ln_address, true).await {
            Ok(()) => {
                log::debug!("Tor test completed successfully");
            }
            Err(e) => {
                log::warn!("Tor test failed (this is not fatal): {:#}", e);
                println!("Note: Tor test failed but this is expected in some environments");
            }
        }

        log::debug!("test_lightning_address_flow completed");
        Ok(())
    }
}
