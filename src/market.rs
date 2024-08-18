#![allow(dead_code)]
use dioxus::prelude::*;
use gloo_utils::window;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::ops::Range;
use wasm_bindgen::JsCast;

use crate::i_db::Selection;
use crate::utils::{command_line_output, get_season, rand_from_range, GameTime};

pub const MAX_SERIES_LENGTH: usize = 96;
pub static MINING_RIG: GlobalSignal<MiningRig> = Signal::global(|| MiningRig::new());
pub static MARKET: GlobalSignal<Market> = Signal::global(|| Market::new());
pub static SELECTION: GlobalSignal<Selection> = Signal::global(|| Selection::default());
pub static GAME_TIME: GlobalSignal<GameTime> = Signal::global(|| GameTime::new());

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AutoPowerFill {
    pub level: u32,
    pub active: bool,
    pub refill_time: Option<i64>,
}

impl AutoPowerFill {
    pub fn new() -> Self {
        AutoPowerFill {
            level: 1,
            active: true,
            refill_time: None,
        }
    }

    pub fn decrement_refill_time(&mut self) {
        self.refill_time = self.refill_time.map(|time| time - 1);
    }

    pub fn set_refill_time(&mut self, time: Option<i64>) {
        self.refill_time = time;
    }

    pub fn toggle_active(&mut self) {
        self.active = !self.active;
    }

