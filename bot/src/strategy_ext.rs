use std::{cell::Cell, error::Error};

use alloy::{pubsub::Subscription, rpc::types::Log};
use monad_exec_events::ExecEventReader;
use serde::Deserialize;
use trade_edge_core::{Trade, TradeQuery, TradeResult};

use crate::{strategy::USER, Strategy};

pub fn get_user() -> String {
    std::env::var("TRADE_EDGE_USER")
        .ok()
        .unwrap_or_else(|| USER.to_string())
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonadNotification<T> {
    pub commit_state: String,
    #[serde(flatten)]
    pub data: T,
}

/// Hello!
///
/// This file is meant to "hide" some of the inner workings of the demo.
///
/// *** Please don't modify it ***
///
/// The orchestration of the polling is meant to give everyone an equal opportunity without
/// destroying the server, we're using a shared resource so please share it nicely :)

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Token {
    #[allow(clippy::upper_case_acronyms)]
    USDC,
    Flash,
}

#[derive(Debug)]
pub struct ArbitrumPool {
    #[allow(unused)]
    token0: Token,
    #[allow(unused)]
    token1: Token,
    client: reqwest::Client,
}

impl ArbitrumPool {
    pub fn new(token0: Token, token1: Token) -> Self {
        Self {
            token0,
            token1,
            client: reqwest::Client::new(),
        }
    }

    pub fn is_unbalanced(&self, _: &Trade) -> bool {
        true
    }

    pub async fn rebalance(
        &self,
        arbitrum_url: &str,
        trade: Trade,
    ) -> Result<TradeResult, Box<dyn Error>> {
        let url = format!("{arbitrum_url}/trade");

        let query = TradeQuery {
            block_number: trade.block_number(),
            id: trade.id(),
            hash: trade.hash(),
            user: get_user(),
        };

        Ok(self
            .client
            .put(&url)
            .query(&query)
            .send()
            .await?
            .error_for_status()?
            .json::<TradeResult>()
            .await?)
    }
}

thread_local! {
    static POLL_CALL_COUNT: Cell<u64> = const { Cell::new(0) };
}
const POLL_REPEAT_COUNT: u64 = 64 * 1024;

impl Strategy {
    #[allow(unused)]
    pub async fn poll_trade_using_ws(
        monad_ws_subscription: &mut Subscription<MonadNotification<Log>>,
    ) -> Result<Option<Trade>, Box<dyn Error>> {
        let Some(trade) = crate::strategy::poll_trade_using_ws(monad_ws_subscription).await? else {
            // Please don't remove this, we only have so many cores on the machine :pray:
            let n = POLL_CALL_COUNT.with(|c| {
                let next = c.get().wrapping_add(1);
                c.set(next);
                next
            });
            if n % POLL_REPEAT_COUNT == 0 {
                tokio::time::sleep(tokio::time::Duration::from_micros(1)).await;
            }
            return Ok(None);
        };

        Ok(Some(trade))
    }

    #[allow(unused)]
    pub async fn poll_trade_using_events(
        event_reader: &mut ExecEventReader<'_>,
    ) -> Result<Option<Trade>, Box<dyn Error>> {
        let Some(trade) = crate::strategy::poll_trade_using_events(event_reader) else {
            // Please don't remove this, we only have so many cores on the machine :pray:
            let n = POLL_CALL_COUNT.with(|c| {
                let next = c.get().wrapping_add(1);
                c.set(next);
                next
            });
            if n % POLL_REPEAT_COUNT == 0 {
                tokio::time::sleep(tokio::time::Duration::from_micros(1)).await;
            }
            return Ok(None);
        };

        Ok(Some(trade))
    }
}
