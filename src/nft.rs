#![allow(dead_code)]
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NftStudio {
    pub rep: u64,
    pub hype: f64,
    pub pop: f64,
    pub nft_drawn: u64,
    pub last_release: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Nft {
    pub name: String,
    pub score: f64,
    pub studio_rep: u64,
    pub price: f64,
}

impl Nft {
    pub fn new(name: String, score: f64, studio_rep: u64, last_nft_days_ago: u64) -> Self {
        let score = Nft::calc_score(studio_rep, score, last_nft_days_ago);
        let price = Nft::calc_price(studio_rep, score);

        Nft {
            name,
            score,
            studio_rep,
            price,
        }
    }

    fn calc_price(studio_rep: u64, score: f64) -> f64 {
        match studio_rep {
            0 => score * 0.5,
            1 => score * 0.75,
            2..=4 => score * (5.0 + ((studio_rep as f64 - 1.0) * 1.25)),
            5..=9 => score * (10.5 + ((studio_rep as f64 - 4.0) * 0.1)),
            10..=19 => score * (50.0 + ((studio_rep as f64 - 9.0) * 4.15)),
            20..=35 => score * (100.0 + ((studio_rep as f64 - 19.0) * 8.0)),
            36..=59 => score * (250.0 + ((studio_rep as f64 - 35.0) * 35.0)),
            60..=99 => score * (1200.25 + ((studio_rep as f64 - 59.0) * 15.0)),
            100..=149 => score * (2000.5 + ((studio_rep as f64 - 99.0) * 15.0)),
            _ => score * 3000.0,
        }
    }

    fn calc_score(studio_rep: u64, score: f64, last_nft_days_ago: u64) -> f64 {
        if last_nft_days_ago == 0 {
            //
        }

        let calc_score = match studio_rep {
            0 => score,
            1 => score * 1.50,
            2..=4 => score * (2.5 + ((studio_rep as f64 - 1.0) * 0.35)),
            5..=9 => score * (3.75 + ((studio_rep as f64 - 4.0) * 0.21)),
            10..=19 => score * (5.0 + ((studio_rep as f64 - 9.0) * 0.45)),
            20..=35 => score * (10.25 + ((studio_rep as f64 - 19.0) * 0.25)),
            36..=59 => score * (15.5 + ((studio_rep as f64 - 35.0) * 0.18)),
            60..=99 => score * (20.75 + ((studio_rep as f64 - 59.0) * 0.18)),
            100..=149 => score * (30.0 + ((studio_rep as f64 - 99.0) * 0.30)),
            _ => score * 50.0,
        };

        calc_score
    }

    pub fn get_price(&self) -> f64 {
        self.price
    }
}

impl NftStudio {
    pub fn new() -> Self {
        NftStudio {
            rep: 0,
            hype: 0.0,
            pop: 0.0,
            nft_drawn: 0,
            last_release: 0,
        }
    }

    fn base_money_per_second(&self) -> f64 {
        let rep = self.rep as f64;
        match rep {
            0.0 => 0.1,
            1.0..=4.0 => rep * (0.25 + (rep - 1.0) * 0.05),
            5.0..=9.0 => rep * (0.5 + (rep - 4.0) * 0.08),
            10.0..=15.0 => rep * (1.0 + (rep - 9.0) * 0.5),
            16.0..=20.0 => rep * (5.0 + (rep - 15.0) * 0.75),
            21.0..=30.0 => rep * (10.0 + (rep - 20.0) * 0.75),
            31.0..=40.0 => rep * (20.0 + (rep - 30.0) * 2.50),
            41.0..=50.0 => rep * (50.0 + (rep - 40.0) * 4.25),
            51.0..=60.0 => rep * (100.0 + (rep - 50.0) * 12.0),
            61.0..=70.0 => rep * (250.0 + (rep - 60.0) * 22.0),
            71.0..=80.0 => rep * (500.0 + (rep - 70.0) * 42.0),
            81.0..=90.0 => rep * (1000.0 + (rep - 80.0) * 175.0),
            91.0..=100.0 => rep * (3000.0 + (rep - 90.0) * 175.0),
            _ => 5000.0 * rep,
        }
    }

    pub fn next_rep(&self) -> u64 {
        let next_rep = if self.rep == 0 {
            25.0
        } else {
            100.0 * 1.15_f64.powi(self.rep as i32)
        };
        next_rep as u64
    }

    fn money_per_second(&self) -> f64 {
        (self.base_money_per_second() * (self.rep).max(1) as f64) + self.nft_drawn as f64 / 2.5
    }

    pub fn money_per_second_adjusted(&self) -> f64 {
        self.money_per_second() * self.popularity()
    }

    pub fn money_per_tick(&self) -> f64 {
        (self.money_per_second() / 20.0) * self.popularity()
    }

    pub fn mint_nft(&mut self, day: u64, name: String, score: f64) -> Nft {
        let last_nft_days_ago = day - self.last_release;

        let nft = Nft::new(name, score, self.rep, last_nft_days_ago);
        self.nft_drawn += 1;
        self.last_release = day;

        let mut new_hype = self.hype + nft.score;

        while new_hype >= self.next_rep() as f64 {
            self.rep += 1;
            new_hype -= self.next_rep() as f64;
        }

        self.hype = new_hype.max(0.0);

        self.pop = self.max_popularity();

        nft
    }

    pub fn max_popularity(&self) -> f64 {
        5760.0 * 6.0
    }

    pub fn popularity(&self) -> f64 {
        self.pop / self.max_popularity()
    }

    pub fn decriment_popularity(&mut self, current_day: u64) -> f64 {
        let last_nft_ago = current_day - self.last_release;

        if last_nft_ago > 0 && self.pop > 0.0 {
            self.pop -= 1.0;
        }
        self.pop = self.pop.max(0.0);

        self.popularity()
    }

    pub fn mint_nft_dry_run(&self, name: String, score: f64, day: u64) -> Nft {
        let last_nft_days_ago = day - self.last_release;
        Nft::new(name, score, self.rep, last_nft_days_ago)
    }
}
