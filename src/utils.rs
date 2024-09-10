#![allow(dead_code)]
use gloo_utils::window;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use wasm_bindgen::JsCast;

use crate::crypto_coin::CryptoCoin;
use crate::i_db::{get_cmd_output, set_cmd_output, CmdOutput};

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct CanvasSize {
    pub width: f64,
    pub height: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct ConfirmModal {
    pub show: bool,
    pub msg: String,
    pub confirm: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct TpsCounter {
    pub tps: f64,
    pub target_tps: f64,
    pub delay: u32,
    start_time: f64,
    last_tick_time: f64,
    tick_times: Vec<f64>,
    window_duration: f64,
    is_paused: bool,
}

impl TpsCounter {
    pub fn new(window_duration_secs: f64, target_tps: f64) -> TpsCounter {
        let time_now = web_sys::js_sys::Date::new_0();
        let now = time_now.get_time();
        TpsCounter {
            tps: 0.0,
            target_tps,
            delay: 50,
            start_time: now,
            last_tick_time: now,
            tick_times: Vec::new(),
            window_duration: window_duration_secs * 1000.0, // Convert to milliseconds
            is_paused: false,
        }
    }

    pub fn tick(&mut self) {
        let time_now = web_sys::js_sys::Date::new_0();
        let current_time = time_now.get_time();

        if !self.is_paused {
            self.tick_times.push(current_time);

            self.tick_times
                .retain(|&time| current_time - time <= self.window_duration);

            let elapsed_window_time = (self.tick_times.last().unwrap_or(&current_time)
                - self.tick_times.first().unwrap_or(&current_time))
                / 1000.0;
            if elapsed_window_time > 0.0 {
                self.tps = self.tick_times.len() as f64 / elapsed_window_time;
            }

            if self.tps > 0.0 {
                let delay_ms =
                    ((1.0 / self.target_tps) * 1000.0 - (1.0 / self.tps) * 1000.0).max(0.0);
                self.delay = delay_ms as u32;
            } else {
                self.delay = (1.0 / self.target_tps * 1000.0) as u32;
            }

            self.last_tick_time = current_time;
        }
    }

    pub fn set_paused(&mut self, paused: bool) {
        if paused {
            self.tick_times.clear();
            self.is_paused = true;
        } else {
            self.last_tick_time = web_sys::js_sys::Date::new_0().get_time() as f64;
            self.is_paused = false;
            self.delay = 50;
        }
    }
}
#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
    pub color: String,
    pub bg_color: String,
    pub line_width: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct PaintUndo {
    pub current_path: Vec<Position>,
    pub paths: Vec<Vec<Position>>,
    pub undo_paths: Vec<Vec<Position>>,
}

impl PaintUndo {
    pub fn new() -> Self {
        PaintUndo {
            current_path: Vec::new(),
            paths: Vec::new(),
            undo_paths: Vec::new(),
        }
    }

    pub fn undo(&mut self) {
        if let Some(path) = self.paths.pop() {
            self.undo_paths.push(path);
        }
    }

    pub fn redo(&mut self) {
        if let Some(path) = self.undo_paths.pop() {
            self.paths.push(path);
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.paths.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.undo_paths.is_empty()
    }

    pub fn clear(&mut self) {
        self.paths.clear();
        self.undo_paths.clear();
        self.current_path.clear();
    }

    pub fn add_path(&mut self) {
        if !self.current_path.is_empty() {
            self.paths.push(self.current_path.clone());
            self.current_path.clear();
        }
    }

    pub fn add_position(&mut self, position: Position) {
        self.current_path.push(position);

        if !self.undo_paths.is_empty() {
            self.undo_paths.clear();
        }
    }

    pub fn calculate_score(&self, canvas_size: &CanvasSize) -> f64 {
        let mut score = 0.0;
        let mut unique_colors = HashSet::new();

        let canvas_width = canvas_size.width;
        let canvas_height = canvas_size.height;

        let canvas_area = canvas_width * canvas_height;

        let background_color = if self.paths.is_empty() {
            "#ffffff".to_string()
        } else {
            match self.paths.last() {
                Some(path) => match path.last() {
                    Some(position) => position.bg_color.clone(),
                    None => "#ffffff".to_string(),
                },
                None => "#ffffff".to_string(),
            }
        };

        for path in &self.paths {
            if path.is_empty() {
                continue;
            }

            let path_color = path[0].color.clone();
            let path_line_width = path[0].line_width.clone();

            if path_color == background_color {
                continue;
            }

            unique_colors.insert(path_color);

            let mut path_length = 0.0;
            for i in 0..(path.len() - 1) {
                let current = &path[i];
                let next = &path[i + 1];

                if current.x > canvas_width
                    || current.y > canvas_height
                    || next.x > canvas_width
                    || next.y > canvas_height
                {
                    continue;
                }

                if current.x < 0.0 || current.y < 0.0 || next.x < 0.0 || next.y < 0.0 {
                    continue;
                }

                let distance = ((next.x - current.x).powi(2) + (next.y - current.y).powi(2)).sqrt();
                path_length += distance;
            }

            score += path_length * path_line_width;
        }

        let color_multiplier = unique_colors.len() as f64;

        score = score.min(canvas_area * 20.0);

        score *= color_multiplier;

        score / 1000.0
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Painting {
    pub name: String,
    pub colors: Vec<PaintColor>,
    pub background_color: PaintColor,
    pub width: f64,
    pub height: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct PaintColor {
    pub name: String,
    pub count: u64,
}

impl PaintColor {
    pub fn new(name: &str) -> Self {
        PaintColor {
            name: name.to_string(),
            count: 1,
        }
    }
}

impl Painting {
    pub fn new(background_color: &str, width: f64, height: f64) -> Self {
        Painting {
            name: String::new(),
            colors: Vec::new(),
            background_color: PaintColor::new(background_color),
            width,
            height,
        }
    }

    pub fn set_bg_color(&mut self, color: String) {
        self.background_color = PaintColor::new(&color);
    }

    pub fn do_paint(&mut self, color: String) {
        let index = self.colors.iter().position(|c| c.name == color);

        match index {
            Some(i) => {
                self.colors[i].count += 1;
            }
            None => {
                self.colors.push(PaintColor::new(&color));
            }
        }
    }

    fn get_area(&self) -> f64 {
        self.width * self.height
    }

    fn get_painted_pixels_total(&self) -> u64 {
        self.colors
            .iter()
            .filter(|c| c.name != self.background_color.name)
            .map(|c| c.count)
            .sum()
    }

    fn get_painted_pixels(&self, color: &str) -> u64 {
        match self.colors.iter().find(|c| c.name == color) {
            Some(c) => c.count,
            None => 0,
        }
    }

    fn get_colors_count(&self) -> u64 {
        self.colors.len() as u64
    }

    fn get_color_percentage(&self, color: &str) -> f64 {
        let total = self.get_painted_pixels_total();
        let count = self.get_painted_pixels(color);

        (count as f64 / total as f64) * 100.0
    }

    fn get_coverage(&self) -> f64 {
        let total = self.get_area();
        let painted = self.get_painted_pixels_total();

        (painted as f64 / total as f64) * 100.0
    }

    pub fn get_painting_score(&self) -> f64 {
        let colors = self.get_colors_count();
        let coverage = self.get_coverage();

        (colors as f64 * coverage) / 100.0
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct GalaxySaveDetails {
    pub slot: Option<u64>,
    pub active: bool,
    pub save_interval: u64,
    pub last_save: f64,
    pub force_save: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct GalaxyLoadingModal {
    pub show: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct ImportExportModal {
    pub show: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct BuyModal {
    pub show: bool,
    pub coin: Option<CryptoCoin>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct HelpModal {
    pub show: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct WelcomeModal {
    pub show: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Paused {
    pub paused: bool,
    pub btn_text: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct DoSave {
    pub save: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct CatchupModal {
    pub show: bool,
    pub total_sim: i64,
    pub current_sim: i64,
    pub cancel: bool,
    pub eta: String,
    pub speed_up: f32,
}

impl CatchupModal {
    pub fn new() -> Self {
        CatchupModal {
            show: false,
            total_sim: 0,
            current_sim: 0,
            cancel: false,
            eta: "Calculating...".to_string(),
            speed_up: 1.0,
        }
    }

    pub fn toggle(&mut self) {
        self.show = !self.show;
    }
}

impl Paused {
    pub fn new() -> Self {
        Paused {
            paused: false,
            btn_text: "Pause".to_string(),
        }
    }

    pub fn toggle(&mut self) {
        self.paused = !self.paused;
        self.btn_text = if self.paused {
            "Resume".to_string()
        } else {
            "Pause".to_string()
        };
    }
}

pub fn rand_from_range(range: std::ops::Range<f64>) -> f64 {
    let mut array = [0u8; 8];
    let window = window();
    let crypto = window.crypto().expect("should have crypto support");

    crypto
        .get_random_values_with_u8_array(&mut array)
        .expect("should be able to get random values");

    let random_int = u64::from_le_bytes(array);
    let random_float = (random_int as f64) / (u64::MAX as f64);

    random_float * (range.end - range.start) + range.start
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GameTime {
    pub day: u64,
    pub hour: u8,
    pub minute: u8,
}

impl GameTime {
    pub fn new() -> Self {
        GameTime {
            day: 0,
            hour: 0,
            minute: 0,
        }
    }

    pub fn increment(&mut self) {
        self.minute += 1;
        if self.minute >= 60 {
            self.minute = 0;
            self.hour += 1;
            if self.hour >= 24 {
                self.hour = 0;
                self.day += 1;
            }
        }
    }

    fn minutes_to_midnight(&self) -> u64 {
        let current_hour = self.hour;
        let current_minute = self.minute;
        let minutes_in_hour = 60;
        let hours_in_day = 24;

        let minutes_left_in_hour = minutes_in_hour - current_minute;
        let hours_left_in_day = hours_in_day - current_hour;

        let minutes_left_in_day = minutes_left_in_hour + (hours_left_in_day * minutes_in_hour);

        minutes_left_in_day as u64
    }

    pub fn ticks_until_day(&self, day: u64) -> u64 {
        let minutes_left_in_day = self.minutes_to_midnight();
        let ticks_left_in_day = minutes_left_in_day * 4;

        let days_left = day - self.day;

        let ticks_left = days_left * 24 * 60 * 4;

        (ticks_left + ticks_left_in_day).max(0)
    }

    pub fn increment_15(&mut self) {
        self.minute += 15;
        if self.minute >= 60 {
            self.minute = 0;
            self.hour += 1;
            if self.hour >= 24 {
                self.hour = 0;
                self.day += 1;
            }
        }
    }

    pub fn increment_day(&mut self) {
        self.day += 1;
    }
}

pub fn get_season(day: u64) -> f64 {
    let day_in_year = if day == 0 { 0 } else { (day - 1) % 360 + 1 };

    match day_in_year {
        ..=90 => 20000.0,
        91..=180 => 14000.0,
        181..=270 => 16000.0,
        271..=360 => 18000.0,
        _ => 0.0,
    }
}

pub async fn command_line_output(msg: &str) {
    let cmd_timeout_opt = get_cmd_output().await.unwrap_or(Some(CmdOutput::default()));
    let mut cmd_timeout = cmd_timeout_opt.unwrap_or(CmdOutput::default());

    if !cmd_timeout.can_next() {
        return;
    }

    let window = window();
    let document = window.document().expect("should have document");

    let command_line = document
        .get_element_by_id("command-line")
        .expect("should have command line");

    let command_line = command_line
        .dyn_into::<web_sys::HtmlTextAreaElement>()
        .expect("should be a textarea");

    let console_history = command_line.value();
    let mut console_history = console_history.split("\n").collect::<Vec<&str>>();

    while console_history.len() > 1024 {
        console_history.remove(0);
    }

    let console_history = console_history.join("\n");

    let new_value = if console_history.is_empty() {
        msg.to_string()
    } else {
        format!("{}\n{}", console_history, msg)
    };

    command_line.set_value(&new_value);

    command_line.set_scroll_top(command_line.scroll_height());

    cmd_timeout.set_last();

    set_cmd_output(&cmd_timeout).await;
}

pub fn truncate_price(value: f64) -> f64 {
    let factor = 10f64.powi(5); // 10^5 = 100000
    (value * factor).round() / factor
}
