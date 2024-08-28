#![allow(dead_code)]
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::ops::Range;

use crate::market::{GAME_TIME, MAX_SERIES_LENGTH};
use crate::mining_rig::MINING_RIG;
use crate::utils::{command_line_output, get_season, rand_from_range, truncate_price};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CryptoCoin {
    pub name: String,
    pub initial_price: f32,
    pub current_price: f32,
    pub volatility: Range<f32>,
    pub prices: Vec<f32>,
    pub trend: f32,
    pub trend_direction: VecDeque<bool>,
    pub active: bool,
    pub rug_pull: f32,
    pub index: usize,
    pub balance: f32,
    pub shares: f32,
    pub hashes: f32,
    pub hashes_per_share: f32,
    pub blocks: u32,
    pub shares_per_block: u32,
    pub max_blocks: u32,
    pub block_reward: f32,
    pub profit_factor: f32,
    pub berth_date: u32,
    pub death_date: Option<u32>,
    pub share_cooldown: i64,
}

impl CryptoCoin {
    pub fn new(
        name: &str,
        initial_price: f32,
        volatility: Range<f32>,
        index: usize,
        shares_per_block: u32,
        block_reward: f32,
        max_blocks: u32,
        hashes_per_share: f32,
        day: u32,
    ) -> Self {
        CryptoCoin {
            name: name.to_string(),
            initial_price,
            current_price: initial_price,
            volatility,
            prices: vec![initial_price],
            trend: 0.0,
            trend_direction: VecDeque::from(vec![false, false, false]),
            active: true,
            rug_pull: 0.0,
            index,
            balance: 0.0,
            shares: 0.0,
            hashes: 0.0,
            hashes_per_share,
            blocks: 0,
            shares_per_block,
            max_blocks,
            block_reward,
            profit_factor: 0.0,
            berth_date: day,
            death_date: None,
            share_cooldown: 0,
        }
    }

    pub fn get_share_cooldown(&self) -> i64 {
        self.share_cooldown
    }

    pub fn get_share_cooldown_seconds(&self) -> f32 {
        self.share_cooldown as f32 / 20.0
    }

    pub fn get_share_cooldown_ticks(&self) -> i64 {
        let rig_lvl = MINING_RIG().get_level();

        let ticks = match rig_lvl {
            // 1..=5 => 8 * (20 - rig_lvl),
            // 6..=10 => 7 * (20 - (rig_lvl - 5)),
            // 11..=15 => 6 * (20 - (rig_lvl - 10)),
            // 16..=20 => 5 * (20 - (rig_lvl - 15)),
            // 21..=25 => 4 * (20 - (rig_lvl - 20)),
            // 26..=30 => 3 * (20 - (rig_lvl - 25)),
            // 31..=35 => 2 * (20 - (rig_lvl - 30)),
            // 36..=40 => 1 * (20 - (rig_lvl - 35)),
            _ => 0,
        };

        ticks as i64
    }

    pub fn set_share_cooldown(&mut self) {
        let ticks = self.get_share_cooldown_ticks();

        self.share_cooldown = ticks;
    }

    pub fn decrement_share_cooldown(&mut self) {
        if self.share_cooldown > 0 {
            self.share_cooldown -= 1;
        }
    }

    pub fn get_share_progress(&self) -> f32 {
        self.hashes as f32 / self.hashes_per_share as f32
    }

    pub fn get_block_progress(&self) -> f32 {
        self.shares as f32 / self.shares_per_block as f32
    }

    pub fn get_difficulty(&self) -> f32 {
        self.current_price / 800.0
    }

    pub fn get_effective_hash(&self, hash_rate: u32) -> f32 {
        let total_hash = hash_rate as f32;
        let effective_hash = total_hash / (1.0 + self.get_difficulty());
        effective_hash
    }

    fn get_share_reward(&self, hash_rate: u32) -> f32 {
        let effective_hash = self.get_effective_hash(hash_rate);
        (self.block_reward / self.shares_per_block as f32)
            * (1.0 + (effective_hash as f32 / 10000.0))
    }

    pub fn calculate_rug_chance(&self) -> f32 {
        let age = self.get_age();
        let rug_chance = 0.01 * (age as f32 / 100.0).powf(2.0);
        rug_chance
    }

    fn calculate_shares_per_minute(&self, hash_rate: u32) -> f32 {
        let effective_hash: f32 = self.get_effective_hash(hash_rate);
        let hashes_per_call: f32 = effective_hash / 4.0;
        let cooldown_ticks: i64 = self.get_share_cooldown_ticks();
        let cooldown_seconds: f32 = cooldown_ticks as f32 / 20.0;
        let calls_per_share: f32 = self.hashes_per_share / hashes_per_call;

        let seconds_per_share: f32 = calls_per_share / 20.0;

        let minutes_per_share: f32 = (seconds_per_share + cooldown_seconds) / 60.0;
        let shares_per_minute: f32 = 1.0 / minutes_per_share;
        shares_per_minute
    }

