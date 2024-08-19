#![allow(dead_code)]
use gloo_utils::window;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;

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

pub fn rand_from_range(range: std::ops::Range<f32>) -> f32 {
    let mut array = [0u8; 4];
    let window = window();
    let crypto = window.crypto().expect("should have crypto support");

    crypto
        .get_random_values_with_u8_array(&mut array)
        .expect("should be able to get random values");

    let random_int = u32::from_le_bytes(array);
    let random_float = (random_int as f32) / (u32::MAX as f32);

    random_float * (range.end - range.start) + range.start
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GameTime {
    pub day: u32,
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

pub fn get_season(day: u32) -> f64 {
    let day_in_year = if day == 0 { 0 } else { (day - 1) % 360 + 1 };

    match day_in_year {
        ..=90 => 200.0,
        91..=180 => 140.0,
        181..=270 => 160.0,
        271..=360 => 180.0,
        _ => 0.0,
    }
}

pub fn command_line_output(msg: &str) {
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
}
