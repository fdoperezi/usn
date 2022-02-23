use near_sdk::Timestamp;

use crate::oracle::priceoracle::{ext_priceoracle, PriceData};
use crate::*;

struct OracleConfig {
    pub oracle_address: &'static str,
    pub asset_id: &'static str,
    pub gas: Gas,
}

const CONFIG: OracleConfig = if cfg!(feature = "mainnet") {
    OracleConfig {
        oracle_address: "priceoracle.near",
        asset_id: "wrap.near", // NEARUSDT
        gas: Gas(5_000_000_000_000),
    }
} else if cfg!(feature = "testnet") {
    OracleConfig {
        oracle_address: "priceoracle.testnet",
        asset_id: "wrap.testnet", // NEARUSDT
        gas: Gas(5_000_000_000_000),
    }
} else {
    OracleConfig {
        oracle_address: "priceoracle.test.near",
        asset_id: "wrap.test.near",
        gas: Gas(5_000_000_000_000),
    }
};

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct ExchangeRate {
    multiplier: u128,
    decimals: u8,
    timestamp: Timestamp,
    recency_duration: Timestamp,
}

impl ExchangeRate {
    pub fn multiplier(&self) -> u128 {
        self.multiplier
    }

    pub fn decimals(&self) -> u8 {
        self.decimals
    }
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Oracle {
    pub last_report: Option<ExchangeRate>,
}

impl Default for Oracle {
    fn default() -> Self {
        Self { last_report: None }
    }
}

impl Oracle {
    pub fn set_exchange_rate(&mut self, exchange_rate: ExchangeRate) {
        self.last_report = Some(exchange_rate);
    }

    pub fn get_exchange_rate(&self) -> PromiseOrValue<ExchangeRate> {
        // Re-use the cached price (exchange rate) of the token while it's valid.
        if let Some(price) = &self.last_report {
            if env::block_timestamp() < price.timestamp + price.recency_duration {
                return PromiseOrValue::Value(price.clone());
            }
        }

        // Else, request fresh price data, extracting a pure token price.
        PromiseOrValue::Promise(ext_priceoracle::get_price_data(
            vec![CONFIG.asset_id.into()],
            CONFIG.oracle_address.parse().unwrap(),
            0,
            CONFIG.gas,
        ))
    }
}

impl From<PriceData> for ExchangeRate {
    fn from(price_data: PriceData) -> Self {
        let price = price_data.price(&CONFIG.asset_id.into());

        if env::block_timestamp() >= price_data.timestamp() + price_data.recency_duration() {
            env::panic_str("Oracle provided an outdated price data");
        }

        let exchange_rate = ExchangeRate {
            multiplier: price.multiplier.into(),
            decimals: price.decimals,
            timestamp: price_data.timestamp(),
            recency_duration: price_data.recency_duration(),
        };

        exchange_rate
    }
}

#[cfg(test)]
impl ExchangeRate {
    pub fn test_fresh_rate() -> Self {
        Self {
            multiplier: 111439,
            decimals: 28,
            timestamp: env::block_timestamp(),
            recency_duration: env::block_timestamp() + 1000000000,
        }
    }

    pub fn test_old_rate() -> Self {
        Self {
            multiplier: 111439,
            decimals: 28,
            timestamp: env::block_timestamp(),
            recency_duration: env::block_timestamp(),
        }
    }
}
