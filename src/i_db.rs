#![allow(dead_code)]
use indexed_db_futures::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;
use web_sys::DomException;

use crate::galaxy_api::GalaxyResponse;
use crate::market::Market;
use crate::mining_rig::MiningRig;
use crate::nft::NftStudio;
use crate::utils::{GalaxySaveDetails, GameTime, PaintUndo, Paused};
use js_sys::JSON;
use wasm_bindgen::JsCast;

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct GameState {
    pub market: Market,
    pub game_time: GameTime,
    pub paused: Paused,
    pub real_time: i64,
    pub selection: Option<Selection>,
    pub mining_rig: MiningRig,
    pub galaxy_save_details: Option<GalaxySaveDetails>,
    pub version: Option<u64>,
    pub nft_studio: Option<NftStudio>,
    pub selection_multi: Option<SelectionMultiList>,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct GalaxyHost {
    pub galaxy: bool,
    pub api_version: u64,
    pub logged_in: bool,
    pub info_check_status: Option<bool>,
    pub info_check_time: Option<f64>,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct GalaxySaveSlot {
    pub slot: u64,
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

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct CmdOutput {
    pub last: f64,
}

impl CmdOutput {
    pub fn new() -> Self {
        CmdOutput { last: 0.0 }
    }

    pub fn set_last(&mut self) {
        let time_now = web_sys::js_sys::Date::new_0();
        let last = time_now.get_time();

        self.last = last;
    }

    pub fn can_next(&self) -> bool {
        let time_now = web_sys::js_sys::Date::new_0();
        let now = time_now.get_time();

        now - self.last > 1000.0
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

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct SelectionMulti {
    pub name: String,
    pub index: usize,
    pub selection_index: usize,
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct SelectionMultiList {
    pub selections: Vec<SelectionMulti>,
    pub max_selectable: u8,
}

impl SelectionMultiList {
    pub fn new() -> Self {
        SelectionMultiList {
            selections: Vec::new(),
            max_selectable: 1,
        }
    }

    fn insert(&mut self, selection: SelectionMulti) {
        self.selections.insert(selection.selection_index, selection);
    }

    fn remove(&mut self, index: usize) {
        self.selections.remove(index);
    }

    pub fn clear(&mut self) {
        self.selections.clear();
    }

    fn is_full(&self) -> bool {
        self.selections.len() as u8 >= self.max_selectable
    }

    pub fn is_selected(&self, index: usize) -> bool {
        self.selections.iter().any(|s| s.index == index)
    }

    pub fn make_selection(&mut self, index: usize, name: &str, do_toggle: bool) {
        if self.is_selected(index) && do_toggle {
            self.unmake_selection(index);
        } else {
            if self.is_full() {
                self.remove(0);
            }

            let mut selection_index = 0;

            for i in 0..self.max_selectable as usize {
                if !self.selections.iter().any(|s| s.selection_index == i) {
                    selection_index = i;
                    break;
                }
            }

            let selection = SelectionMulti {
                index,
                name: name.to_string(),
                selection_index,
            };
            self.insert(selection);
        }

        self.update_ui();
    }

    pub fn unmake_selection(&mut self, index: usize) {
        if let Some(position) = self.selections.iter().position(|s| s.index == index) {
            self.remove(position);
        }
    }

    pub fn update_ui(&self) {
        let window = web_sys::window().expect("should have a window");
        let document = window.document().expect("should have a document");

        let radios = document
            .query_selector_all("input[name='coin-selection']")
            .expect("should have radios");

        for i in 0..radios.length() {
            let radio = radios.get(i).expect("should have radio");
            let radio = radio
                .dyn_into::<web_sys::HtmlInputElement>()
                .expect("should be a radio");
            radio.set_checked(false); // Reset all radios
        }

        let rows = document.query_selector_all("tr").expect("should have rows");
        for i in 0..rows.length() {
            let row = rows.get(i).expect("should have row");
            let row = row.dyn_into::<web_sys::Element>().expect("should be a row");
            row.set_class_name(""); // Clear all row classes
        }

        for selection in &self.selections {
            let coin_name = &selection.name;

            for i in 0..radios.length() {
                let radio = radios.get(i).expect("should have radio");
                let radio = radio
                    .dyn_into::<web_sys::HtmlInputElement>()
                    .expect("should be a radio");

                if radio.id() == *coin_name {
                    radio.set_checked(true);
                }
            }

            for i in 0..rows.length() {
                let row = rows.get(i).expect("should have row");
                let row = row.dyn_into::<web_sys::Element>().expect("should be a row");

                if row.id() == format!("{}-row", coin_name) {
                    row.set_class_name(&format!("selected-{}", selection.index));
                }
            }
        }
    }

    pub fn increment_max_selectable(&mut self) {
        if self.max_selectable < 10 {
            self.max_selectable += 1;
        }
    }

    pub fn get_first_selection(&self) -> Option<&SelectionMulti> {
        self.selections.first()
    }

    pub fn get_selected(&self) -> Vec<SelectionMulti> {
        self.selections.clone()
    }

    pub fn selection_by_index(&self, index: usize) -> Option<&SelectionMulti> {
        self.selections.iter().find(|s| s.selection_index == index)
    }

    pub fn get_upgrade_cost(&self) -> f64 {
        match self.max_selectable {
            1 => 10_000.0,
            2 => 100_000.0,
            3 => 1_000_000.0,
            4 => 10_000_000.0,
            5 => 100_000_000.0,
            6 => 1_000_000_000.0,
            7 => 10_000_000_000.0,
            8 => 100_000_000_000.0,
            _ => 1_000_000_000_000.0,
        }
    }
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

pub async fn set_paint_undo(paint_undo: &PaintUndo) -> JsValue {
    let value: JsValue = serde_wasm_bindgen::to_value(paint_undo).unwrap();
    wasm_set_item("paint_undo", &value).await
}

pub async fn get_paint_undo() -> Result<Option<PaintUndo>, JsValue> {
    let value = get_item("paint_undo").await.map_err(JsValue::from)?;

    let value = match value {
        Some(value) => {
            if value.is_null() {
                None
            } else {
                Some(serde_wasm_bindgen::from_value::<PaintUndo>(value).unwrap())
            }
        }
        None => return Ok(None),
    };

    Ok(value)
}

pub async fn clear_paint_undo() -> JsValue {
    let future = async move {
        set_item("paint_undo", &JsValue::NULL)
            .await
            .map_err(|err| JsValue::from(err))?;
        Ok(JsValue::from(true))
    }
    .await;

    future.unwrap_or_else(|err| err)
}

pub async fn set_cmd_output(cmd_output: &CmdOutput) -> JsValue {
    let value: JsValue = serde_wasm_bindgen::to_value(cmd_output).unwrap();
    wasm_set_item("cmd_output", &value).await
}

pub async fn get_cmd_output() -> Result<Option<CmdOutput>, JsValue> {
    let value = get_item("cmd_output").await.map_err(JsValue::from)?;

    let value = match value {
        Some(value) => {
            if value.is_null() {
                None
            } else {
                Some(serde_wasm_bindgen::from_value::<CmdOutput>(value).unwrap())
            }
        }
        None => return Ok(None),
    };

    Ok(value)
}

pub async fn clear_cmd_output() -> JsValue {
    let future = async move {
        set_item("cmd_output", &JsValue::NULL)
            .await
            .map_err(|err| JsValue::from(err))?;
        Ok(JsValue::from(true))
    }
    .await;

    future.unwrap_or_else(|err| err)
}
