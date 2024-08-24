use dioxus_logger::tracing::info;
use gloo_utils::window;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::from_value;
use std::collections::HashMap;
use wasm_bindgen::JsValue;

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
    pub theme: String,
    pub logged_in: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct SaveListRes {
    pub error: bool,
    pub message: Option<String>,
    pub list: HashMap<String, SaveData>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SaveData {
    pub label: String,
    pub content: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct SaveContentRes {
    pub error: bool,
    pub message: Option<String>,
    pub slot: u32,
    pub label: Option<String>,
    pub content: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct SavedRes {
    pub error: bool,
    pub message: Option<String>,
    pub slot: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct DeletedRes {
    pub error: bool,
    pub message: Option<String>,
    pub slot: u32,
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
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SaveReq {
    pub action: String,
    pub slot: u32,
    pub label: Option<String>,
    pub data: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LoadReq {
    pub action: String,
    pub slot: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DeleteReq {
    pub action: String,
    pub slot: u32,
}

pub fn send_message(data: JsValue) {
    let win_res = window().parent();

    match win_res {
        Ok(win) => match win {
            Some(win) => {
                info!("Sending message to parent window");

                let _ = win.post_message(&data, "https://galaxy.click/");
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

pub fn galaxy_response(js_value: JsValue) {
    match from_value::<GalaxyResponse>(js_value) {
        Ok(response) => match response {
            GalaxyResponse::SaveList(save_list) => {
                info!("Handling save list: {:?}", save_list);
            }
            GalaxyResponse::SaveContent(save_content) => {
                info!("Handling save content: {:?}", save_content);
            }
            GalaxyResponse::Saved(saved) => {
                info!("Handling saved: {:?}", saved);
            }
            GalaxyResponse::Deleted(deleted) => {
                info!("Handling deleted: {:?}", deleted);
            }
            GalaxyResponse::Info(info) => {
                info!("Handling info: {:?}", info);

                // let data = serde_wasm_bindgen::to_value(&SupportsReq {
                //     action: "supports".to_string(),
                //     saving: true,
                //     eval: false,
                // });

                // match data {
                //     Ok(data) => send_message(data),
                //     Err(err) => {
                //         info!("Failed to serialize SupportsReq: {:?}", err);
                //     }
                // }
            }
        },
        Err(err) => {
            info!("Failed to deserialize JsValue: {:?}", err);
        }
    }
}
