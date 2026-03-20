use std::{error::Error, process, sync::Arc};

use alloy_primitives::U256;
use axum::{
    routing::{get, put},
    Router,
};
use clap::Parser;
use log::info;
use simplelog::{ColorChoice, CombinedLogger, Config, LevelFilter, TermLogger, TerminalMode};

use crate::state::{AppState, TradeHistory};

mod routes;
mod state;

#[derive(Debug, Parser)]
#[command(name = "trade-edge-server", about, long_about = None)]
struct Cli {
    #[arg(long, default_value_t = 8082)]
    port: u16,

    #[arg(long)]
    secret: String,

    #[arg(long, default_value_t = 0)]
    min_trade_id: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    if let Err(err) = CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Debug,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )]) {
        eprintln!("failed to initialise logger: {err}");
        process::exit(1);
    }

    let Cli {
        port,
        secret,
        min_trade_id,
    } = Cli::parse();

    let secret: U256 = U256::from_str_radix(&secret, 16)?;

    let app_state = Arc::new(AppState {
        secret,
        min_trade_id: U256::from(min_trade_id),

        history: std::sync::Mutex::new(TradeHistory::new()),
        broadcast: tokio::sync::broadcast::channel(64).0,
    });

    let router = Router::new()
        .route("/trade", put(routes::trade))
        .route("/trade/sse", get(routes::trade_sse))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
    info!("listening on 0.0.0.0:{port}");

    Ok(axum::serve(listener, router).await?)
}
