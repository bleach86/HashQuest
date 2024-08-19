#![allow(dead_code)]
use dioxus::prelude::*;
use gloo_utils::window;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;

use crate::crypto_coin::CryptoCoin;
use crate::i_db::Selection;
use crate::mining_rig::{Bank, MINING_RIG};
use crate::utils::{command_line_output, rand_from_range, GameTime};

pub const MAX_SERIES_LENGTH: usize = 96;
pub static MARKET: GlobalSignal<Market> = Signal::global(|| Market::new());
pub static SELECTION: GlobalSignal<Selection> = Signal::global(|| Selection::default());
pub static GAME_TIME: GlobalSignal<GameTime> = Signal::global(|| GameTime::new());

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
