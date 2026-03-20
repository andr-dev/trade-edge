use std::collections::{HashMap, VecDeque};

use ratatui::prelude::Color;
use trade_edge_core::SseEventTrade;

use crate::animation::{AnimationState, RankDir};

const HISTORY_LEN: usize = 256;

#[derive(Clone, Copy, PartialEq)]
pub enum ViewMode {
    Cumulative,
    Recent,
}

impl ViewMode {
    pub fn prev(self) -> Self {
        match self {
            ViewMode::Cumulative => ViewMode::Cumulative,
            ViewMode::Recent => ViewMode::Cumulative,
        }
    }

    pub fn next(self) -> Self {
        match self {
            ViewMode::Cumulative => ViewMode::Recent,
            ViewMode::Recent => ViewMode::Recent,
        }
    }
}

pub struct UserStats {
    pub total_pnl: f64,
    pub trade_count: u32,
    pub cumulative_pnl: Vec<f64>,
}

impl UserStats {
    pub fn new() -> Self {
        Self {
            total_pnl: 0.0,
            trade_count: 0,
            cumulative_pnl: Vec::new(),
        }
    }

    pub fn record_trade(&mut self, profit: f64) {
        self.total_pnl += profit;
        self.trade_count += 1;
        self.cumulative_pnl.push(self.total_pnl);
    }

    pub fn carry_forward(&mut self) {
        self.cumulative_pnl.push(self.total_pnl);
    }
}

const USER_COLORS: &[Color] = &[
    Color::Cyan,
    Color::Yellow,
    Color::Green,
    Color::Magenta,
    Color::Red,
    Color::Blue,
    Color::LightCyan,
    Color::LightYellow,
    Color::LightGreen,
    Color::LightMagenta,
];

pub struct App {
    pub users: HashMap<String, UserStats>,
    pub trade_seq: usize,
    pub user_filter: Option<String>,
    pub color_map: HashMap<String, Color>,
    pub anim: AnimationState,
    pub view_mode: ViewMode,
    pub history: VecDeque<(String, f64)>,
}

impl App {
    pub fn new(user_filter: Option<String>) -> Self {
        Self {
            users: HashMap::new(),
            trade_seq: 0,
            user_filter,
            color_map: HashMap::new(),
            anim: AnimationState::new(),
            view_mode: ViewMode::Cumulative,
            history: VecDeque::new(),
        }
    }

    pub fn prev_view(&mut self) {
        self.view_mode = self.view_mode.prev();
    }

    pub fn next_view(&mut self) {
        self.view_mode = self.view_mode.next();
    }

    pub fn color_for(&mut self, user: &str) -> Color {
        let next_idx = self.color_map.len();
        *self
            .color_map
            .entry(user.to_string())
            .or_insert_with(|| USER_COLORS[next_idx % USER_COLORS.len()])
    }

    pub fn should_include(&self, user: &str) -> bool {
        match &self.user_filter {
            Some(filter) => filter == user,
            None => true,
        }
    }

    pub fn leaderboard(&self) -> Vec<(&str, &UserStats)> {
        let mut entries: Vec<(&str, &UserStats)> = self
            .users
            .iter()
            .map(|(name, stats)| (name.as_str(), stats))
            .collect();
        entries.sort_by(|a, b| b.1.total_pnl.total_cmp(&a.1.total_pnl));
        entries
    }

    /// Snapshot the current rank ordering and detect changes from the previous snapshot.
    fn update_ranks(&mut self) {
        let tick = self.anim.tick;

        let ranked: Vec<(String, usize)> = self
            .leaderboard()
            .into_iter()
            .enumerate()
            .map(|(i, (name, _))| (name.to_string(), i))
            .collect();

        for (name, i) in &ranked {
            if let Some(&prev_rank) = self.anim.prev_ranks.get(name.as_str()) {
                if *i < prev_rank {
                    self.anim
                        .rank_change
                        .insert(name.clone(), (RankDir::Up, tick));
                } else if *i > prev_rank {
                    self.anim
                        .rank_change
                        .insert(name.clone(), (RankDir::Down, tick));
                }
            }
        }

        self.anim.prev_ranks.clear();
        for (name, i) in ranked {
            self.anim.prev_ranks.insert(name, i);
        }
    }

    pub fn apply_trade(&mut self, trade: SseEventTrade) {
        if !self.should_include(&trade.user) {
            return;
        }

        self.trade_seq += 1;
        self.color_for(&trade.user);

        self.anim
            .flash_map
            .insert(trade.user.clone(), self.anim.tick);

        self.history.push_back((trade.user.clone(), trade.profit));
        if self.history.len() > HISTORY_LEN {
            self.history.pop_front();
        }

        self.users.entry(trade.user.clone()).or_insert_with(|| {
            let mut stats = UserStats::new();
            stats.cumulative_pnl.resize(self.trade_seq, 0.0);
            stats
        });

        for (name, stats) in &mut self.users {
            if *name == trade.user {
                stats.record_trade(trade.profit);
            } else {
                stats.carry_forward();
            }
        }

        self.update_ranks();
    }

    pub fn apply_snapshot(&mut self, trades: Box<[SseEventTrade]>) {
        for trade in trades.into_vec() {
            self.apply_trade(trade);
        }
    }

    pub fn tick(&mut self) {
        self.anim.advance();
    }
}
