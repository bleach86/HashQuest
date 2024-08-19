use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::utils::get_season;

pub static MINING_RIG: GlobalSignal<MiningRig> = Signal::global(|| MiningRig::new());

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
