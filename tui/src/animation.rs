use std::{collections::HashMap, time::Duration};

pub const POLL_INTERVAL: Duration = Duration::from_millis(50);
pub const FLASH_DURATION: u64 = 20; // ~1 s at 50ms poll (~20 fps)
pub const RANK_ARROW_DURATION: u64 = 30; // ~1.5 s

#[derive(Clone, Copy)]
pub enum RankDir {
    Up,
    Down,
}

pub struct AnimationState {
    pub tick: u64,
    /// user -> tick when they last received a trade
    pub flash_map: HashMap<String, u64>,
    /// user -> (direction, tick when rank changed)
    pub rank_change: HashMap<String, (RankDir, u64)>,
    /// previous frame's rank ordering (user -> rank index)
    pub prev_ranks: HashMap<String, usize>,
}

impl AnimationState {
    pub fn new() -> Self {
        Self {
            tick: 0,
            flash_map: HashMap::new(),
            rank_change: HashMap::new(),
            prev_ranks: HashMap::new(),
        }
    }

    pub fn advance(&mut self) {
        self.tick += 1;
    }

    pub fn flash_intensity(&self, user: &str) -> f64 {
        match self.flash_map.get(user) {
            Some(&t) if self.tick >= t => {
                let elapsed = self.tick - t;
                if elapsed < FLASH_DURATION {
                    1.0 - (elapsed as f64 / FLASH_DURATION as f64)
                } else {
                    0.0
                }
            }
            _ => 0.0,
        }
    }

    pub fn rank_arrow(&self, user: &str) -> Option<(RankDir, f64)> {
        let &(dir, t) = self.rank_change.get(user)?;
        if self.tick < t {
            return None;
        }
        let elapsed = self.tick - t;
        if elapsed < RANK_ARROW_DURATION {
            let intensity = 1.0 - (elapsed as f64 / RANK_ARROW_DURATION as f64);
            Some((dir, intensity))
        } else {
            None
        }
    }
}
