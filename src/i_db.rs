use indexed_db_futures::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;
use web_sys::DomException;

use crate::market::Market;
use crate::mining_rig::MiningRig;
use crate::utils::{GameTime, Paused};
use js_sys::JSON;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct GameState {
    pub market: Market,
    pub game_time: GameTime,
    pub labels: Vec<String>,
    pub series: Vec<Vec<f32>>,
    pub series_labels: Vec<String>,
    pub paused: Paused,
    pub real_time: i64,
    pub selection: Selection,
    pub mining_rig: MiningRig,
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

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
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
                Some(serde_wasm_bindgen::from_value::<GameState>(value).unwrap())
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
