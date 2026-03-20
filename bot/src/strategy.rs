use std::{cell::Cell, error::Error};

use alloy::{
    providers::{Provider, ProviderBuilder, RootProvider},
    pubsub::Subscription,
    rpc::{
        client::WsConnect,
        types::{Filter, Log},
    },
};
use itertools::{Either, Itertools};
use log::{error, info};
#[allow(unused)]
use monad_event_ring::*;
use monad_exec_events::ffi::monad_exec_block_start;
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

        let event_ring_path = EventRingPath::resolve(DEFAULT_FILE_NAME)?;
        let event_ring = EventRing::new(event_ring_path)?;
        let mut event_reader = event_ring.create_reader();

        loop {
            let Some(trade) = Self::poll_trade_using_events(&mut event_reader).await? else {
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

thread_local! {
    static CURRENT_BLOCK_NUMBER: Cell<Option<u64>> = const { Cell::new(None) };
}

pub fn poll_trade_using_events(event_reader: &mut ExecEventReader) -> Option<Trade> {
    loop {
        let event_descriptor = match event_reader.next_descriptor() {
            EventNextResult::Gap => panic!("gap"),
            EventNextResult::NotReady => return None,
            EventNextResult::Ready(event_descriptor) => event_descriptor,
        };

        // CURRENT_BLOCK_NUMBER is a thread-local used to pass block_number into the fn
        // pointer below without capturing. try_filter_map requires a fn pointer (not a
        // closure) to enforce purity over the zero-copy event buffer.
        match event_descriptor.try_filter_map(|event| match event {
            ExecEventRef::BlockStart(monad_exec_block_start {
                eth_block_input, ..
            }) => Some(Either::Left(eth_block_input.number)),
            ExecEventRef::TxnLog {
                txn_log,
                topic_bytes,
                data_bytes,
                ..
            } => CURRENT_BLOCK_NUMBER
                .get()
                .and_then(|block_number| {
                    Trade::decode(
                        block_number,
                        &txn_log.address.bytes,
                        topic_bytes,
                        data_bytes,
                    )
                })
                .map(Either::Right),
            _ => None,
        }) {
            EventPayloadResult::Expired => panic!("expired"),
            EventPayloadResult::Ready(None) => {}
            EventPayloadResult::Ready(Some(Either::Left(new_block_number))) => {
                CURRENT_BLOCK_NUMBER.set(Some(new_block_number));
            }
            EventPayloadResult::Ready(Some(Either::Right(trade))) => {
                return Some(trade);
            }
        };
    }
}
