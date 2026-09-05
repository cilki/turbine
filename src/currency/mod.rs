use std::{collections::HashMap, time::Duration};

use anyhow::Result;
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

/// Lookup the current USD value of the given currency.
#[once(time = "3600", result = true)]
pub async fn lookup(symbol: &str) -> Result<f64> {
    let mapping: HashMap<String, f64> = reqwest::get(format!(
        "https://min-api.cryptocompare.com/data/price?fsym=USD&tsyms={symbol}"
    ))
    .await?
    .json()
    .await?;

    Ok(*mapping.get(symbol).unwrap())
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
