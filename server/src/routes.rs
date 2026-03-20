use std::{collections::HashSet, convert::Infallible, sync::Arc};

use alloy_primitives::{keccak256, B256, U256};
use axum::{
    extract::{Query, State},
    response::{
        sse::{Event, KeepAlive},
        Sse,
    },
    Json,
};
use futures::Stream;
use log::{debug, info, warn};
use rand::Rng;
use rand_distr::StudentT;
use tokio::time::Instant;
use tokio_stream::{wrappers::BroadcastStream, StreamExt};
use trade_edge_core::{Delay, SseEvent, SseEventTrade, TradeQuery, TradeResult};

use crate::state::{AppState, ClaimedTrade};

fn check_response(secret: U256, block_number: u64, id: U256, hash: B256) -> bool {
    let result = B256::from(id) & B256::from(U256::from(block_number)) & B256::from(secret);
    let value = keccak256(result);

    value == hash
}

fn random_profit() -> f64 {
    let t_dist = StudentT::new(5.0).expect("valid degrees of freedom");
    let mut rng = rand::thread_rng();
    let sample: f64 = rng.sample(t_dist);

    let sample_raw = (-2.6 + 0.8 * sample).exp().max(0.01);
    let raw = ((10.0 * (1.0 - (-sample_raw / 10.0_f64).exp()) * 100.0).round() / 100.0).max(0.01);

    if rng.gen::<f64>() < 0.25 {
        (-raw / 2.0).min(-0.01)
    } else {
        raw
    }
}

pub(super) async fn trade(
    State(state): State<Arc<AppState>>,
    Query(query): Query<TradeQuery>,
) -> Json<TradeResult> {
    let TradeQuery {
        block_number,
        id,
        hash,
        user,
    } = query;

    if id < state.min_trade_id {
        warn!("trade {} below min trade id {}", id, state.min_trade_id);
        return Json(TradeResult::Invalid);
    }

    if !check_response(state.secret, block_number, id, hash) {
        warn!("invalid trade response");
        return Json(TradeResult::Invalid);
    }

    let mut history = state.history.lock().unwrap_or_else(|e| e.into_inner());

    if let Some(&idx) = history.id_to_idx.get(&id) {
        let existing = &history.trades[idx];
        debug!(
            "trade {} already claimed by {}, late from {}",
            id, existing.user, user
        );
        return Json(TradeResult::TooSlow {
            delay: Delay(existing.time.elapsed().as_nanos() as u64),
        });
    }

    let profit = random_profit();

    info!("trade {} claimed by {} — profit ${:.2}", id, user, profit);

    let claimed = ClaimedTrade {
        id,
        user: user.clone(),
        time: Instant::now(),
        profit,
    };

    let idx = history.trades.len();
    history.trades.push(claimed);
    history.id_to_idx.insert(id, idx);
    drop(history);

    let _ = state.broadcast.send((id, SseEventTrade { user, profit }));

    Json(TradeResult::Success { profit })
}

pub(super) async fn trade_sse(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = state.broadcast.subscribe();

    let claimed = {
        let history = state.history.lock().unwrap_or_else(|e| e.into_inner());
        history.trades.clone()
    };

    let mut snapshot_ids: HashSet<_> = claimed.iter().map(|t| t.id).collect();
    let snapshot: Box<[_]> = claimed
        .iter()
        .map(|t| SseEventTrade {
            user: t.user.clone(),
            profit: t.profit,
        })
        .collect();

    info!(
        "sse client connected, snapshot has {} claimed trades",
        snapshot.len()
    );

    let snapshot_stream = tokio_stream::once(Ok(Event::default()
        .json_data(SseEvent::Snapshot { trades: snapshot })
        .expect("snapshot serializes to JSON")));

    let update_stream = BroadcastStream::new(rx)
        .map_while(Result::ok)
        .filter_map(move |(id, event)| snapshot_ids.insert(id).then_some(event))
        .map(|event| {
            Ok(Event::default()
                .json_data(SseEvent::Update(event))
                .expect("event serializes to JSON"))
        });

    Sse::new(snapshot_stream.chain(update_stream)).keep_alive(KeepAlive::default())
}
