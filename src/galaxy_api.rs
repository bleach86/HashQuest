#![allow(dead_code)]
use dioxus_logger::tracing::info;
use gloo_timers::future::TimeoutFuture;
use gloo_utils::window;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::from_value;
use std::collections::HashMap;
use wasm_bindgen::JsValue;

use crate::i_db::{
    get_galaxy_host, get_galaxy_response_queue, get_galaxy_save_list, get_game_state,
    set_galaxy_host, set_galaxy_response_queue, set_galaxy_save_list, GalaxyHost,
    GalaxyResponseQueue, GalaxySaveList, GalaxySaveSlot,
};
use crate::{export_game_state, DO_SAVE, GALAXY_SAVE_DETAILS};

static MAX_MSG_SIZE: u32 = 256_000;
static GALAXY_LABEL_BASE: &str = "HashQuest AutoSave";

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GalaxyResponse {
    SaveList(SaveListRes),
    SaveContent(SaveContentRes),
    Saved(SavedRes),
    Deleted(DeletedRes),
    Info(InfoRes),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct InfoRes {
    pub galaxy: bool,
    pub api_version: u32,
    pub theme_preference: String,
    pub logged_in: bool,
    pub echo: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct SaveListRes {
    pub error: bool,
    pub message: Option<String>,
    pub list: HashMap<String, SaveData>,
    pub echo: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SaveData {
    pub label: String,
    pub content: String,
    pub echo: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct SaveContentRes {
    pub error: bool,
    pub message: Option<String>,
    pub slot: u32,
    pub label: Option<String>,
    pub content: Option<String>,
    pub echo: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct SavedRes {
    pub error: bool,
    pub message: Option<String>,
    pub slot: u32,
    pub echo: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct DeletedRes {
    pub error: bool,
    pub message: Option<String>,
    pub slot: u32,
    pub echo: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SupportsReq {
    pub action: String,
    pub saving: bool,
    pub eval: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SaveListReq {
    pub action: String,
    pub echo: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SaveReq {
    pub action: String,
    pub slot: u32,
    pub label: Option<String>,
    pub data: Option<String>,
    pub echo: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LoadReq {
    pub action: String,
    pub slot: u32,
    pub echo: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeleteReq {
    pub action: String,
    pub slot: u32,
    pub echo: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InfoReq {
    pub action: String,
    pub echo: Option<String>,
}

pub fn send_message(data: JsValue) {
    let win_res = window().top();

    match win_res {
        Ok(win) => match win {
            Some(win) => {
                info!("Sending message to parent window");

                let _ = win.post_message(&data, "https://galaxy.click");
            }
            None => {
                info!("No parent window found");
            }
        },
        Err(err) => {
            info!("Failed to get parent window: {:?}", err);
        }
    }
}

pub async fn galaxy_response(js_value: JsValue) {
    let response_queue = get_galaxy_response_queue().await.unwrap_or_else(|err| {
        info!("Failed to get galaxy response queue: {:?}", err);
        Some(GalaxyResponseQueue::new())
    });

    let mut response_queue = match response_queue {
        Some(response_queue) => response_queue,
        None => {
            info!("Failed to get galaxy response queue");
            return;
        }
    };

    match from_value::<GalaxyResponse>(js_value) {
        Ok(response) => match response {
            GalaxyResponse::SaveList(save_list) => {
                response_queue.insert(GalaxyResponse::SaveList(save_list));
                set_galaxy_response_queue(&response_queue).await;
            }
            GalaxyResponse::SaveContent(save_content) => {
                response_queue.insert(GalaxyResponse::SaveContent(save_content));
                set_galaxy_response_queue(&response_queue).await;
            }
            GalaxyResponse::Saved(saved) => {
                response_queue.insert(GalaxyResponse::Saved(saved));
                set_galaxy_response_queue(&response_queue).await;
            }
            GalaxyResponse::Deleted(deleted) => {
                response_queue.insert(GalaxyResponse::Deleted(deleted));
                set_galaxy_response_queue(&response_queue).await;
            }
            GalaxyResponse::Info(info) => {
                let g_host = GalaxyHost {
                    galaxy: info.galaxy,
                    api_version: info.api_version,
                    logged_in: info.logged_in,
                    info_check_status: Some(true),
                    info_check_time: None,
                };

                set_galaxy_host(&g_host).await;
            }
        },
        Err(err) => {
            info!("Failed to deserialize JsValue: {:?}", err);
        }
    }
}

pub async fn save_list_response(save_list: SaveListRes) {
    if save_list.error {
        info!("Error fetching save list: {:?}", save_list.message);
        return;
    }

    let mut galaxy_save_list = GalaxySaveList::new();

    for (key, value) in save_list.list {
        let slot = match key.parse::<u32>() {
            Ok(slot) => slot,
            Err(err) => {
                info!("Failed to parse slot: {:?}", err);
                continue;
            }
        };

        let galaxy_save_slot = GalaxySaveSlot {
            slot,
            label: Some(value.label),
            content: Some(value.content),
        };

        galaxy_save_list.insert(galaxy_save_slot);
    }

    set_galaxy_save_list(&galaxy_save_list).await;
    loop {
        let galaxy_save_list = get_galaxy_save_list().await.unwrap_or_else(|err| {
            info!("Failed to get galaxy save list: {:?}", err);
            None
        });

        match galaxy_save_list {
            Some(_) => {
                break;
            }
            None => {
                info!("Failed to get galaxy save list, continuing to wait...");
                TimeoutFuture::new(100).await;
            }
        }
    }
}

pub async fn do_cloud_save(save_slot: u32) {
    let game_state_res = get_game_state().await;

    let game_state_opt = match game_state_res {
        Ok(game_state) => game_state,
        Err(_) => None,
    };

    let game_state = match game_state_opt {
        Some(game_state) => game_state,
        None => {
            return;
        }
    };

    // let game_state = export_game_state(&game_state).await;

    let save_data = export_game_state(&game_state)
        .await
        .unwrap_or_else(|| "".to_string());

    if save_data.len() as u32 > MAX_MSG_SIZE {
        info!("Save data too large");
        if let Some(mut save_details) = GALAXY_SAVE_DETAILS() {
            save_details.active = false;
            *GALAXY_SAVE_DETAILS.write() = Some(save_details);
            DO_SAVE.write().save = true;
            info!("Cloud save disabled");
        }

        let win = window();

        let msg = format!("Save data too large for Galaxy.click cloud save.\nMax allowed: 256,000 Characters\nYour save: {} Characters.\nDiasabling Cloud Autosave.", save_data.len());
        let _ = win.alert_with_message(&msg);

        return;
    }

    let data = SaveReq {
        action: "save".to_string(),
        slot: save_slot,
        label: Some(GALAXY_LABEL_BASE.to_string()),
        data: Some(save_data),
        echo: None,
    };

    let js_data = serde_wasm_bindgen::to_value(&data);

    match js_data {
        Ok(js_data) => {
            send_message(js_data);

            TimeoutFuture::new(100).await;

            let res =
                wait_for_response(|response| matches!(response, GalaxyResponse::Saved(_))).await;

            match res {
                Some(GalaxyResponse::Saved(_)) => {}
                _ => {
                    info!("Failed to get saved response");
                }
            }
        }
        Err(err) => {
            info!("Failed to serialize SaveReq: {:?}", err);
        }
    }
}

pub fn fetch_cloud_save(slot: u32) {
    let data: LoadReq = LoadReq {
        action: "load".to_string(),
        slot,
        echo: None,
    };

    let js_data = serde_wasm_bindgen::to_value(&data);

    match js_data {
        Ok(js_data) => send_message(js_data),
        Err(err) => {
            info!("Failed to serialize LoadReq: {:?}", err);
        }
    }
}

pub async fn delete_cloud_save(slot: u32) {
    let data: DeleteReq = DeleteReq {
        action: "delete".to_string(),
        slot,
        echo: None,
    };

    let js_data = serde_wasm_bindgen::to_value(&data);

    match js_data {
        Ok(js_data) => send_message(js_data),
        Err(err) => {
            info!("Failed to serialize DeleteReq: {:?}", err);
        }
    }

    wait_for_response(|response| matches!(response, GalaxyResponse::Deleted(_))).await;
}

pub async fn fetch_save_list() {
    let data: SaveListReq = SaveListReq {
        action: "save_list".to_string(),
        echo: None,
    };

    let js_data = match serde_wasm_bindgen::to_value(&data) {
        Ok(js_data) => js_data,
        Err(err) => {
            info!("Failed to serialize SaveListReq: {:?}", err);
            return;
        }
    };

    send_message(js_data);

    let res = wait_for_response(|response| matches!(response, GalaxyResponse::SaveList(_))).await;

    match res {
        Some(GalaxyResponse::SaveList(save_list)) => {
            save_list_response(save_list).await;
        }
        _ => {
            info!("Failed to get save list response");
        }
    }
}

pub fn galaxy_supports() {
    let data = serde_wasm_bindgen::to_value(&SupportsReq {
        action: "supports".to_string(),
        saving: true,
        eval: false,
    });

    match data {
        Ok(data) => send_message(data),
        Err(err) => {
            info!("Failed to serialize SupportsReq: {:?}", err);
        }
    }
}

pub async fn galaxy_info() {
    let check_time = web_sys::js_sys::Date::new_0();
    let check_time = check_time.get_time();

    info!("Checking galaxy info");

    let data = serde_wasm_bindgen::to_value(&InfoReq {
        action: "info".to_string(),
        echo: Some("sheet".to_string()),
    });

    match data {
        Ok(data) => send_message(data),
        Err(err) => {
            info!("Failed to serialize InfoReq: {:?}", err);
        }
    }

    loop {
        TimeoutFuture::new(100).await;

        let time_now = web_sys::js_sys::Date::new_0();
        let time_now = time_now.get_time();

        if time_now - check_time > 30000.0 {
            info!("Failed to get info response");
            break;
        }

        let galaxy_host = get_galaxy_host().await.unwrap_or_else(|err| {
            info!("Failed to get galaxy host: {:?}", err);
            Some(GalaxyHost::default())
        });

        let galaxy_host = match galaxy_host {
            Some(galaxy_host) => galaxy_host,
            None => {
                info!("Failed to get galaxy host");
                return;
            }
        };

        if galaxy_host.info_check_status == Some(true) {
            break;
        }
    }
}

pub async fn get_galaxy_save_data() -> Option<String> {
    let galaxy_save_list = get_galaxy_save_list().await.unwrap_or_else(|err| {
        info!("Failed to get galaxy save list: {:?}", err);
        None
    });

    let galaxy_save_list = match galaxy_save_list {
        Some(galaxy_save_list) => galaxy_save_list.list,
        None => {
            info!("Failed to get galaxy save list");
            return None;
        }
    };

    let mut save_content = None;

    for save_slot in galaxy_save_list.iter() {
        if save_slot.label == Some(GALAXY_LABEL_BASE.to_string()) {
            save_content = save_slot.content.clone();
        }
    }
    save_content
}

async fn wait_for_response<F>(predicate: F) -> Option<GalaxyResponse>
where
    F: Fn(&GalaxyResponse) -> bool,
{
    let time_now = web_sys::js_sys::Date::new_0();
    let start_time = time_now.get_time();

    loop {
        TimeoutFuture::new(100).await;

        let response_queue = get_galaxy_response_queue().await.unwrap_or_else(|err| {
            info!("Failed to get galaxy response queue: {:?}", err);
            Some(GalaxyResponseQueue::new())
        });

        let mut response_queue = match response_queue {
            Some(response_queue) => response_queue.queue,
            None => {
                info!("Failed to get galaxy response queue");
                return None;
            }
        };

        if response_queue.is_empty() {
            let time_now = web_sys::js_sys::Date::new_0();
            let current_time = time_now.get_time();

            if current_time - start_time > 30000.0 {
                info!("Failed to get response");
                return None;
            }

            continue;
        }

        let mut found_index = None;
        let mut found_response = None;

        for (index, response) in response_queue.iter().enumerate() {
            if predicate(response) {
                found_index = Some(index);
                found_response = Some(response.clone());
                break;
            }
        }

        if let Some(index) = found_index {
            response_queue.remove(index);

            let updated_queue = GalaxyResponseQueue {
                queue: response_queue,
            };
            set_galaxy_response_queue(&updated_queue).await;

            return found_response;
        } else {
            info!("Desired response not found, continuing to wait...");
        }
    }
}

pub async fn find_save_slot() -> Option<u32> {
    let galaxy_save_list = get_galaxy_save_list().await.unwrap_or_else(|err| {
        info!("Failed to get galaxy save list: {:?}", err);
        None
    });

    let galaxy_save_list = match galaxy_save_list {
        Some(galaxy_save_list) => galaxy_save_list.list,
        None => {
            info!("Failed to get galaxy save list");
            return None;
        }
    };

    let mut slots: Vec<u32> = (0..=10).collect();

    for save_slot in galaxy_save_list.iter() {
        if save_slot.label == Some(GALAXY_LABEL_BASE.to_string()) {
            let slot = save_slot.slot;
            return Some(slot);
        }
        slots.retain(|&x| x != save_slot.slot);
    }

    if !slots.is_empty() {
        Some(slots[0])
    } else {
        None
    }
}
