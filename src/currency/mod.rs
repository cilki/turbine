use std::{collections::HashMap, time::Duration};

use anyhow::{anyhow, bail, Result};
use cached::proc_macro::once;

#[cfg(feature = "monero")]
pub mod monero;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Address {
    BTC(String),
    #[cfg(feature = "monero")]
    XMR(String),
}

impl Address {
    pub fn try_parse(currency: &str, address: &str) -> Option<Self> {
        match currency.to_lowercase().as_str() {
            // BTC address validation needs a dedicated bitcoin crate; for now only
            // reject the obviously-empty case.
            "btc" if !address.is_empty() => Some(Self::BTC(address.into())),
            #[cfg(feature = "monero")]
            "xmr" => {
                // Reject anything the monero crate can't parse as a real address so
                // we never register (and later try to pay out to) a garbage string.
                address
                    .parse::<::monero::Address>()
                    .ok()
                    .map(|_| Self::XMR(address.into()))
            }
            _ => None,
        }
    }
}

/// Look up the current USD value of one unit of the given currency.
#[once(time = "3600", result = true)]
pub async fn lookup(symbol: &str) -> Result<f64> {
    // CoinGecko's public price endpoint keys on the coin's *id*, not its ticker,
    // and needs no API key (unlike cryptocompare, which now 401s without one).
    let id = match symbol.to_uppercase().as_str() {
        "XMR" => "monero",
        "BTC" => "bitcoin",
        other => bail!("No price source configured for currency {other}"),
    };

    // Response shape: { "monero": { "usd": 168.0 } }. We ask for the price of the
    // coin *in* USD, so the result can be multiplied directly by a coin balance to
    // get its USD value.
    let mapping: HashMap<String, HashMap<String, f64>> = reqwest::get(format!(
        "https://api.coingecko.com/api/v3/simple/price?ids={id}&vs_currencies=usd"
    ))
    .await?
    .json()
    .await?;

    mapping
        .get(id)
        .and_then(|quotes| quotes.get("usd"))
        .copied()
        .ok_or_else(|| anyhow!("Price response missing a USD quote for {symbol}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_parse_btc() {
        assert_eq!(
            Address::try_parse("btc", "1BvBMSEYstWetqTFn5Au4m4GFg7xJaNVN2"),
            Some(Address::BTC("1BvBMSEYstWetqTFn5Au4m4GFg7xJaNVN2".into()))
        );
        // Currency matching is case-insensitive.
        assert!(Address::try_parse("BTC", "1BvBMSEYstWetqTFn5Au4m4GFg7xJaNVN2").is_some());
        // Empty and unknown currencies are rejected.
        assert_eq!(Address::try_parse("btc", ""), None);
        assert_eq!(Address::try_parse("doge", "whatever"), None);
    }

    #[cfg(feature = "monero")]
    #[test]
    fn try_parse_xmr() {
        let valid = "4AdUndXHHZ6cfufTMvppY6JwXNouMBzSkbLYfpAV5Usx3skxNgYeYTRj5UzqtReoS44qo9mtmXCqY45DJ852K5Jv2684Rge";
        assert_eq!(
            Address::try_parse("xmr", valid),
            Some(Address::XMR(valid.into()))
        );
        // A malformed address (bad checksum / wrong length) is rejected instead of
        // being stored as a payout target.
        assert_eq!(Address::try_parse("xmr", "not-a-real-address"), None);
        assert_eq!(Address::try_parse("xmr", ""), None);
    }
}