    fn calculate_power_cost_per_minute(&self, day: u32) -> f64 {
        let cost_per_unit = 1.0 / get_season(day);
        let power_usage = MINING_RIG().get_power_usage() as f64;
        let usage_per_tick = power_usage / 20.0;
        let cost_per_tick = usage_per_tick * cost_per_unit;

        let cost_per_second = cost_per_tick * 20.0;
        let cost_per_minute = cost_per_second * 60.0;
        cost_per_minute
    }

    pub fn calculate_profit_factor(&mut self, hash_rate: u32) -> f32 {
        let spm = self.calculate_shares_per_minute(hash_rate);
        let coins_share = self.get_share_reward(hash_rate);
        (spm * coins_share) * self.current_price
    }

    pub fn hash_coin(&mut self, hash_rate: u32) {
        let share_cooldown = self.get_share_cooldown() != 0;

        if self.blocks >= self.max_blocks || share_cooldown || !self.active {
            return;
        }

        let effective_hash = self.get_effective_hash(hash_rate);
        self.hashes += effective_hash / 6.0;

        while self.hashes >= self.hashes_per_share {
            self.set_share_cooldown();

            let fail_chance = rand_from_range(0.0..1.0);

            if fail_chance < 0.01 {
                self.hashes -= self.hashes_per_share;
                let msg = format!("Share rejected for {}, boo!", self.name);
                command_line_output(&msg);
                continue;
            }

            self.shares += 1.0;

            let msg = format!("Share accepted for {}, yay!", self.name);
            command_line_output(&msg);

            self.hashes -= self.hashes_per_share;
            self.balance += self.get_share_reward(hash_rate);

            if self.shares as u32 >= self.shares_per_block {
                self.blocks += 1;

                let msg = format!("Block mined for {}, yay!", self.name);
                command_line_output(&msg);

                self.shares -= self.shares_per_block as f32;

                // 25% bonus for completing a block
                self.balance += self.block_reward * 0.25;
            }
        }
    }

    pub fn get_age(&self) -> u32 {
        if self.death_date.is_some() {
            return self.death_date.unwrap() - self.berth_date;
        }
        GAME_TIME().day - self.berth_date
    }

    pub fn update_price(&mut self) {
        let starting_price = self.current_price;

        // Encourage a trend correction if the trend is too strong
        let trend_adjustment = if self.trend_direction.clone().into_iter().all(|x| x == true) {
            rand_from_range(-0.03..0.001)
        } else if self.trend_direction.clone().into_iter().all(|x| x == false) {
            rand_from_range(-0.001..0.03)
        } else {
            rand_from_range(-0.003..0.003)
        };
        self.trend += trend_adjustment;

        // Market sentiment factor
        let sentiment_factor = -0.02..0.02;
        let sentiment = rand_from_range(sentiment_factor);
        self.trend += sentiment;

        // Periodic sawtooth pattern
        let period = 30; // Number of updates for a full cycle
        let position = (self.prices.len() % period) as f32;
        let sawtooth = (position / period as f32) - 0.5; // Range from -0.5 to 0.5

        // Combine sawtooth with random change and trend
        let change_percent =
            sawtooth * 0.05 + rand_from_range(self.volatility.clone()) + self.trend;

        // Random events with variable impact
        if rand_from_range(0.0..1.0) < 0.01 {
            let event = rand_from_range(-0.1..0.1);
            self.current_price *= 1.0 + event;
        } else {
            self.current_price *= 1.0 + change_percent;
        }

        // Seasonality effect
        let seasonality = 0.01 * (self.prices.len() as f32 / 10.0).sin()
            + 0.005 * (self.prices.len() as f32 / 50.0).cos();
        self.current_price *= 1.0 + seasonality;

        // Introduce news impact
        if rand_from_range(0.0..1.0) < 0.015 {
            let news_impact = rand_from_range(-0.05..0.05);
            self.current_price *= 1.0 + news_impact;
        }

        // Clamp price to prevent excessive growth or decline
        if self.current_price > 100_000.0 {
            // Limit to 3% growth
            if (self.current_price - starting_price) / starting_price > 0.03 {
                self.current_price = starting_price * 1.03;
            }
        }

        // Limit losses if price is less than 0.05
        if self.current_price < 0.05 {
            // Limit to 4% loss
            if (self.current_price - starting_price) / starting_price < -0.04 {
                self.current_price = starting_price * 0.96;
            }
        }

        self.current_price = truncate_price(self.current_price);

        self.prices.push(self.current_price);

        if self.prices.len() > MAX_SERIES_LENGTH {
            self.prices.remove(0);
        }

        self.trend_direction
            .push_front(self.current_price > starting_price);

        if self.trend_direction.len() > 4 {
            self.trend_direction.pop_back();
        }
    }
}
