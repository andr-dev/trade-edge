use std::io;

use clap::Parser;
use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use tokio::sync::mpsc;
use trade_edge_core::SseEvent;

mod animation;
mod app;
mod sse;
mod ui;

use animation::POLL_INTERVAL;
use app::App;

#[derive(Debug, Parser)]
#[command(name = "trade-edge-tui", about = "Trade Edge Live Dashboard")]
struct Cli {
    #[arg(long, default_value = "http://localhost:8082/trade/sse")]
    url: String,

    #[arg(long)]
    user: Option<String>,
}

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = io::stdout().execute(LeaveAlternateScreen);
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let cli = Cli::parse();

    let (sse_tx, mut sse_rx) = mpsc::unbounded_channel::<SseEvent>();

    tokio::spawn(async move {
        sse::sse_stream(cli.url, sse_tx).await;
    });

    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let _guard = TerminalGuard;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(cli.user);

    loop {
        terminal.draw(|f| ui::draw(f, &app))?;

        app.tick();

        while let Ok(event) = sse_rx.try_recv() {
            match event {
                SseEvent::Snapshot { trades } => app.apply_snapshot(trades),
                SseEvent::Update(trade) => app.apply_trade(trade),
            }
        }

        if event::poll(POLL_INTERVAL)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Left => app.prev_view(),
                        KeyCode::Right => app.next_view(),
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(())
}
