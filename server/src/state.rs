use std::{collections::HashMap, sync::Mutex};

use alloy_primitives::U256;
use tokio::time::Instant;
use trade_edge_core::SseEventTrade;

#[derive(Clone, Debug)]
pub(crate) struct ClaimedTrade {
    pub(crate) id: U256,
    pub(crate) user: String,
    pub(crate) time: Instant,
    pub(crate) profit: f64,
}

pub(crate) struct TradeHistory {
    pub(crate) trades: Vec<ClaimedTrade>,
    pub(crate) id_to_idx: HashMap<U256, usize>,
}

impl TradeHistory {
    pub(crate) fn new() -> Self {
        Self {
            trades: Vec::new(),
            id_to_idx: HashMap::new(),
        }
    }
}

pub(crate) struct AppState {
    pub(crate) secret: U256,
    pub(crate) min_trade_id: U256,

    pub(crate) history: Mutex<TradeHistory>,
    pub(crate) broadcast: tokio::sync::broadcast::Sender<(U256, SseEventTrade)>,
}
