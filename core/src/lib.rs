use alloy_primitives::{Address, B256, Bytes, FixedBytes, U256, address};
use alloy_sol_macro::sol;
use alloy_sol_types::SolEventInterface;
use serde::{Deserialize, Serialize};

sol!(IContract, "abi/IContract.json");

pub const CONTRACT_ADDRESS: Address = address!("0x6b0aE12C4FD0dA4DFBEF4c2184b94D51eF7439D3");

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Trade {
    block_number: u64,
    id: U256,
    hash: B256,
}

impl Trade {
    pub fn decode(
        block_number: u64,
        address: &[u8; 20],
        topics: &[u8],
        data: &[u8],
    ) -> Option<Self> {
        if address != &CONTRACT_ADDRESS.0.0 {
            return None;
        }

        assert_eq!(topics.len() % 32, 0);

        let topics: Vec<_> = topics
            .chunks_exact(32)
            .map(|chunk| FixedBytes::new(chunk.try_into().expect("chunk is exactly 32 bytes")))
            .collect();

        let Ok(IContract::IContractEvents::Trade(IContract::Trade { trade_id, hash })) =
            IContract::IContractEvents::decode_raw_log(
                &topics,
                &Bytes::copy_from_slice(data),
                true,
            )
        else {
            return None;
        };

        Some(Self {
            block_number,
            id: trade_id,
            hash,
        })
    }

    pub fn block_number(&self) -> u64 {
        self.block_number
    }

    pub fn id(&self) -> U256 {
        self.id
    }

    pub fn hash(&self) -> B256 {
        self.hash
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Delay(pub u64);

impl std::fmt::Display for Delay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (divisor, unit) = match self.0 {
            d if d < 1_000 => return write!(f, "{}ns", self.0),
            d if d < 1_000_000 => (1_000.0, "us"),
            d if d < 1_000_000_000 => (1_000_000.0, "ms"),
            _ => (1_000_000_000.0, "s"),
        };

        write!(f, "{:.2}{}", self.0 as f64 / divisor, unit)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TradeQuery {
    pub block_number: u64,
    pub id: U256,
    pub hash: B256,
    pub user: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum TradeResult {
    Success { profit: f64 },
    TooSlow { delay: Delay },
    Invalid,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SseEvent {
    Snapshot { trades: Box<[SseEventTrade]> },
    Update(SseEventTrade),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SseEventTrade {
    pub user: String,
    pub profit: f64,
}
