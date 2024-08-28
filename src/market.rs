#![allow(dead_code)]

use dioxus::prelude::*;
use gloo_utils::window;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;

use crate::crypto_coin::CryptoCoin;
use crate::i_db::Selection;
use crate::mining_rig::{Bank, MINING_RIG};
use crate::utils::{command_line_output, rand_from_range, truncate_price, GameTime};

pub const MAX_SERIES_LENGTH: usize = 96;
pub static MARKET: GlobalSignal<Market> = Signal::global(|| Market::new());
pub static SELECTION: GlobalSignal<Selection> = Signal::global(|| Selection::default());
pub static GAME_TIME: GlobalSignal<GameTime> = Signal::global(|| GameTime::new());

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketChart {
    pub labels: Vec<String>,
    pub series: Vec<Vec<f32>>,
    pub series_labels: Vec<String>,
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

    pub fn sell_coins(&mut self, coin: &CryptoCoin, amount: Option<f32>) {
        if let Some(coin) = self.coins.iter_mut().find(|c| c.name == coin.name) {
            let amount = amount.unwrap_or(coin.balance);

            self.bank.deposit((amount * coin.current_price) as f64);
            coin.balance -= amount;
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
            self.coins[index].prices.clear();

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

    pub fn simulate_day(&mut self) {
        for coin in &mut self.coins {
            coin.update_price();
        }
    }

    pub fn simulate_day_single(&mut self, coin: &CryptoCoin) {
        if let Some(coin) = self.coins.iter_mut().find(|c| c.name == coin.name) {
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

    pub fn buy_coin(&mut self, coin: &CryptoCoin, amount: f32) -> bool {
        if let Some(coin) = self.coins.iter_mut().find(|c| c.name == coin.name) {
            let cost = coin.current_price * amount;

            if self.bank.withdraw(cost as f64) {
                coin.balance += amount;
                return true;
            }
        }
        false
    }

    pub fn get_max_buyable(&self, coin: &CryptoCoin) -> f64 {
        let bal = self.bank.balance;
        let coin = self.coins.iter().find(|c| c.name == coin.name).unwrap();
        let price = coin.current_price as f64;

        bal / price
    }

    pub fn buy_max_coin(&mut self, coin: &CryptoCoin) -> bool {
        let max_buyable = self.get_max_buyable(coin);
        self.buy_coin(coin, max_buyable as f32)
    }

    pub fn get_newest_coin(&self) -> Option<CryptoCoin> {
        let newest_idx = self.index - 1;
        self.coins
            .iter()
            .find(|c| {
                let name = c.name.clone();
                let name_split = name.split('-').collect::<Vec<&str>>();
                let curr_idx = name_split[1].parse::<u32>().unwrap();
                curr_idx == newest_idx
            })
            .cloned()
    }

    pub fn get_coin_prince(&self, coin: &CryptoCoin) -> f32 {
        self.coins
            .iter()
            .find(|c| c.name == coin.name)
            .map(|c| c.current_price)
            .unwrap_or(0.0)
    }

    pub fn truncate_prices(&mut self) {
        for coin in &mut self.coins {
            for price in coin.prices.iter_mut() {
                *price = truncate_price(*price);
            }
        }
    }

    fn get_sersies(&self) -> Vec<Vec<f32>> {
        let mut series = Vec::new();

        for coin in &self.index_sorted_coins(false) {
            series.push(coin.prices.clone());
        }

        series
    }

    fn get_series_labels(&self) -> Vec<String> {
        let mut labels = Vec::new();

        for coin in &self.index_sorted_coins(false) {
            labels.push(coin.name.clone());
        }

        labels
    }

    fn get_labels(&self) -> Vec<String> {
        let mut max_len = 0;

        for coin in &self.coins {
            if coin.prices.len() > max_len {
                max_len = coin.prices.len();
            }
        }

        let mut labels = Vec::new();

        for _ in 0..max_len {
            labels.push("|".to_string());
        }

        labels
    }

    pub fn get_chart(&self) -> MarketChart {
        let labels = self.get_labels();
        let series = self.get_sersies();
        let series_labels = self.get_series_labels();

        MarketChart {
            labels,
            series,
            series_labels,
        }
    }

    pub fn reverse_price_history(&mut self) {
        for coin in &mut self.coins {
            let reverse_list = coin.prices.clone().into_iter().rev().collect::<Vec<f32>>();

            coin.prices = reverse_list;
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
    let volitility = rand_from_range(0.02..0.08);
    let mkt = MARKET();

    let coin_name = { format!("Coin-{}", mkt.index) };

    let shares_per_block = 1000;
    let block_reward = 100.0;
    let max_blocks = rand_from_range(10.0..25.0) as u32;

    let max_hashes_per_share = (rig_lvl * 1000).min(5_000);

    let hashes_per_share = rand_from_range(1000.0..max_hashes_per_share as f32);

    let berth_date = GAME_TIME().day;

    let price_range = match rig_lvl {
        0..=3 => 8.0..20.0,
        4..=6 => 20.0..40.0,
        7..=9 => 40.0..60.0,
        10..=12 => 60.0..80.0,
        13.. => 80.0..100.0,
    };

    CryptoCoin::new(
        &coin_name,
        rand_from_range(price_range),
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
    let volitility = rand_from_range(0.02..0.08);

    let coin_name = { format!("Coin-{}", index) };

    let shares_per_block = 1000;
    let block_reward = 100.0;
    let max_blocks = rand_from_range(10.0..25.0) as u32;

    let max_hashes_per_share = (rig_lvl * 1000).min(5_000);

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
