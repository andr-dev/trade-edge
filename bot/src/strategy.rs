use std::error::Error;

use alloy::{
    providers::{Provider, ProviderBuilder, RootProvider},
    pubsub::Subscription,
    rpc::{
        client::WsConnect,
        types::{Filter, Log},
    },
};
use itertools::Itertools;
use log::{error, info};
#[allow(unused)]
use monad_event_ring::*;
#[allow(unused)]
use monad_exec_events::{ffi::DEFAULT_FILE_NAME, *};
use tokio::sync::broadcast::error::TryRecvError;
use trade_edge_core::{Trade, TradeResult, CONTRACT_ADDRESS};

use crate::strategy_ext::{get_user, ArbitrumPool, MonadNotification};

//---------/
// STEP #1 /
//---------/
//
// Update your username!

pub const USER: &str = "";

//---------/
// STEP #2 /
//---------/
//
// Press ctrl + ` (tilde) to open the terminal and run:
// > make money
//
// To view the trading dashboard run:
// > make dash

//---------/
// STEP #3 /
//---------/
//
// Wait for Ken to teach you how to use the execution events rust SDK!

pub struct Strategy {
    monad_rpc_url: String,
    arbitrum_url: String,
    arbitrum_pool: ArbitrumPool,
}

impl Strategy {
    pub fn new(monad_rpc_url: String, arbitrum_url: String, arbitrum_pool: ArbitrumPool) -> Self {
        assert!(!get_user().is_empty(), "Please specify your username");
        Self {
            monad_rpc_url,
            arbitrum_pool,
            arbitrum_url,
        }
    }

    pub async fn run(self) -> Result<(), Box<dyn Error>> {
        let Self {
            monad_rpc_url,
            arbitrum_url,
            arbitrum_pool,
        } = self;

        let monad_rpc_provider: RootProvider<_, alloy::network::Ethereum> =
            ProviderBuilder::default()
                .on_ws(WsConnect::new(monad_rpc_url))
                .await?;

        let mut monad_ws_subscription: Subscription<MonadNotification<Log>> = monad_rpc_provider
            .subscribe(("monadLogs", Filter::default().address(CONTRACT_ADDRESS)))
            .await?;

        loop {
            let Some(trade) = Self::poll_trade_using_ws(&mut monad_ws_subscription).await? else {
                continue;
            };

            let tid = trade.id();

            info!("[strategy][tid: {tid}] detected trade");

            if !arbitrum_pool.is_unbalanced(&trade) {
                continue;
            }

            info!("[strategy][tid: {tid}] executing pool rebalance");

            let trade_result = arbitrum_pool.rebalance(&arbitrum_url, trade).await?;

            match trade_result {
                TradeResult::Invalid => {
                    error!("[strategy][tid: {tid}] invalid trade attempt: {trade:?}");
                }
                TradeResult::Success { profit } => {
                    info!("[strategy][tid: {tid}] TRADE EXECUTED ✅ PROFIT: ${profit:.2}");
                }
                TradeResult::TooSlow { delay } => {
                    info!("[strategy][tid: {tid}] TRADE MISSED ❌ by {delay}");
                }
            }
        }
    }
}

pub async fn poll_trade_using_ws(
    monad_ws_subscription: &mut Subscription<MonadNotification<Log>>,
) -> Result<Option<Trade>, Box<dyn Error>> {
    loop {
        let notification = match monad_ws_subscription.try_recv() {
            Err(TryRecvError::Closed) => return Err("ws closed".into()),
            Err(TryRecvError::Empty) => return Ok(None),
            Err(TryRecvError::Lagged(lag)) => return Err(format!("ws lagged by {lag}").into()),
            Ok(notification) => notification,
        };

        if notification.commit_state != "Proposed" {
            continue;
        }

        let log = notification.data;

        let Some(trade) = Trade::decode(
            log.block_number.ok_or("ws block number missing")?,
            &log.address().0 .0,
            &log.inner.topics().iter().flat_map(|x| x.0).collect_vec(),
            &log.inner.data.data,
        ) else {
            error!("[strategy] failed to decode trade");
            continue;
        };

        return Ok(Some(trade));
    }
}

pub fn poll_trade_using_events(event_reader: &mut ExecEventReader) -> Option<Trade> {
    todo!()
}
