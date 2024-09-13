#![allow(dead_code)]
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::utils::get_season;

pub static MINING_RIG: GlobalSignal<MiningRig> = Signal::global(|| MiningRig::new());

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AutoPowerFill {
    pub level: u64,
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
    pub level: u64,
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
        Bank {
            balance: 100_000_000_000.0,
        }
    }

    pub fn deposit(&mut self, amount: f64) {
        self.balance += amount;
    }

    pub fn withdraw(&mut self, amount: f64) -> bool {
        let persition = 0.0001;

        if self.balance >= amount {
            self.balance -= amount;
            true
        } else {
            if (self.balance - amount).abs() < persition {
                self.balance -= amount;
                true
            } else {
                false
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MiningRig {
    pub level: u64,
    pub power_capacity: f64,
    pub available_power: f64,
    pub cpu_slot: CpuSlot,
    pub gpu_slot: GpuSlot,
    pub asic_slot: AsicSlot,
    pub click_power: u64,
    pub max_gpu_slots: u64,
    pub max_asic_slots: u64,
    pub max_click_power: u64,
    pub cpu_upgrade_level: u64,
    pub gpu_upgrade_level: u64,
    pub asic_upgrade_level: u64,
    pub auto_power_fill: Option<AutoPowerFill>,
    pub rug_protection: RugProtection,
    pub auto_mining_level: Option<u64>,
}

impl MiningRig {
    pub fn new() -> Self {
        MiningRig {
            level: 165,
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
            auto_mining_level: None,
        }
    }

    pub fn get_rug_protection_level(&self) -> u64 {
        if !self.rug_protection.active {
            return 0;
        }
        self.rug_protection.level
    }

    pub fn get_rug_protection_active(&self) -> bool {
        self.rug_protection.active
    }

    pub fn get_rug_protection_amount(&self) -> f64 {
        if self.level < 10 || !self.rug_protection.active {
            return 0.0;
        }

        match self.rug_protection.level {
            0 => 0.0,
            1..=3 => 0.1 + ((self.rug_protection.level - 1) as f64 * 0.02),
            4..=6 => 0.15 + ((self.rug_protection.level - 4) as f64 * 0.02),
            7..=9 => 0.20 + ((self.rug_protection.level - 7) as f64 * 0.02),
            10..=12 => 0.25 + ((self.rug_protection.level - 10) as f64 * 0.02),
            13..=15 => 0.30 + ((self.rug_protection.level - 13) as f64 * 0.02),
            16..=18 => 0.35 + ((self.rug_protection.level - 16) as f64 * 0.02),
            19..=21 => 0.40 + ((self.rug_protection.level - 19) as f64 * 0.02),
            22..=24 => 0.45 + ((self.rug_protection.level - 22) as f64 * 0.02),
            25..=27 => 0.50 + ((self.rug_protection.level - 25) as f64 * 0.02),
            28..=30 => 0.55 + ((self.rug_protection.level - 28) as f64 * 0.02),
            31..=33 => 0.60 + ((self.rug_protection.level - 31) as f64 * 0.02),
            34..=36 => 0.65 + ((self.rug_protection.level - 34) as f64 * 0.02),
            37..=40 => 0.70 + ((self.rug_protection.level - 37) as f64 * 0.02),
            41..=45 => 0.75 + ((self.rug_protection.level - 41) as f64 * 0.01),
            46..=50 => 0.80 + ((self.rug_protection.level - 46) as f64 * 0.01),
            51..=55 => 0.85 + ((self.rug_protection.level - 51) as f64 * 0.01),
            56..=60 => 0.90 + ((self.rug_protection.level - 56) as f64 * 0.01),
            61..=64 => 0.95 + ((self.rug_protection.level - 61) as f64 * 0.01),
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

    pub fn get_level(&self) -> u64 {
        self.level
    }

    pub fn consume_power(&mut self) -> bool {
        let power_usage_watts = (self.get_power_usage() as f64) / 40.0;

        if self.available_power >= power_usage_watts {
            self.available_power -= power_usage_watts;
            true
        } else {
            // Not enough power to run the rig
            false
        }
    }

    pub fn get_new_coin_cooldown(&self) -> u64 {
        self.click_power
    }

    pub fn decrement_new_coin_cooldown(&mut self) {
        if self.click_power > 0 {
            self.click_power -= 1;
        }
    }

    pub fn set_new_coin_cooldown(&mut self) {
        self.click_power = 5 * 20;
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

    pub fn get_auto_power_fill_level(&self) -> u64 {
        if let Some(auto_power_fill) = &self.auto_power_fill {
            auto_power_fill.level
        } else {
            0
        }
    }

    fn power_capacity(&self) -> f64 {
        self.get_power_usage() as f64 * 40.0
    }

    pub fn get_auto_power_fill_cost(&self, day: u64) -> f64 {
        let refill_cost = self.power_capacity() / get_season(day);
        refill_cost * (1.0 + self.get_auto_fill_fee()) * self.get_auto_power_fill_amount()
    }

    pub fn get_available_power(&self) -> f64 {
        self.available_power
    }

    pub fn get_power_capacity(&self) -> f64 {
        self.power_capacity()
    }

    pub fn get_auto_power_fill_delay(&self) -> u64 {
        let auto_fill_level = self.get_auto_power_fill_level();
        match auto_fill_level {
            1 => 5_000 / 50,
            2 => 4_500 / 50,
            3 => 4_000 / 50,
            4 => 3_500 / 50,
            5 => 3_000 / 50,
            6 => 2_500 / 50,
            7 => 2_000 / 50,
            8 => 1_500 / 50,
            9 => 1_000 / 50,
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

    pub fn get_auto_power_fill_amount(&self) -> f64 {
        let auto_fill_level = self.get_auto_power_fill_level();
        match auto_fill_level {
            1..=3 => 0.25 + ((auto_fill_level - 1) as f64 * 0.02),
            4..=6 => 0.33 + ((auto_fill_level - 4) as f64 * 0.03),
            7..=9 => 0.50 + ((auto_fill_level - 7) as f64 * 0.05),
            10..=12 => 0.75 + ((auto_fill_level - 10) as f64 * 0.08),
            _ => 1.0,
        }
        .min(1.0)
    }

    pub fn get_auto_fill_fee(&self) -> f64 {
        let auto_fill_level = self.get_auto_power_fill_level();
        match auto_fill_level {
            1..=3 => 0.25 - (auto_fill_level - 1) as f64 * 0.05,
            4..=6 => 0.10 - (auto_fill_level - 4) as f64 * 0.05,
            7..=9 => 0.05 - (auto_fill_level - 7) as f64 * 0.05,
            10..=12 => 0.0 - (auto_fill_level - 10) as f64 * 0.05,
            _ => 0.0,
        }
    }

    pub fn get_power_fill(&self) -> f64 {
        self.available_power / self.power_capacity()
    }

    pub fn get_power_fill_cost(&self, day: u64) -> f64 {
        (self.power_capacity() - self.available_power) / get_season(day)
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

    pub fn get_max_asic_slots(&self) -> u64 {
        let ranges = [
            (35, 49, 0, 2),
            (51, 65, 19, 3),
            (67, 79, 47, 4),
            (81, 95, 81, 6),
            (97, 109, 137, 8),
            (111, 125, 203, 10),
            (127, 139, 295, 12),
            (141, 155, 393, 14),
            (157, 169, 521, 16),
            (171, 191, 651, 18),
            (193, 207, 873, 24),
            (209, 229, 1121, 32),
            (231, 255, 1513, 40),
            (257, 281, 2081, 48),
            (283, 307, 2761, 56),
            (309, 333, 3553, 64),
            (335, 359, 4457, 72),
            (361, 385, 5473, 80),
            (387, 411, 6601, 88),
            (413, 437, 7841, 96),
            (439, 463, 9193, 104),
            (465, 489, 10657, 112),
            (491, 515, 12233, 120),
            (517, u64::MAX, 13921, 128),
        ];

        if self.level < 35 {
            return 0;
        }

        for &(start, end, base, multiplier) in &ranges {
            if self.level >= start && self.level <= end {
                let odd_level_increments = ((self.level - start) / 2) + 1;
                return base + odd_level_increments * multiplier;
            }
        }

        self.max_asic_slots
    }

    pub fn get_max_gpu_slots(&self) -> u64 {
        let ranges = [
            (5, 21, 0, 1),
            (22, 34, 9, 2),
            (36, 50, 27, 4),
            (52, 66, 57, 6),
            (66, 80, 107, 8),
            (82, 94, 173, 10),
            (96, 110, 245, 12),
            (112, 124, 343, 14),
            (126, 140, 443, 16),
            (142, 154, 573, 18),
            (156, 170, 701, 20),
            (172, 196, 863, 22),
            (198, 222, 1149, 24),
            (224, 248, 1461, 32),
            (250, 274, 1917, 40),
            (276, 300, 2485, 48),
            (302, 326, 3165, 56),
            (328, 352, 4977, 64),
            (354, 378, 5881, 72),
            (380, 404, 6897, 80),
            (406, 430, 8025, 88),
            (432, 456, 9265, 96),
            (458, 482, 10617, 104),
            (484, 508, 12081, 112),
            (510, 534, 13657, 120),
            (536, u64::MAX, 15345, 128),
        ];

        if self.level < 5 {
            return 0;
        }

        for &(start, end, base, multiplier) in &ranges {
            if self.level >= start && self.level <= end {
                let increments = if self.level < 35 {
                    ((self.level - start) / 2) + 1
                } else {
                    (self.level - start) / 2
                };
                return base + increments * multiplier;
            }
        }

        self.max_gpu_slots
    }

    pub fn upgrade_cpu(&mut self) {
        if self.cpu_upgrade_level < 5 {
            self.cpu_slot.upgrade();
            self.cpu_upgrade_level += 1;
        }
    }

    pub fn get_cpu_level(&self) -> u64 {
        self.cpu_slot.get_level()
    }

    pub fn get_gpu_level(&self) -> u64 {
        self.gpu_slot.get_level()
    }

    pub fn get_asic_level(&self) -> u64 {
        self.asic_slot.get_level()
    }

    pub fn add_click_power(&mut self) {
        self.available_power += self.power_capacity() * 0.05;
        self.available_power = self.available_power.min(self.power_capacity());
    }

    pub fn add_power(&mut self, power: f64) {
        self.available_power += power;
    }

    pub fn fill_to_percent(&mut self, percent: f64) {
        self.available_power = self.power_capacity() * percent;
    }

    pub fn upgrade_gpu(&mut self) {
        self.gpu_slot.upgrade();
        self.gpu_upgrade_level = self.get_top_gpu_level();
    }

    fn get_top_gpu_level(&self) -> u64 {
        self.gpu_slot.level
    }

    pub fn upgrade_asic(&mut self) {
        self.asic_slot.upgrade();
        self.asic_upgrade_level = self.get_top_asic_level();
    }

    fn get_top_asic_level(&self) -> u64 {
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
            _ => 26_600.0 + (self.level - 41) as f64 * 2_500.0,
        }
    }

    pub fn get_gpu_upgrade_cost(&self) -> f64 {
        match self.gpu_upgrade_level {
            1..=5 => (150 * self.gpu_upgrade_level) as f64,
            6..=10 => (200 * self.gpu_upgrade_level) as f64,
            11..=15 => (250 * self.gpu_upgrade_level) as f64,
            16..=20 => (300 * self.gpu_upgrade_level) as f64,
            21..=25 => (250 * self.gpu_upgrade_level) as f64,
            26..=30 => (400 * self.gpu_upgrade_level) as f64,
            31..=35 => (450 * self.gpu_upgrade_level) as f64,
            36..=40 => (500 * self.gpu_upgrade_level) as f64,
            _ => (600 * self.gpu_upgrade_level) as f64,
        }
    }

    pub fn get_asic_upgrade_cost(&self) -> f64 {
        match self.asic_upgrade_level {
            1..=5 => (3000 * self.asic_upgrade_level) as f64,
            6..=10 => (3500 * self.asic_upgrade_level) as f64,
            11..=15 => (4000 * self.asic_upgrade_level) as f64,
            16..=20 => (4500 * self.asic_upgrade_level) as f64,
            21..=25 => (5000 * self.asic_upgrade_level) as f64,
            26..=30 => (5500 * self.asic_upgrade_level) as f64,
            31..=35 => (6000 * self.asic_upgrade_level) as f64,
            36..=40 => (6500 * self.asic_upgrade_level) as f64,
            _ => (7500 * self.asic_upgrade_level) as f64,
        }
    }

    pub fn get_new_gpu_cost(&self) -> u64 {
        500 * self.gpu_upgrade_level
    }

    pub fn get_new_asic_cost(&self) -> u64 {
        5000 * self.asic_upgrade_level
    }

    pub fn get_power_usage(&self) -> u64 {
        let cpu_power = self.cpu_slot.get_power_usage();
        let gpu_power = self.gpu_slot.get_power_usage();
        let asic_power = self.asic_slot.get_power_usage();

        cpu_power + gpu_power + asic_power
    }

    pub fn get_cpu_hash_rate(&self) -> u64 {
        self.cpu_slot.get_hash_rate()
    }

    pub fn get_cpu_power_usage(&self) -> u64 {
        self.cpu_slot.get_power_usage()
    }

    pub fn get_gpu_hash_rate(&self) -> u64 {
        self.gpu_slot.get_hash_rate()
    }

    pub fn get_gpu_power_usage(&self) -> u64 {
        self.gpu_slot.get_power_usage()
    }

    pub fn get_asic_hash_rate(&self) -> u64 {
        self.asic_slot.get_hash_rate()
    }

    pub fn get_asic_power_usage(&self) -> u64 {
        self.asic_slot.get_power_usage()
    }

    pub fn get_filled_gpu_slots(&self) -> u64 {
        self.gpu_slot.amount
    }

    pub fn get_filled_asic_slots(&self) -> u64 {
        self.asic_slot.amount
    }

    pub fn get_hash_rate(&self) -> u64 {
        let cpu_hash = self.cpu_slot.get_hash_rate();
        let gpu_hash = self.gpu_slot.get_hash_rate();
        let asic_hash = self.asic_slot.get_hash_rate();

        cpu_hash + gpu_hash + asic_hash
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AsicSlot {
    pub level: u64,
    pub active: bool,
    pub amount: u64,
}

impl AsicSlot {
    pub fn new(level: u64) -> Self {
        AsicSlot {
            level,
            active: true,
            amount: 0,
        }
    }

    pub fn toggle_active(&mut self) {
        self.active = !self.active;
    }
    pub fn get_level(&self) -> u64 {
        self.level
    }
    pub fn upgrade(&mut self) {
        self.level += 1;
        self.amount += 1;
    }
    pub fn get_power_usage(&self) -> u64 {
        if !self.active {
            return 0;
        }
        1800 * self.amount
    }
    pub fn get_hash_rate(&self) -> u64 {
        if !self.active {
            return 0;
        }
        1200 * self.amount
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GpuSlot {
    pub level: u64,
    pub active: bool,
    pub amount: u64,
}

impl GpuSlot {
    pub fn new(level: u64) -> Self {
        GpuSlot {
            level,
            active: true,
            amount: 0,
        }
    }

    pub fn toggle_active(&mut self) {
        self.active = !self.active;
    }
    pub fn get_level(&self) -> u64 {
        self.level
    }
    pub fn upgrade(&mut self) {
        self.level += 1;
        self.amount += 1;
    }
    pub fn get_power_usage(&self) -> u64 {
        if !self.active {
            return 0;
        }
        500 * self.amount
    }
    pub fn get_hash_rate(&self) -> u64 {
        if !self.active {
            return 0;
        }
        225 * self.amount
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CpuSlot {
    pub level: u64,
    pub active: bool,
}

impl CpuSlot {
    pub fn new(level: u64) -> Self {
        CpuSlot {
            level,
            active: true,
        }
    }
    pub fn toggle_active(&mut self) {
        self.active = !self.active;
    }
    pub fn get_level(&self) -> u64 {
        self.level
    }
    pub fn upgrade(&mut self) {
        let max_level = 10;
        if self.level < max_level {
            self.level += 1;
        }
    }
    pub fn get_power_usage(&self) -> u64 {
        if !self.active {
            return 0;
        }
        125 * self.level
    }
    pub fn get_hash_rate(&self) -> u64 {
        if !self.active {
            return 0;
        }
        125 * self.level
    }
}
