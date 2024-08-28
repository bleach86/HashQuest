#![allow(dead_code)]
use indexed_db_futures::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;
use web_sys::DomException;

use crate::galaxy_api::GalaxyResponse;
use crate::market::Market;
use crate::mining_rig::MiningRig;
use crate::utils::{GalaxySaveDetails, GameTime, Paused};
use js_sys::JSON;

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct GameState {
    pub market: Market,
    pub game_time: GameTime,
    pub paused: Paused,
    pub real_time: i64,
    pub selection: Selection,
    pub mining_rig: MiningRig,
    pub galaxy_save_details: Option<GalaxySaveDetails>,
    pub version: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct GalaxyHost {
    pub galaxy: bool,
    pub api_version: u32,
    pub logged_in: bool,
    pub info_check_status: Option<bool>,
    pub info_check_time: Option<f64>,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct GalaxySaveSlot {
    pub slot: u32,
    pub label: Option<String>,
    pub content: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct GalaxySaveList {
    pub list: Vec<GalaxySaveSlot>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GalaxyResponseQueue {
    pub queue: Vec<GalaxyResponse>,
}

impl GalaxyResponseQueue {
    pub fn new() -> Self {
        GalaxyResponseQueue { queue: Vec::new() }
    }

    pub fn insert(&mut self, response: GalaxyResponse) {
        self.queue.push(response);
    }
}

impl GalaxySaveList {
    pub fn new() -> Self {
        GalaxySaveList { list: Vec::new() }
    }

    pub fn insert(&mut self, slot: GalaxySaveSlot) {
        self.list.push(slot);
    }
}

impl GameState {
    pub fn to_string(&self) -> String {
        serde_wasm_bindgen::to_value(self)
            .map(|value| JSON::stringify(&value).unwrap())
            .unwrap()
            .into()
    }
}

pub fn game_state_from_string(json: &str) -> Result<GameState, JsValue> {
    let js_value = JSON::parse(json)?;

    serde_wasm_bindgen::from_value::<GameState>(js_value)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct Selection {
    pub index: Option<usize>,
    pub name: Option<String>,
}

const DB_NAME: &str = "HashQuestDB";
const OBJECT_STORE_NAME: &str = "HashQuestStore";
const DB_VERSION: u32 = 1;

pub async fn open_db() -> Result<IdbDatabase, DomException> {
    let mut db_req: OpenDbRequest = IdbDatabase::open_u32(DB_NAME, DB_VERSION)?;
    db_req.set_on_upgrade_needed(Some(|evt: &IdbVersionChangeEvent| -> Result<(), JsValue> {
        evt.db().create_object_store(OBJECT_STORE_NAME)?;
        Ok(())
    }));

    db_req.await
}

pub async fn set_item(key: &str, value: &JsValue) -> Result<(), DomException> {
    let db = open_db().await?;
    let tx = db.transaction_on_one_with_mode(OBJECT_STORE_NAME, IdbTransactionMode::Readwrite)?;
    let store = tx.object_store(OBJECT_STORE_NAME)?;

    store.put_key_val_owned(key, value)?;
    tx.await.into_result()?;
    Ok(())
}

pub async fn get_item(key: &str) -> Result<Option<JsValue>, DomException> {
    let db = open_db().await?;
    let tx = db.transaction_on_one(OBJECT_STORE_NAME)?;
    let store = tx.object_store(OBJECT_STORE_NAME)?;

    let value: Option<JsValue> = store.get_owned(key)?.await?;
    Ok(value)
}

pub async fn wasm_set_item(key: &str, value: &JsValue) -> JsValue {
    let future = async move {
        set_item(key, value)
            .await
            .map_err(|err| JsValue::from(err))?;
        Ok(JsValue::from(true))
    }
    .await;

    future.unwrap_or_else(|err| err)
}

pub async fn get_game_state() -> Result<Option<GameState>, JsValue> {
    let value = get_item("game_state").await.map_err(JsValue::from)?;
    let value = match value {
        Some(value) => {
            if value.is_null() {
                None
            } else {
                let mut game_state = serde_wasm_bindgen::from_value::<GameState>(value).unwrap();

                game_state.market.truncate_prices();
                Some(game_state)
            }
        }
        None => return Ok(None),
    };

    Ok(value)
}

pub async fn set_game_state(game_state: &GameState) -> JsValue {
    let value: JsValue = serde_wasm_bindgen::to_value(game_state).unwrap();
    wasm_set_item("game_state", &value).await
}

pub async fn clear_game_state() -> JsValue {
    let future = async move {
        set_item("game_state", &JsValue::NULL)
            .await
            .map_err(|err| JsValue::from(err))?;
        Ok(JsValue::from(true))
    }
    .await;

    future.unwrap_or_else(|err| err)
}

pub async fn set_seen_welcome() -> JsValue {
    let future = async move {
        set_item("seen_welcome", &JsValue::from(true))
            .await
            .map_err(|err| JsValue::from(err))?;
        Ok(JsValue::from(true))
    }
    .await;

    future.unwrap_or_else(|err| err)
}

pub async fn get_seen_welcome() -> Result<bool, JsValue> {
    let value = get_item("seen_welcome").await.map_err(JsValue::from)?;

    let value = match value {
        Some(value) => {
            if value.is_null() {
                false
            } else {
                value.as_bool().unwrap()
            }
        }
        None => false,
    };

    Ok(value)
}

pub async fn set_galaxy_host(galaxy_host: &GalaxyHost) -> JsValue {
    let value: JsValue = serde_wasm_bindgen::to_value(galaxy_host).unwrap();
    wasm_set_item("galaxy_host", &value).await
}

pub async fn get_galaxy_host() -> Result<Option<GalaxyHost>, JsValue> {
    let value = get_item("galaxy_host").await.map_err(JsValue::from)?;

    let value = match value {
        Some(value) => {
            if value.is_null() {
                None
            } else {
                Some(serde_wasm_bindgen::from_value::<GalaxyHost>(value).unwrap())
            }
        }
        None => return Ok(None),
    };

    Ok(value)
}

pub async fn clear_galaxy_host() -> JsValue {
    let future = async move {
        set_item("galaxy_host", &JsValue::NULL)
            .await
            .map_err(|err| JsValue::from(err))?;
        Ok(JsValue::from(true))
    }
    .await;

    future.unwrap_or_else(|err| err)
}

pub async fn set_galaxy_save_list(galaxy_save_list: &GalaxySaveList) -> JsValue {
    let value: JsValue = serde_wasm_bindgen::to_value(galaxy_save_list).unwrap();
    wasm_set_item("galaxy_save_list", &value).await
}

pub async fn get_galaxy_save_list() -> Result<Option<GalaxySaveList>, JsValue> {
    let value = get_item("galaxy_save_list").await.map_err(JsValue::from)?;

    let value = match value {
        Some(value) => {
            if value.is_null() {
                None
            } else {
                Some(serde_wasm_bindgen::from_value::<GalaxySaveList>(value).unwrap())
            }
        }
        None => return Ok(None),
    };

    Ok(value)
}

pub async fn clear_galaxy_save_list() -> JsValue {
    let future = async move {
        set_item("galaxy_save_list", &JsValue::NULL)
            .await
            .map_err(|err| JsValue::from(err))?;
        Ok(JsValue::from(true))
    }
    .await;

    future.unwrap_or_else(|err| err)
}

pub async fn set_galaxy_response_queue(galaxy_response_queue: &GalaxyResponseQueue) -> JsValue {
    let value: JsValue = serde_wasm_bindgen::to_value(galaxy_response_queue).unwrap();
    wasm_set_item("galaxy_response_queue", &value).await
}

pub async fn get_galaxy_response_queue() -> Result<Option<GalaxyResponseQueue>, JsValue> {
    let value = get_item("galaxy_response_queue")
        .await
        .map_err(JsValue::from)?;

    let value = match value {
        Some(value) => {
            if value.is_null() {
                None
            } else {
                Some(serde_wasm_bindgen::from_value::<GalaxyResponseQueue>(value).unwrap())
            }
        }
        None => return Ok(None),
    };

    Ok(value)
}

pub async fn clear_galaxy_response_queue() -> JsValue {
    let future = async move {
        set_item("galaxy_response_queue", &JsValue::NULL)
            .await
            .map_err(|err| JsValue::from(err))?;
        Ok(JsValue::from(true))
    }
    .await;

    future.unwrap_or_else(|err| err)
}
