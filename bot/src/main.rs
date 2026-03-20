use std::error::Error;

use clap::{CommandFactory, FromArgMatches, Parser};
use simplelog::{ColorChoice, CombinedLogger, Config, LevelFilter, TermLogger, TerminalMode};

use self::{
    strategy::Strategy,
    strategy_ext::{ArbitrumPool, Token},
};

pub(crate) mod strategy;
pub(crate) mod strategy_ext;

#[derive(Debug, Parser)]
#[command(name = "trade-edge-bot", about, long_about = None)]
pub struct Cli {
    #[arg(long, default_value = "ws://localhost:8081")]
    monad_rpc_url: String,

    #[arg(long, default_value = "http://localhost:8082")]
    arbitrum_url: String,
}

fn main() {
    let mut cmd = Cli::command();

    CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Info,
        Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )])
    .unwrap_or_else(|err| {
        cmd.error(clap::error::ErrorKind::Io, err.to_string())
            .exit()
    });

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap_or_else(|e| cmd.error(clap::error::ErrorKind::Io, e).exit());

    if let Err(e) = runtime.block_on(run(cmd)) {
        panic!("trade-edge-bot crashed: {:?}", e);
    }
}

async fn run(mut cmd: clap::Command) -> Result<(), Box<dyn Error>> {
    let Cli {
        monad_rpc_url,
        arbitrum_url,
    } = Cli::from_arg_matches_mut(&mut cmd.get_matches_mut())?;

    let arbitrum_pool = ArbitrumPool::new(Token::Flash, Token::USDC);

    let strategy = Strategy::new(monad_rpc_url, arbitrum_url, arbitrum_pool);

    strategy.run().await
}