    pub fn upgrade(&mut self) {
        self.level += 1;
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct RugProtection {
    pub level: u32,
    pub active: bool,
}

impl RugProtection {
    pub fn new() -> Self {
        RugProtection {
            level: 1,
            active: false,
        }
    }

    pub fn upgrade(&mut self) {
        if !self.active {
            self.active = true;
            return;
        }
        self.level += 1;
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Bank {
    pub balance: f64,
}

impl Bank {
    pub fn new() -> Self {
        Bank { balance: 0.0 }
    }

    pub fn deposit(&mut self, amount: f64) {
        self.balance += amount;
    }

    pub fn withdraw(&mut self, amount: f64) -> bool {
        if self.balance >= amount {
            self.balance -= amount;
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MiningRig {
    pub level: u32,
    pub power_capacity: f32,
    pub available_power: f32,
    pub cpu_slot: CpuSlot,
    pub gpu_slot: GpuSlot,
    pub asic_slot: AsicSlot,
    pub click_power: u32,
    pub max_gpu_slots: u32,
    pub max_asic_slots: u32,
    pub max_click_power: u32,
    pub cpu_upgrade_level: u32,
    pub gpu_upgrade_level: u32,
    pub asic_upgrade_level: u32,
    pub auto_power_fill: Option<AutoPowerFill>,
    pub rug_protection: RugProtection,
}

impl MiningRig {
    pub fn new() -> Self {
        MiningRig {
            level: 1,
            power_capacity: 500.0,
            available_power: 0.0,
            cpu_slot: CpuSlot::new(1),
            gpu_slot: GpuSlot::new(1),
            asic_slot: AsicSlot::new(1),
            click_power: 1,
            max_gpu_slots: 0,
            max_asic_slots: 0,
            max_click_power: 10,
            cpu_upgrade_level: 1,
            gpu_upgrade_level: 1,
            asic_upgrade_level: 1,
            auto_power_fill: None,
            rug_protection: RugProtection::new(),
        }
    }

    pub fn get_rug_protection_level(&self) -> u32 {
        if !self.rug_protection.active {
            return 0;
        }
        self.rug_protection.level
    }

    pub fn get_rug_protection_active(&self) -> bool {
        self.rug_protection.active
    }

    pub fn get_rug_protection_amount(&self) -> f32 {
        if self.level < 10 || !self.rug_protection.active {
            return 0.0;
        }

        match self.rug_protection.level {
            0 => 0.0,
            1..=3 => 0.1 + ((self.rug_protection.level - 1) as f32 * 0.02),
            4..=6 => 0.15 + ((self.rug_protection.level - 4) as f32 * 0.02),
            7..=9 => 0.20 + ((self.rug_protection.level - 7) as f32 * 0.02),
            10..=12 => 0.25 + ((self.rug_protection.level - 10) as f32 * 0.02),
            13..=15 => 0.30 + ((self.rug_protection.level - 13) as f32 * 0.02),
            16..=18 => 0.35 + ((self.rug_protection.level - 16) as f32 * 0.02),
            19..=21 => 0.40 + ((self.rug_protection.level - 19) as f32 * 0.02),
            22..=24 => 0.45 + ((self.rug_protection.level - 22) as f32 * 0.02),
            25..=27 => 0.50 + ((self.rug_protection.level - 25) as f32 * 0.02),
            28..=30 => 0.55 + ((self.rug_protection.level - 28) as f32 * 0.02),
            31..=33 => 0.60 + ((self.rug_protection.level - 31) as f32 * 0.02),
            34..=36 => 0.65 + ((self.rug_protection.level - 34) as f32 * 0.02),
            37..=40 => 0.70 + ((self.rug_protection.level - 37) as f32 * 0.02),
            41..=45 => 0.75 + ((self.rug_protection.level - 41) as f32 * 0.01),
            46..=50 => 0.80 + ((self.rug_protection.level - 46) as f32 * 0.01),
            51..=55 => 0.85 + ((self.rug_protection.level - 51) as f32 * 0.01),
            56..=60 => 0.90 + ((self.rug_protection.level - 56) as f32 * 0.01),
            61..=64 => 0.95 + ((self.rug_protection.level - 61) as f32 * 0.01),
            _ => 1.0,
        }
    }

    pub fn get_rug_protection_upgrade_cost(&self) -> f64 {
        let rug_protection_level = self.get_rug_protection_level();

        let rug_protection_active = self.get_rug_protection_active();

        if !rug_protection_active {
            return 5000.0;
        }

        match rug_protection_level {
            1..=3 => 100.0,
            4..=6 => 250.0,
            7..=9 => 500.0,
            10..=12 => 1000.0,
            13..=15 => 2500.0,
            16..=18 => 5000.0,
            19..=21 => 10_000.0,
            22..=24 => 25_000.0,
            25..=27 => 50_000.0,
            28..=30 => 100_000.0,
            31..=33 => 250_000.0,
            34..=36 => 500_000.0,
            37..=40 => 1_000_000.0,
            41..=45 => 2_500_000.0,
            46..=50 => 5_000_000.0,
            51..=55 => 10_000_000.0,
            56..=60 => 25_000_000.0,
            61..=64 => 50_000_000.0,
            _ => 100_000_000.0,
        }
    }

    pub fn get_global_share_cooldown(&self) -> bool {
        !self.rug_protection.active
    }

    pub fn upgrade_rug_protection(&mut self) {
        if self.level < 10 {
            return;
        }
        self.rug_protection.upgrade();
    }

    pub fn get_level(&self) -> u32 {
        self.level
    }

    pub fn consume_power(&mut self) -> bool {
        let power_usage_watts = (self.get_power_usage() as f32) / 20.0;

        if self.available_power >= power_usage_watts {
            self.available_power -= power_usage_watts;
            true
        } else {
            // Not enough power to run the rig
            false
        }
    }

    pub fn decrement_auto_power_refill_time(&mut self) {
        if let Some(auto_power_fill) = &mut self.auto_power_fill {
            auto_power_fill.decrement_refill_time();
        }
    }

    pub fn get_auto_power_refill_time(&self) -> Option<i64> {
        if let Some(auto_power_fill) = &self.auto_power_fill {
            auto_power_fill.refill_time
        } else {
            None
        }
    }

    pub fn set_auto_power_refill_time(&mut self, time: Option<i64>) {
        if let Some(auto_power_fill) = &mut self.auto_power_fill {
            auto_power_fill.set_refill_time(time);
        }
    }

    pub fn get_auto_power_fill_active(&self) -> bool {
        if let Some(auto_power_fill) = &self.auto_power_fill {
            auto_power_fill.active
        } else {
            false
        }
    }

    pub fn toggle_auto_power_fill(&mut self) {
        if let Some(auto_power_fill) = &mut self.auto_power_fill {
            auto_power_fill.toggle_active();
        } else {
            self.auto_power_fill = Some(AutoPowerFill::new());
        }
    }

    pub fn get_auto_power_fill_level(&self) -> u32 {
        if let Some(auto_power_fill) = &self.auto_power_fill {
            auto_power_fill.level
        } else {
            0
        }
    }

    fn power_capacity(&self) -> f32 {
        self.get_power_usage() as f32 * 40.0
    }

    pub fn get_auto_power_fill_cost(&self, day: u32) -> f64 {
        let refill_cost = self.power_capacity() as f64 / get_season(day);
        refill_cost * ((1.0 + self.get_auto_fill_fee()) * self.get_auto_power_fill_amount()) as f64
    }

    pub fn get_available_power(&self) -> f32 {
        self.available_power
    }

    pub fn get_power_capacity(&self) -> f32 {
        self.power_capacity()
    }

    pub fn get_auto_power_fill_delay(&self) -> u32 {
        let auto_fill_level = self.get_auto_power_fill_level();
        match auto_fill_level {
            1..=3 => 30_000 / 50,
            4..=6 => (25_000 - (auto_fill_level - 4) * 2_000) / 50,
            7..=9 => (20_000 - (auto_fill_level - 7) * 2_000) / 50,
            10..=12 => (15_000 - (auto_fill_level - 10) * 1_500) / 50,
            13..=15 => (10_000 - (auto_fill_level - 13) * 1_500) / 50,
            16..=18 => (5_000 - (auto_fill_level - 16) * 1_250) / 50,
            19..=21 => (2_500 - (auto_fill_level - 19) * 750) / 50,
            22..=24 => (1_000 - (auto_fill_level - 22) * 500) / 50,
            _ => 0,
        }
    }

    pub fn upgrade_auto_power_fill(&mut self) {
        if let Some(auto_power_fill) = &mut self.auto_power_fill {
            auto_power_fill.upgrade();
        } else {
            self.auto_power_fill = Some(AutoPowerFill::new());
        }
    }

    pub fn get_auto_power_fill_upgrade_cost(&self) -> f64 {
        let auto_fill_level = self.get_auto_power_fill_level();

        let cost = match auto_fill_level {
            ..=3 => 100,
            4..=6 => 250,
            7..=9 => 500,
            10..=12 => 1000,
            13..=15 => 2500,
            16..=18 => 5000,
            19..=21 => 10_000,
            22..=24 => 25_000,
            25..=27 => 50_000,
            28..=30 => 100_000,
            31..=33 => 250_000,
            34..=36 => 500_000,
            37..=40 => 1_000_000,
            _ => 0,
        };

        cost as f64
    }

    pub fn get_auto_power_fill_amount(&self) -> f32 {
        let auto_fill_level = self.get_auto_power_fill_level();
        match auto_fill_level {
            1..=3 => 0.25 + ((auto_fill_level - 1) as f32 * 0.02),
            4..=6 => 0.33 + ((auto_fill_level - 4) as f32 * 0.03),
            7..=9 => 0.50 + ((auto_fill_level - 7) as f32 * 0.05),
            10..=12 => 0.75 + ((auto_fill_level - 10) as f32 * 0.08),
            _ => 1.0,
        }
        .min(1.0)
    }

    pub fn get_auto_fill_fee(&self) -> f32 {
        let auto_fill_level = self.get_auto_power_fill_level();
        match auto_fill_level {
            1..=3 => 0.75,
            4..=6 => 0.65 - (auto_fill_level - 4) as f32 * 0.05,
            7..=9 => 0.60 - (auto_fill_level - 7) as f32 * 0.05,
            10..=12 => 0.55 - (auto_fill_level - 10) as f32 * 0.05,
            13..=15 => 0.50 - (auto_fill_level - 13) as f32 * 0.05,
            16..=18 => 0.45 - (auto_fill_level - 16) as f32 * 0.05,
            19..=21 => 0.32 - (auto_fill_level - 19) as f32 * 0.02,
            22..=24 => 0.25 - (auto_fill_level - 22) as f32 * 0.02,
            25..=27 => 0.20 - (auto_fill_level - 25) as f32 * 0.02,
            28..=30 => 0.15 - (auto_fill_level - 28) as f32 * 0.02,
            31..=33 => 0.10 - (auto_fill_level - 31) as f32 * 0.02,
            34..=36 => 0.05 - (auto_fill_level - 34) as f32 * 0.01,
            _ => 0.0,
        }
    }

    pub fn get_power_fill(&self) -> f32 {
        self.available_power / self.power_capacity()
    }

    pub fn get_power_fill_cost(&self, day: u32) -> f64 {
        (self.power_capacity() - self.available_power) as f64 / get_season(day)
    }

    pub fn fill_power(&mut self) {
        self.available_power = self.power_capacity();
    }

    pub fn upgrade(&mut self) {
        self.level += 1;
        self.fill_power();

        let max_gpu_slots = self.get_max_gpu_slots();

        let max_asic_slots = self.get_max_asic_slots();

        let max_click_power = if self.level < 5 { 10 } else { self.level };

        self.max_gpu_slots = max_gpu_slots;
        self.max_asic_slots = max_asic_slots;
        self.max_click_power = max_click_power;
    }

    pub fn get_max_asic_slots(&self) -> u32 {
        let max_asic_slots = if self.level < 35 {
            0
        } else {
            match self.level {
                35..=50 => (self.level - 35) / 2 + 1,
                51..=65 => 8 + (self.level - 51) / 2 * 2,
                66..=80 => 22 + (self.level - 66) / 2 * 4,
                81..=95 => 52 + (self.level - 81) / 2 * 6,
                96..=110 => 94 + (self.level - 96) / 2 * 8,
                111..=125 => 150 + (self.level - 111) / 2 * 10,
                126..=140 => 220 + (self.level - 126) / 2 * 12,
                141..=155 => 304 + (self.level - 141) / 2 * 14,
                156..=170 => 402 + (self.level - 156) / 2 * 16,
                _ => 514 + (self.level - 171) / 2 * 18,
            }
        };
        max_asic_slots
    }

    pub fn get_max_gpu_slots(&self) -> u32 {
        let max_gpu_slots = if self.level < 5 {
            0
        } else {
            match self.level {
                5..=21 => (self.level - 5) / 2 + 1,
                22..=35 => 9 + (self.level - 22) / 2 * 2,
                36..=50 => 23 + (self.level - 36) / 2 * 4,
                51..=65 => 53 + (self.level - 51) / 2 * 6,
                66..=80 => 95 + (self.level - 66) / 2 * 8,
                81..=95 => 151 + (self.level - 81) / 2 * 10,
                96..=110 => 221 + (self.level - 96) / 2 * 12,
                111..=125 => 305 + (self.level - 111) / 2 * 14,
                126..=140 => 403 + (self.level - 126) / 2 * 16,
                141..=155 => 515 + (self.level - 141) / 2 * 18,
                156..=170 => 641 + (self.level - 156) / 2 * 20,
                _ => 781 + (self.level - 171) / 2 * 22,
            }
        };
        max_gpu_slots
    }

    pub fn add_gpu_slot(&mut self) {
        if self.gpu_slot.amount < self.max_gpu_slots {
            self.gpu_slot.add_gpu();
        }
    }

    pub fn add_asic_slot(&mut self) {
        if self.asic_slot.amount < self.max_asic_slots {
            self.asic_slot.add_asic();
        }
    }

    pub fn upgrade_cpu(&mut self) {
        if self.cpu_upgrade_level < 5 {
            self.cpu_slot.upgrade();
            self.cpu_upgrade_level += 1;
        }
    }

    pub fn get_cpu_level(&self) -> u32 {
        self.cpu_slot.get_level()
    }

    pub fn get_gpu_level(&self) -> u32 {
        self.gpu_slot.get_level()
    }

    pub fn get_asic_level(&self) -> u32 {
        self.asic_slot.get_level()
    }

    pub fn upgrade_click_power(&mut self) {
        if self.click_power < self.max_click_power {
            self.click_power += 1;
        }
    }

    pub fn add_click_power(&mut self) {
        self.available_power += (self.power_capacity() * 0.05) as f32;
        self.available_power = self.available_power.min(self.power_capacity());
    }

    pub fn add_power(&mut self, power: f32) {
        self.available_power += power;
    }

    pub fn fill_to_percent(&mut self, percent: f32) {
        self.available_power = self.power_capacity() * percent;
    }

    pub fn get_click_upgrade_cost(&self) -> f64 {
        25.0 * self.click_power as f64
    }

    pub fn upgrade_gpu(&mut self) {
        self.gpu_slot.upgrade();
        self.add_gpu_slot();
        self.gpu_upgrade_level = self.get_top_gpu_level();
    }

    fn get_top_gpu_level(&self) -> u32 {
        self.gpu_slot.level
    }

    pub fn upgrade_asic(&mut self) {
        self.asic_slot.upgrade();
        self.add_asic_slot();
        self.asic_upgrade_level = self.get_top_asic_level();
    }

    fn get_top_asic_level(&self) -> u32 {
        self.asic_slot.level
    }

    pub fn get_cpu_upgrade_cost(&self) -> f64 {
        (25 * self.cpu_upgrade_level) as f64
    }

    pub fn get_rig_upgrade_cost(&self) -> f64 {
        match self.level {
            1..=5 => 10.0 + (self.level - 1) as f64 * 20.0,
            6..=10 => 90.0 + (self.level - 6) as f64 * 60.0,
            11..=15 => 350.0 + (self.level - 11) as f64 * 150.0,
            16..=20 => 1_000.0 + (self.level - 16) as f64 * 300.0,
            21..=25 => 2_600.0 + (self.level - 21) as f64 * 500.0,
            26..=30 => 5_600.0 + (self.level - 26) as f64 * 1_000.0,
            31..=35 => 10_600.0 + (self.level - 31) as f64 * 1_500.0,
            36..=40 => 16_600.0 + (self.level - 36) as f64 * 2_000.0,
            _ => (26_600.0 + (self.level - 41) as f64 * 2_500.0).min(50_000.0),
        }
    }

    pub fn get_gpu_upgrade_cost(&self) -> f64 {
        match self.gpu_upgrade_level {
            1..=5 => (250 * self.gpu_upgrade_level) as f64,
            6..=10 => (300 * self.gpu_upgrade_level) as f64,
            11..=15 => (350 * self.gpu_upgrade_level) as f64,
            16..=20 => (400 * self.gpu_upgrade_level) as f64,
            21..=25 => (450 * self.gpu_upgrade_level) as f64,
            26..=30 => (500 * self.gpu_upgrade_level) as f64,
            31..=35 => (550 * self.gpu_upgrade_level) as f64,
            36..=40 => (600 * self.gpu_upgrade_level) as f64,
            _ => (1000 * self.gpu_upgrade_level) as f64,
        }
    }

    pub fn get_asic_upgrade_cost(&self) -> f64 {
        match self.asic_upgrade_level {
            1..=5 => (2500 * self.asic_upgrade_level) as f64,
            6..=10 => (3000 * self.asic_upgrade_level) as f64,
            11..=15 => (3500 * self.asic_upgrade_level) as f64,
            16..=20 => (4000 * self.asic_upgrade_level) as f64,
            21..=25 => (4500 * self.asic_upgrade_level) as f64,
            26..=30 => (5000 * self.asic_upgrade_level) as f64,
            31..=35 => (5500 * self.asic_upgrade_level) as f64,
            36..=40 => (6000 * self.asic_upgrade_level) as f64,
            _ => (10_000 * self.asic_upgrade_level) as f64,
        }
    }

    pub fn get_new_gpu_cost(&self) -> u32 {
        500 * self.gpu_upgrade_level
    }

    pub fn get_new_asic_cost(&self) -> u32 {
        5000 * self.asic_upgrade_level
    }

    pub fn get_power_usage(&self) -> u32 {
        let cpu_power = self.cpu_slot.get_power_usage();
        let gpu_power = self.gpu_slot.get_power_usage();
        let asic_power = self.asic_slot.get_power_usage();

        cpu_power + gpu_power + asic_power
    }

    pub fn get_cpu_hash_rate(&self) -> u32 {
        self.cpu_slot.get_hash_rate()
    }

    pub fn get_cpu_power_usage(&self) -> u32 {
        self.cpu_slot.get_power_usage()
    }

    pub fn get_gpu_hash_rate(&self) -> u32 {
        self.gpu_slot.get_hash_rate()
    }

    pub fn get_gpu_power_usage(&self) -> u32 {
        self.gpu_slot.get_power_usage()
    }

    pub fn get_asic_hash_rate(&self) -> u32 {
        self.asic_slot.get_hash_rate()
    }

    pub fn get_asic_power_usage(&self) -> u32 {
        self.asic_slot.get_power_usage()
    }

    pub fn get_filled_gpu_slots(&self) -> u32 {
        self.gpu_slot.amount
    }

    pub fn get_filled_asic_slots(&self) -> u32 {
        self.asic_slot.amount
    }

    pub fn get_hash_rate(&self) -> u32 {
        let cpu_hash = self.cpu_slot.get_hash_rate();
        let gpu_hash = self.gpu_slot.get_hash_rate();
        let asic_hash = self.asic_slot.get_hash_rate();

        cpu_hash + gpu_hash + asic_hash
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AsicSlot {
    pub level: u32,
    pub active: bool,
    pub amount: u32,
}

impl AsicSlot {
    pub fn new(level: u32) -> Self {
        AsicSlot {
            level,
            active: true,
            amount: 0,
        }
    }
    pub fn add_asic(&mut self) {
        self.amount += 1;
    }
    pub fn toggle_active(&mut self) {
        self.active = !self.active;
    }
    pub fn get_level(&self) -> u32 {
        self.level
    }
    pub fn upgrade(&mut self) {
        self.level += 1;
    }
    pub fn get_power_usage(&self) -> u32 {
        if !self.active {
            return 0;
        }
        350 * self.amount
    }
    pub fn get_hash_rate(&self) -> u32 {
        if !self.active {
            return 0;
        }
        125 * self.amount
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GpuSlot {
    pub level: u32,
    pub active: bool,
    pub amount: u32,
}

impl GpuSlot {
    pub fn new(level: u32) -> Self {
        GpuSlot {
            level,
            active: true,
            amount: 0,
        }
    }
    pub fn add_gpu(&mut self) {
        self.amount += 1;
    }
    pub fn toggle_active(&mut self) {
        self.active = !self.active;
    }
    pub fn get_level(&self) -> u32 {
        self.level
    }
    pub fn upgrade(&mut self) {
        self.level += 1;
    }
    pub fn get_power_usage(&self) -> u32 {
        if !self.active {
            return 0;
        }
        125 * self.amount
    }
    pub fn get_hash_rate(&self) -> u32 {
        if !self.active {
            return 0;
        }
        40 * self.amount
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CpuSlot {
    pub level: u32,
    pub active: bool,
}

impl CpuSlot {
    pub fn new(level: u32) -> Self {
        CpuSlot {
            level,
            active: true,
        }
    }
    pub fn toggle_active(&mut self) {
        self.active = !self.active;
    }
    pub fn get_level(&self) -> u32 {
        self.level
    }
    pub fn upgrade(&mut self) {
        let max_level = 10;
        if self.level < max_level {
            self.level += 1;
        }
    }
    pub fn get_power_usage(&self) -> u32 {
        if !self.active {
            return 0;
        }
        25 * self.level
    }
    pub fn get_hash_rate(&self) -> u32 {
        if !self.active {
            return 0;
        }
        10 * self.level
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CryptoCoin {
    pub name: String,
    pub initial_price: f32,
    pub current_price: f32,
    pub volatility: Range<f32>,
    pub prices: VecDeque<f32>,
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
            prices: VecDeque::from(vec![initial_price]),
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
            1..=5 => 8 * (20 - rig_lvl),
            6..=10 => 7 * (20 - (rig_lvl - 5)),
            11..=15 => 6 * (20 - (rig_lvl - 10)),
            16..=20 => 5 * (20 - (rig_lvl - 15)),
            21..=25 => 4 * (20 - (rig_lvl - 20)),
            26..=30 => 3 * (20 - (rig_lvl - 25)),
            31..=35 => 2 * (20 - (rig_lvl - 30)),
            36..=40 => 1 * (20 - (rig_lvl - 35)),
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

    fn calculate_rug_chance(&self) -> f32 {
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

        if self.blocks >= self.max_blocks || share_cooldown {
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

        self.prices.push_front(self.current_price);

        if self.prices.len() > MAX_SERIES_LENGTH {
            self.prices.pop_back();
        }

        self.trend_direction
            .push_front(self.current_price > starting_price);

        if self.trend_direction.len() > 4 {
            self.trend_direction.pop_back();
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Market {
    pub coins: Vec<CryptoCoin>,
    pub inactive_coins: Vec<CryptoCoin>,
    pub index: u32,
    pub bank: Bank,
}

impl Market {
    pub fn new() -> Self {
        let inactive_coins = Vec::with_capacity(1000);

        Market {
            coins: Vec::new(),
            inactive_coins,
            index: 0,
            bank: Bank::new(),
        }
    }

    pub fn add_coin(&mut self, coin: CryptoCoin) {
        self.coins.push(coin);
        self.index += 1;
    }

    pub fn remove_coin(&mut self, coin: &CryptoCoin) -> Option<usize> {
        let index = self.coins.iter().position(|c| c.name == coin.name);

        if let Some(index) = index {
            self.coins.remove(index);
        }

        index
    }

    pub fn set_profit_factor(&mut self) {
        let rig = MINING_RIG();
        let hash_rate = rig.get_hash_rate();

        for coin in &mut self.coins {
            coin.profit_factor = coin.calculate_profit_factor(hash_rate);
        }
    }

    pub fn sell_coins(&mut self, coin: &CryptoCoin) {
        if let Some(coin) = self.coins.iter_mut().find(|c| c.name == coin.name) {
            self.bank
                .deposit((coin.balance * coin.current_price) as f64);
            coin.balance = 0.0;
        }
    }

    pub fn update_coin(&mut self, coin: &CryptoCoin) {
        if let Some(index) = self.get_coin_index(coin) {
            self.coins[index] = coin.clone();
        }
    }

    pub fn get_coin_index(&self, coin: &CryptoCoin) -> Option<usize> {
        self.coins.iter().position(|c| c.name == coin.name)
    }

    pub fn get_coin_by_index(&self, index: usize) -> Option<&CryptoCoin> {
        self.coins.iter().find(|c| c.index == index)
    }

    pub fn set_coin_inactive(&mut self, coin: &CryptoCoin, day: u32) {
        if let Some(index) = self.get_coin_index(coin) {
            self.coins[index].active = false;
            self.coins[index].current_price = 0.0;
            self.coins[index].index = 100;
            self.coins[index].death_date = Some(day);

            let bal = self.coins[index].balance;

            if bal > 0.0 {
                self.inactive_coins.push(self.coins[index].clone());
            }

            self.coins.swap_remove(index);
        }
    }

    pub fn coin_by_name(&self, name: &str) -> Option<&CryptoCoin> {
        self.coins.iter().find(|c| c.name == name)
    }

    pub fn mut_coin_by_name(&mut self, name: &str) -> Option<&mut CryptoCoin> {
        self.coins.iter_mut().find(|c| c.name == name)
    }

    pub fn price_sorted_coins(&self) -> Vec<CryptoCoin> {
        let mut coins = self.coins.clone();
        coins.sort_by(|a, b| a.current_price.partial_cmp(&b.current_price).unwrap());
        coins.reverse();

        coins
    }

    pub fn index_sorted_coins(&self, with_inactive: bool) -> Vec<CryptoCoin> {
        let mut coins = self.coins.clone();
        coins.sort_by(|a, b| a.index.partial_cmp(&b.index).unwrap());

        if !with_inactive {
            return coins.into_iter().filter(|c| c.active).collect();
        } else {
            let inactive_coins = self.inactive_coins.clone();

            coins
                .into_iter()
                .filter(|c| c.active)
                .chain(inactive_coins.into_iter())
                .collect()
        }
    }

    pub fn get_active_coins(&self) -> Vec<CryptoCoin> {
        self.coins.iter().filter(|c| c.active).cloned().collect()
    }

    pub async fn simulate_day(&mut self) {
        for coin in &mut self.coins {
            coin.update_price();
        }
    }

    pub fn run_rug_pull(&mut self, day: u32) {
        for coin in &mut self.coins {
            let rug_chance = coin.calculate_rug_chance();
            if rand_from_range(0.0..1.0) < rug_chance {
                // Rug pull chance

                let rug_protection_active = MINING_RIG().get_rug_protection_active();

                if rug_protection_active && coin.balance > 0.0 {
                    let rug_protection_amount = MINING_RIG().get_rug_protection_amount();

                    let protected_amount = coin.balance * rug_protection_amount;
                    let protection_value = protected_amount * coin.current_price;

                    self.bank.deposit(protection_value as f64);

                    let msg = format!(
                        "DerpFi Rug protection activated for {}, {} coins sold for ${}",
                        coin.name, protected_amount, protection_value
                    );
                    command_line_output(&msg);
                }

                let msg = format!("{} has been rug pulled!", coin.name);
                command_line_output(&msg);
                coin.current_price = 0.0;
                coin.death_date = Some(day);
            }
        }
    }

    pub fn mut_get_any_share_cooldown(&mut self) -> Option<&mut CryptoCoin> {
        let longest_cooldown = self
            .coins
            .iter_mut()
            .filter(|c| c.get_share_cooldown() > 0)
            .max_by(|a, b| a.get_share_cooldown().cmp(&b.get_share_cooldown()));
        longest_cooldown
    }

    pub fn get_any_share_cooldown(&self) -> Option<&CryptoCoin> {
        let longest_cooldown = self
            .coins
            .iter()
            .filter(|c| c.get_share_cooldown() > 0)
            .max_by(|a, b| a.get_share_cooldown().cmp(&b.get_share_cooldown()));
        longest_cooldown
    }

    pub fn decrement_all_share_cooldowns(&mut self) {
        for coin in &mut self.coins {
            coin.decrement_share_cooldown();
        }
    }
}

pub fn cull_market(
    series_labels: &mut Signal<Vec<String>>,
    series: &mut Signal<Vec<Vec<f32>>>,
    rig_lvl: u32,
    day: u32,
) {
    let active_coins = MARKET().get_active_coins();
    for coin in active_coins {
        let mined_out = coin.blocks >= coin.max_blocks;
        let has_bal = coin.balance > 0.0;
        if coin.current_price < 0.01 || (mined_out && !has_bal) {
            replace_coin(&coin, series_labels, series, rig_lvl, day);
        }
    }
}

pub fn replace_coin(
    coin: &CryptoCoin,
    series_labels: &mut Signal<Vec<String>>,
    series: &mut Signal<Vec<Vec<f32>>>,
    rig_lvl: u32,
    day: u32,
) {
    let mut mkt = MARKET();
    let series_index = coin.index;
    mkt.set_coin_inactive(&coin, day);

    match SELECTION().name {
        Some(selection) => {
            if selection == coin.name {
                SELECTION.write().name = None;
                SELECTION.write().index = None;
                clear_selected_coin();
            }
        }
        None => {}
    }

    let mut current_series = series.write();
    current_series[series_index].clear();

    let new_coin = gen_random_coin(series_index, rig_lvl);
    mkt.add_coin(new_coin.clone());

    let mut series_labels = series_labels.write();

    series_labels[series_index] = new_coin.name.clone();

    *MARKET.write() = mkt;
}

pub fn gen_random_coin(index: usize, rig_lvl: u32) -> CryptoCoin {
    let volitility = rand_from_range(0.008..0.08);
    let mkt = MARKET();

    let coin_name = { format!("Coin-{}", mkt.index) };

    let shares_per_block = 1000;
    let block_reward = 100.0;
    let max_blocks = rand_from_range(10.0..25.0) as u32;

    let max_hashes_per_share = (rig_lvl * 1000).min(25_000);

    let hashes_per_share = rand_from_range(1000.0..max_hashes_per_share as f32);

    let berth_date = GAME_TIME().day;

    CryptoCoin::new(
        &coin_name,
        rand_from_range(8.0..20.0),
        -volitility..volitility,
        index,
        shares_per_block,
        block_reward,
        max_blocks,
        hashes_per_share,
        berth_date,
    )
}

pub fn gen_random_coin_with_set_index(index: usize, rig_lvl: u32) -> CryptoCoin {
    let volitility = rand_from_range(0.008..0.08);

    let coin_name = { format!("Coin-{}", index) };

    let shares_per_block = 1000;
    let block_reward = 100.0;
    let max_blocks = rand_from_range(10.0..25.0) as u32;

    let max_hashes_per_share = (rig_lvl * 1000).min(25_000);

    let hashes_per_share = rand_from_range(1000.0..max_hashes_per_share as f32);

    let berth_date = GAME_TIME().day;

    CryptoCoin::new(
        &coin_name,
        rand_from_range(8.0..20.0),
        -volitility..volitility,
        index,
        shares_per_block,
        block_reward,
        max_blocks,
        hashes_per_share,
        berth_date,
    )
}

pub fn clear_selected_coin() {
    let window = window();
    let document = window.document().expect("should have document");

    let radios = document
        .query_selector_all("input[name='coin-selection']")
        .expect("should have radios");

    for i in 0..radios.length() {
        let radio = radios.get(i).expect("should have radio");
        let radio = radio
            .dyn_into::<web_sys::HtmlInputElement>()
            .expect("should be a radio");

        radio.set_checked(false);
    }

    let rows = document.query_selector_all("tr").expect("should have rows");

    for i in 0..rows.length() {
        let row = rows.get(i).expect("should have row");
        let row = row.dyn_into::<web_sys::Element>().expect("should be a row");

        row.set_class_name("");
    }
}
