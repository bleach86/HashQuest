#![allow(non_snake_case)]

use dioxus::html::input_data::MouseButton;
use dioxus::prelude::*;
use dioxus_charts::LineChart;
use dioxus_logger::tracing::{info, Level};
use gloo_timers::future::TimeoutFuture;
use gloo_utils::window;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::{spawn_local, JsFuture};

mod i_db;
use i_db::{
    clear_game_state, clear_paint_undo, game_state_from_string, get_galaxy_host, get_game_state,
    get_paint_undo, get_seen_welcome, set_galaxy_host, set_galaxy_response_queue,
    set_galaxy_save_list, set_game_state, set_paint_undo, set_seen_welcome, GalaxyHost,
    GalaxyResponseQueue, GalaxySaveList, GameState, SelectionMultiList,
};

mod crypto_coin;
mod galaxy_api;
mod market;
mod mining_rig;
mod nft;
mod utils;

use crypto_coin::CryptoCoin;
use galaxy_api::{
    delete_cloud_save, do_cloud_save, fetch_save_list, find_save_slot, galaxy_info,
    galaxy_response, get_galaxy_save_data,
};
use market::{
    cull_market, gen_random_coin_with_set_index, replace_coin, GAME_TIME, MARKET,
    MAX_SERIES_LENGTH, SELECTION,
};
use mining_rig::MINING_RIG;
use utils::{
    command_line_output, BuyModal, CatchupModal, ConfirmModal, DoSave, GalaxyLoadingModal,
    GalaxySaveDetails, GameTime, HelpModal, ImportExportModal, PaintUndo, Paused, Position,
    TpsCounter, WelcomeModal,
};

use nft::NftStudio;

// Urls are relative to your Cargo.toml file
const _TAILWIND_URL: &str = manganis::mg!(file("public/tailwind.css"));

static IS_PAUSED: GlobalSignal<Paused> = Signal::global(|| Paused::new());
static DO_SAVE: GlobalSignal<DoSave> = Signal::global(|| DoSave::default());
static CATCHUP_MODAL: GlobalSignal<CatchupModal> = Signal::global(|| CatchupModal::default());
static HELP_MODAL: GlobalSignal<HelpModal> = Signal::global(|| HelpModal::default());
static WELCOME_MODAL: GlobalSignal<WelcomeModal> = Signal::global(|| WelcomeModal::default());
static BUY_MODAL: GlobalSignal<BuyModal> = Signal::global(|| BuyModal::default());
static IMPORT_EXPORT_MODAL: GlobalSignal<ImportExportModal> =
    Signal::global(|| ImportExportModal::default());
static GALAXY_LOADING_MODAL: GlobalSignal<GalaxyLoadingModal> =
    Signal::global(|| GalaxyLoadingModal::default());
static GALAXY_SAVE_DETAILS: GlobalSignal<Option<GalaxySaveDetails>> = Signal::global(|| None);
static NFT_STUDIO: GlobalSignal<NftStudio> = Signal::global(|| NftStudio::new());

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    // Init logger
    dioxus_logger::init(Level::INFO).expect("failed to init logger");
    info!("starting app");
    launch(App);
}

#[component]
fn App() -> Element {
    // State to store the series data for the chart
    let series: Signal<Vec<Vec<f64>>> = use_signal(|| vec![vec![]]);
    let labels: Signal<Vec<String>> = use_signal(|| vec![String::new()]);

    let series_labels: Signal<Vec<String>> = use_signal(|| Vec::new());

    let mut game_ready: Signal<bool> = use_signal(|| false);

    let ticks_per_second: Signal<TpsCounter> = use_signal(|| TpsCounter::new(10.0, 10.0));
    let confirm_modal = use_signal(|| ConfirmModal::default());

    let selected_tab: Signal<String> = use_signal(|| "mining-0".to_string());

    let listener = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
        let msg_origin: String = event.origin();

        if msg_origin == "https://galaxy.click" {
            let data = event.data();

            spawn_local(async move {
                galaxy_response(data).await;
            });
        }
    }) as Box<dyn FnMut(_)>);

    let galaxy_db_init = use_future(move || async move {
        let galaxy_host = GalaxyHost::default();
        let galaxy_list = GalaxySaveList::new();
        let galaxy_queue = GalaxyResponseQueue { queue: Vec::new() };

        set_galaxy_host(&galaxy_host).await;
        set_galaxy_save_list(&galaxy_list).await;
        set_galaxy_response_queue(&galaxy_queue).await;
    });

    use_effect(move || {
        let win = window();

        let win_self = win.self_();
        let win_top_res = win.top();

        let win_top = match win_top_res {
            Ok(win_top) => match win_top {
                Some(win_top) => win_top,
                None => win_self.clone(),
            },
            Err(_) => win_self.clone(),
        };

        if win_self != win_top {
            let win = window();
            let document = win.document();
            match document {
                Some(document) => {
                    let referrer = document.referrer();

                    match referrer.as_str() {
                        "" | "https://galaxy.click/" => {
                            let win = window();

                            let res = win.add_event_listener_with_callback(
                                "message",
                                listener.as_ref().unchecked_ref(),
                            );

                            GALAXY_LOADING_MODAL.write().show = true;

                            match res {
                                Ok(_) => {
                                    use_future(move || async move {
                                        info!("Added message listener for galaxy.click");

                                        loop {
                                            if galaxy_db_init.finished() {
                                                break;
                                            }
                                            TimeoutFuture::new(100).await;
                                        }

                                        galaxy_info().await;
                                        game_ready.set(true);
                                    });
                                }
                                Err(_) => {
                                    info!("Failed to add message listener for galaxy.click");
                                }
                            }
                        }
                        _ => {}
                    }
                }
                None => {}
            }
        } else {
            game_ready.set(true);
        }

        use_future(move || {
            let mut series = series.clone();
            let mut labels = labels.clone();
            let mut series_labels = series_labels.clone();
            let game_ready = game_ready.clone();
            let mut ticks_per_second = ticks_per_second.clone();

            async move {
                loop {
                    if game_ready() {
                        break;
                    }
                    TimeoutFuture::new(100).await;
                }

                game_loop(
                    &mut series,
                    &mut labels,
                    &mut series_labels,
                    &mut ticks_per_second,
                )
                .await;
            }
        });
    });

    use_effect(move || {
        SELECTION().update_ui();
    });

    rsx! {
        link { rel: "stylesheet", href: "/98css/98.css" }
        link { rel: "stylesheet", href: "main.css?v=1.1" }
        div {
            id: "content",
            class: "flex flex-col items-center justify-center relative",
            style: "margin-top: 15px;margin-bottom: 15px;",

            div { class: "grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-4 px-2",

                div { class: "grid grid-cols-1 w-full gap-4 order-3 sm:order-3 xl:order-1",
                    div { class: "flex-1 xl:order-2", Upgrades {} }
                    div { class: "flex-1 xl:order-1",
                        Coins { series_labels: series_labels.clone(), series: series.clone(), labels: labels.clone() }
                    }
                }

                div { class: "grid grid-cols-1 w-full gap-4 order-1 xl:order-2",
                    div { class: "flex-1",
                        Header { ticks_per_second: ticks_per_second.clone(), selected_tab: selected_tab.clone() }
                    }
                    div { class: "flex-1",
                        HeaderBelow { selected_tab: selected_tab.clone() }
                    }
                }
                div { class: "grid grid-cols-1 w-full gap-4 order-2 sm:order-2 xl:order-3",
                    div { class: "flex-1",
                        Chart { labels, series, series_labels }
                    }
                    div { class: "flex-1", CommandLine {} }
                }
                div { class: "grid grid-cols-1 w-full gap-4 order-4",
                    div { class: "flex-1",
                        Paint { confirm_modal: confirm_modal.clone() }
                    }
                }
            }
            Footer {}
        }
        Modal { confirm_modal: confirm_modal.clone() }
        CatchupModal {}
        HelpModal {}
        WelcomeModal {}
        BuyModal {
            series_labels: series_labels.clone(),
            series: series.clone(),
            labels: labels.clone(),
            confirm_modal: confirm_modal.clone()
        }
        ImportExportModal { series_labels: series_labels.clone(), series: series.clone(), labels: labels.clone() }
        GalaxyLoadingModal {}
        ConfirmModal { confirm_modal: confirm_modal.clone() }
    }
}

#[component]
fn Coins(
    series_labels: Signal<Vec<String>>,
    series: Signal<Vec<Vec<f64>>>,
    labels: Signal<Vec<String>>,
) -> Element {
    let mut show_inactive = use_signal(|| false);

    let toggel_inactive = {
        move |_| {
            *show_inactive.write() = !show_inactive();
        }
    };

    let new_coin_ready = || {
        let new_coin_cooldown = MINING_RIG().get_new_coin_cooldown();

        if new_coin_cooldown == 0 {
            "Now!".to_string()
        } else {
            let seconds = new_coin_cooldown as f64 / 20.0;
            format!("{seconds:.2}s")
        }
    };

    let profit_value = |coin: &CryptoCoin| {
        let pf = coin.profit_factor;

        if pf >= 1_000_000.0 {
            format_comma_seperator(pf, 0)
        } else {
            format_comma_seperator(pf, 2)
        }
    };

    let has_balance = { !MARKET().has_balance() };

    rsx! {
        div { class: "items-center justify-center container",
            div {
                class: "aspect-w-1 aspect-h-1 window ",
                style: "max-width: 403px;",
                div { class: "title-bar",
                    div { class: "title-bar-text", "Coins To Mine" }
                    div { class: "title-bar-controls",
                        button {
                            class: "close",
                            aria_label: "Close",
                            onclick: |_| {
                                info!("Closing window");
                            },
                            ""
                        }
                    }
                }
                div { class: "window-body", style: "overflow: auto;",

                    div { class: "sunken-panel", style: "",

                        table { class: "interactive w-full noselect",
                            thead { class: "mb-3 fixed-header", style: "",
                                tr {
                                    //th { "Select" }
                                    th { "Coin" }
                                    th { "Curent Price" }
                                    th { "Balance" }
                                    th { "$ / Min" }
                                    th { "Age" }
                                    th { "Market" }
                                }
                            }
                            tbody {
                                id: "coins-table",
                                class: "p-5",
                                style: "height: 262px; overflow: auto;",
                                for coin in MARKET().index_sorted_coins(show_inactive()) {
                                    tr {
                                        id: format!("{}-row", coin.name),
                                        onclick: {
                                            let coin = coin.clone();
                                            move |_| {
                                                let coin_name = coin.name.clone();
                                                let coin_index = coin.index;
                                                info!("selections {:?}", SELECTION().clone());
                                                SELECTION.write().make_selection(coin_index, &coin_name, true);
                                                DO_SAVE.write().save = true;
                                            }
                                        },
                                        td { style: "padding: 3px;display:none;",
                                            div {
                                                class: "field-row flex flex-row justify-center",
                                                style: "position:relative;top:-5px;",
                                                input {
                                                    class: "",
                                                    id: coin.clone().name,
                                                    r#type: "radio",
                                                    name: "coin-selection",
                                                    value: "{coin.name}"
                                                }
                                                label {
                                                    class: "",
                                                    r#for: coin.clone().name
                                                }
                                            }
                                        }
                                        td { style: "padding: 3px;", "{coin.name}" }
                                        td { style: "padding: 3px;",
                                            "${format_comma_seperator(coin.current_price, 2)}"
                                        }
                                        td { style: "padding: 3px;font-family: 'Courier New', Courier, monospace;",
                                            "{format_comma_seperator(coin.balance,5)}"
                                        }
                                        td { style: "padding: 3px;", "${profit_value(&coin)}" }
                                        td { style: "padding: 3px;", "{coin.get_age()}" }
                                        if coin.active {
                                            td { style: "padding: 3px;",
                                                div { class: "flex flex-row justify-center",
                                                    button {
                                                        class: "sell-btn",
                                                        onclick: {
                                                            let coin = coin.clone();
                                                            move |event| {
                                                                event.stop_propagation();
                                                                BUY_MODAL.write().coin = Some(coin.clone());
                                                                BUY_MODAL.write().show = true;
                                                            }
                                                        },
                                                        "Market"
                                                    }
                                                }
                                            }
                                        } else {
                                            td { style: "padding: 3px;",
                                                button {
                                                    disabled: true,
                                                    class: "sell-btn",
                                                    "Market"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    div {
                        class: "flex flex-row",
                        style: "justify-content:end;margin-top:10px;",
                        button {
                            disabled: has_balance,
                            onclick: move |_| {
                                MARKET.write().sell_all_coins();
                            },
                            "Sell All"
                        }
                    }

                    div { class: "status-bar", style: "margin-top:10px;",
                        p {
                            class: "status-bar-field p-1 font-mono p-2",
                            style: "padding:4px;",
                            ""
                            input {
                                id: "show-inactive",
                                class: "",
                                style: "",
                                r#type: "checkbox",
                                onchange: toggel_inactive
                            }
                            label { class: "", r#for: "show-inactive", "Show Inactive" }
                        }
                        p {
                            class: "status-bar-field p-1 font-mono p-2",
                            style: "padding:4px;",
                            "New Ready in: {new_coin_ready()}"
                        }

                        p {
                            class: "status-bar-field p-1 p-2",
                            style: "padding:4px;",
                            "Selected: {SELECTION().get_selected().len()}"
                            span { style: "margin-left:1ch;margin-right:1ch;", "/" }
                            span { "{SELECTION().max_selectable}" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn Footer() -> Element {
    let current_year = {
        let date_now = web_sys::js_sys::Date::new_0();
        let year = date_now.get_full_year();
        year
    };

    rsx! {
        div { class: "",
            p { style: "text-align:center;margin-top: 15px;",
                "HashQuest {VERSION} | \u{00a9} {current_year} HashQuest.lol"
            }
        }
    }
}

#[component]
pub fn HeaderBelow(selected_tab: Signal<String>) -> Element {
    let get_details_tab_class = {
        let can_upgrade_auto_fill = {
            if MARKET().bank.balance > MINING_RIG().get_auto_power_fill_upgrade_cost()
                && MINING_RIG().get_auto_power_fill_level() < 13
            {
                true
            } else {
                false
            }
        };

        if !can_upgrade_rig() || can_upgrade_auto_fill {
            "rig-tab upgradeable"
        } else {
            "rig-tab"
        }
    };

    rsx! {
        div { class: "items-center justify-center container",
            div { class: "aspect-w-1 aspect-h-1 overflow-hidden window h-fit",
                div { class: "title-bar",
                    div { class: "title-bar-text", "Mining Rig" }
                    div { class: "title-bar-controls",
                        button {
                            class: "close",
                            aria_label: "Close",
                            onclick: |_| {
                                info!("Closing window");
                            },
                            ""
                        }
                    }
                }
                div { class: "window-body",
                    menu { role: "tablist", class: "noselect multirows",

                        li {
                            id: "details-tab",
                            role: "tab",
                            aria_selected: if selected_tab() == "details" { "true" } else { "false" },
                            style: "padding:5px;padding-left:25px;padding-right:25px;max-width: fit-content;",
                            onclick: move |_| selected_tab.set("details".to_string()),
                            p { class: get_details_tab_class, "Details" }
                        }
                    }
                    for row in 0..((SELECTION().max_selectable + 4) / 5) {
                        menu { role: "tablist", class: "noselect multirows",
                            for i in (row * 5)..((row + 1) * 5).min(SELECTION().max_selectable) {
                                li {
                                    id: "mining-tab",
                                    role: "tab",
                                    aria_selected: if selected_tab() == format!("mining-{}", i) { "true" } else { "false" },
                                    style: "padding:5px;padding-left:10px;padding-right:10px;",
                                    onclick: move |_| selected_tab.set(format!("mining-{}", i)),
                                    p { class: "rig-tab", "Rig {i + 1}" }
                                }
                            }
                        }
                    }

                    for i in 0..SELECTION().max_selectable {
                        RigMiningTab { selected_tab, index: i as usize }
                    }
                    RigDetailsTab { selected_tab }
                }
            }
        }
    }
}

#[component]
pub fn Upgrades() -> Element {
    let mut selected_tab: Signal<String> = use_signal(|| "cpu".to_string());

    let get_cpu_tab_class = {
        if MARKET().bank.balance > MINING_RIG().get_cpu_upgrade_cost()
            && MINING_RIG().get_cpu_level() < 5
        {
            "rig-tab upgradeable"
        } else {
            "rig-tab"
        }
    };

    let get_gpu_tab_class = {
        if (MARKET().bank.balance >= MINING_RIG().get_gpu_upgrade_cost())
            && MINING_RIG().get_filled_gpu_slots() < MINING_RIG().get_max_gpu_slots()
        {
            "rig-tab upgradeable"
        } else {
            "rig-tab"
        }
    };

    let get_asic_tab_class = {
        if (MARKET().bank.balance >= MINING_RIG().get_asic_upgrade_cost())
            && MINING_RIG().get_filled_asic_slots() < MINING_RIG().get_max_asic_slots()
        {
            "rig-tab upgradeable"
        } else {
            "rig-tab"
        }
    };

    let get_rug_tab_class = {
        if MARKET().bank.balance > MINING_RIG().get_rug_protection_upgrade_cost()
            && MINING_RIG().get_rug_protection_level() < 65
        {
            "rig-tab upgradeable"
        } else {
            "rig-tab"
        }
    };

    let get_multimining_tab_class = {
        if MARKET().bank.balance > SELECTION().get_upgrade_cost() && SELECTION().max_selectable < 10
        {
            "rig-tab upgradeable"
        } else {
            "rig-tab"
        }
    };

    rsx! {
        div { class: "items-center justify-center container",
            div { class: "aspect-w-1 aspect-h-1 overflow-hidden window h-fit",
                div { class: "title-bar",
                    div { class: "title-bar-text", "Upgrades" }
                    div { class: "title-bar-controls",
                        button {
                            class: "close",
                            aria_label: "Close",
                            onclick: |_| {
                                info!("Closing window");
                            },
                            ""
                        }
                    }
                }
                div { class: "window-body",
                    menu { role: "tablist", class: "noselect",
                        if MINING_RIG().get_level() >= 2 {
                            li {
                                id: "cpu-tab",
                                role: "tab",
                                aria_selected: if selected_tab() == "cpu" { "true" } else { "false" },
                                style: "padding:5px;padding-left:10px;padding-right:10px;",
                                onclick: move |_| selected_tab.set("cpu".to_string()),
                                p { class: get_cpu_tab_class, "CPU" }
                            }
                        }

                        if MINING_RIG().get_level() >= 10 {
                            li {
                                id: "rug-tab",
                                role: "tab",
                                aria_selected: if selected_tab() == "rug" { "true" } else { "false" },
                                style: "padding:5px;padding-left:10px;padding-right:10px;",
                                onclick: move |_| selected_tab.set("rug".to_string()),
                                p { class: get_rug_tab_class, "DerpFi" }
                            }
                        }

                        if MINING_RIG().get_level() >= 5 {
                            li {
                                id: "gpu-tab",
                                role: "tab",
                                aria_selected: if selected_tab() == "gpu" { "true" } else { "false" },
                                style: "padding:5px;padding-left:10px;padding-right:10px;",
                                onclick: move |_| selected_tab.set("gpu".to_string()),
                                p { class: get_gpu_tab_class, "GPU" }
                            }
                        }

                        if MINING_RIG().get_level() >= 35 {
                            li {
                                id: "asic-tab",
                                role: "tab",
                                aria_selected: if selected_tab() == "asic" { "true" } else { "false" },
                                style: "padding:5px;padding-left:10px;padding-right:10px;",
                                onclick: move |_| selected_tab.set("asic".to_string()),
                                p { class: get_asic_tab_class, "ASIC" }
                            }
                        }

                        if !MINING_RIG().get_global_share_cooldown() {
                            li {
                                id: "multi-mining-tab",
                                role: "tab",
                                aria_selected: if selected_tab() == "multi-mining" { "true" } else { "false" },
                                style: "padding:5px;padding-left:10px;padding-right:10px;",
                                onclick: move |_| selected_tab.set("multi-mining".to_string()),
                                p { class: get_multimining_tab_class, "Multi-Mining" }
                            }
                        }
                    }

                    if MINING_RIG().get_level() >= 2 {
                        RigCPUTab { selected_tab }
                    }

                    if MINING_RIG().get_level() >= 5 {
                        RigGPUTab { selected_tab }
                    }

                    if MINING_RIG().get_level() >= 10 {
                        RigRugProtectionTab { selected_tab }
                    }

                    if MINING_RIG().get_level() >= 35 {
                        RigAsicTab { selected_tab }
                    }

                    if !MINING_RIG().get_global_share_cooldown() {
                        RigMultiMiningTab { selected_tab }
                    }
                }
            }
        }
    }
}

#[component]
pub fn RigMultiMiningTab(selected_tab: Signal<String>) -> Element {
    let get_style = {
        let selected_tab = selected_tab.clone();
        move || {
            if selected_tab() == "multi-mining" {
                "display: block;padding: 10px;"
            } else {
                "display: none;padding: 10px;"
            }
        }
    };

    let get_style_buttons = {
        let selected_tab = selected_tab.clone();
        move || {
            if selected_tab() == "multi-mining" {
                "display: flex;justify-content: space-between;margin-top: 10px;"
            } else {
                "display: none;justify-content: space-between;"
            }
        }
    };

    let upgrade_available = {
        if MARKET().bank.balance > SELECTION().get_upgrade_cost() && SELECTION().max_selectable < 10
        {
            false
        } else {
            true
        }
    };

    let get_upgrade_cost = || {
        if SELECTION().max_selectable >= 10 {
            "Max Level".to_string()
        } else {
            format_comma_seperator(SELECTION().get_upgrade_cost(), 0)
        }
    };

    rsx! {
        div { class: "window", style: get_style(), role: "tabpanel",
            div {
                class: "flex flex-row",
                style: "justify-content: space-between;",
                div {
                    h4 { "Multi Mining" }
                    p { "Split the hashrate of your rig between multiple coins." }
                    br {}
                    p { "Level: {SELECTION().max_selectable}" }
                }
                div {
                    h4 { "Mining Upgrade" }
                    br {}
                    p { "Upgrade Cost:" }
                    p { "${get_upgrade_cost()}" }
                }
            }
        }

        div { class: "flex flex", style: get_style_buttons(),
            button {
                class: "",
                disabled: upgrade_available,
                onclick: move |_| {
                    if MARKET.write().bank.withdraw(SELECTION().get_upgrade_cost()) {
                        SELECTION.write().increment_max_selectable();
                        DO_SAVE.write().save = true;
                    }
                },
                "Upgrade Multi-Mining"
            }
        }
    }
}

#[component]
pub fn RigRugProtectionTab(selected_tab: Signal<String>) -> Element {
    let get_style = {
        let selected_tab = selected_tab.clone();
        move || {
            if selected_tab() == "rug" {
                "display: block;padding: 10px;"
            } else {
                "display: none;padding: 10px;"
            }
        }
    };

    let get_style_buttons = {
        let selected_tab = selected_tab.clone();
        move || {
            if selected_tab() == "rug" {
                "display: flex;justify-content: space-between;margin-top: 10px;"
            } else {
                "display: none;justify-content: space-between;"
            }
        }
    };

    let rug_protection_cost = MINING_RIG().get_rug_protection_upgrade_cost();

    let rug_protection_can_upgrade = {
        let rug_protec_lvl = MINING_RIG().get_rug_protection_level();
        if MARKET().bank.balance >= rug_protection_cost && rug_protec_lvl < 65 {
            false
        } else {
            true
        }
    };

    let enable_or_upgrade = {
        if MINING_RIG().get_rug_protection_active() {
            "Upgrade"
        } else {
            "Enable"
        }
    };

    let rug_protection_active = MINING_RIG().get_rug_protection_active();

    let do_rug_protection_upgrade = move |_| {
        let cost = rug_protection_cost;

        if MARKET.write().bank.withdraw(cost) {
            MINING_RIG.write().upgrade_rug_protection();

            let rug_protec_lvl = MINING_RIG().get_rug_protection_level();

            let msg = if rug_protection_active {
                format!("Rug Protection enabled, new level {rug_protec_lvl}")
            } else {
                format!("Rug Protection upgraded, new level {rug_protec_lvl}")
            };
            spawn_local(async move {
                command_line_output(&msg).await;
            });
        }
        DO_SAVE.write().save = true;
    };

    rsx! {
        div { class: "window", style: get_style(), role: "tabpanel",
            div {
                class: "flex flex-row",
                style: "justify-content: space-between;",
                div {
                    h4 { "DerpFi Rug Protection" }
                    p {
                        "Rug Protection Level: {format_comma_seperator(MINING_RIG().get_rug_protection_level(), 2)}"
                    }
                    p { "Multi-Mining Activated: {!MINING_RIG().get_global_share_cooldown()}" }
                    p {
                        "Amount Rug Protected: {format_comma_seperator(MINING_RIG().get_rug_protection_amount() * 100.0, 2)}%"
                    }
                }
                div {
                    h4 { "Rug Protection Upgrade" }
                    br {}
                    p { "Upgrade Cost: ${format_comma_seperator(rug_protection_cost, 2)}" }
                }
            }
        }

        div { class: "flex flex-row", style: get_style_buttons(),
            button {
                class: "",
                disabled: rug_protection_can_upgrade,
                onclick: do_rug_protection_upgrade,

                "{enable_or_upgrade} Rug Protection"
            }
        }
    }
}

#[component]
pub fn RigAsicTab(selected_tab: Signal<String>) -> Element {
    let upgrade_asic = {
        move |_| {
            let cost = MINING_RIG().get_asic_upgrade_cost();

            if MARKET.write().bank.withdraw(cost) {
                MINING_RIG.write().upgrade_asic();

                let asic_lvl = MINING_RIG().get_asic_level();

                let msg = format!("ASIC upgrade successful, new level {asic_lvl}");
                spawn_local(async move {
                    command_line_output(&msg).await;
                });
            }
            DO_SAVE.write().save = true;
        }
    };

    let upgrade_max = move |_| loop {
        let cost = MINING_RIG().get_asic_upgrade_cost();

        let slots_available =
            MINING_RIG().get_filled_asic_slots() < MINING_RIG().get_max_asic_slots();

        if slots_available && MARKET.write().bank.withdraw(cost) {
            MINING_RIG.write().upgrade_asic();

            let asic_lvl = MINING_RIG().get_asic_level();

            let msg = format!("ASIC upgrade successful, new level {asic_lvl}");
            spawn_local(async move {
                command_line_output(&msg).await;
            });
        } else {
            DO_SAVE.write().save = true;
            break;
        }
    };

    let upgrade_available = {
        if (MARKET().bank.balance < MINING_RIG().get_asic_upgrade_cost())
            || MINING_RIG().get_filled_asic_slots() >= MINING_RIG().get_max_asic_slots()
        {
            true
        } else {
            false
        }
    };

    let get_style = {
        let selected_tab = selected_tab.clone();
        move || {
            if selected_tab() == "asic" {
                "display: block;padding: 10px;"
            } else {
                "display: none;padding: 10px;"
            }
        }
    };

    let get_style_buttons = {
        let selected_tab = selected_tab.clone();
        move || {
            if selected_tab() == "asic" {
                "display: flex;justify-content: space-between;margin-top: 10px;"
            } else {
                "display: none;justify-content: space-between;"
            }
        }
    };

    rsx! {
        div { class: "window", style: get_style(), role: "tabpanel",
            div {
                class: "flex flex-row",
                style: "justify-content: space-between;",
                div {
                    h4 { "ASIC Details" }
                    p {
                        "ASICs: {format_comma_seperator(MINING_RIG().get_filled_asic_slots(), 0)} / {format_comma_seperator(MINING_RIG().get_max_asic_slots(), 0)}"
                    }
                    p { "Hash Rate: {format_comma_seperator(MINING_RIG().get_asic_hash_rate(), 0)}" }
                    p { "Power: {format_comma_seperator(MINING_RIG().get_asic_power_usage(), 0)}" }
                }
                div {
                    h4 { "ASIC Upgrade" }
                    br {}
                    p {
                        "Upgrade Cost: ${format_comma_seperator(MINING_RIG().get_asic_upgrade_cost(), 2)}"
                    }
                }
            }
        }

        div { class: "flex flex-row", style: get_style_buttons(),
            button {
                class: "",
                disabled: upgrade_available,
                onclick: upgrade_asic,
                "Upgrade ASIC"
            }
            button { disabled: upgrade_available, onclick: upgrade_max, "Upgrade Max" }
        }
    }
}

#[component]
pub fn RigGPUTab(selected_tab: Signal<String>) -> Element {
    let upgrade_gpu = {
        move |_| {
            let cost = MINING_RIG().get_gpu_upgrade_cost();

            if MARKET.write().bank.withdraw(cost) {
                MINING_RIG.write().upgrade_gpu();

                let gpu_lvl = MINING_RIG().get_gpu_level();

                let msg = format!("GPU upgrade successful, new level {gpu_lvl}");
                spawn_local(async move {
                    command_line_output(&msg).await;
                });
            }
            DO_SAVE.write().save = true;
        }
    };

    let upgrade_max = move |_| loop {
        let cost = MINING_RIG().get_gpu_upgrade_cost();

        let slots_available =
            MINING_RIG().get_filled_gpu_slots() < MINING_RIG().get_max_gpu_slots();

        if slots_available && MARKET.write().bank.withdraw(cost) {
            MINING_RIG.write().upgrade_gpu();

            let gpu_lvl = MINING_RIG().get_gpu_level();

            let msg = format!("GPU upgrade successful, new level {gpu_lvl}");
            spawn_local(async move {
                command_line_output(&msg).await;
            });
        } else {
            DO_SAVE.write().save = true;
            break;
        }
    };

    let upgrade_available = {
        if (MARKET().bank.balance < MINING_RIG().get_gpu_upgrade_cost())
            || MINING_RIG().get_filled_gpu_slots() >= MINING_RIG().get_max_gpu_slots()
        {
            true
        } else {
            false
        }
    };

    let get_style = {
        let selected_tab = selected_tab.clone();
        move || {
            if selected_tab() == "gpu" {
                "display: block;padding: 10px;"
            } else {
                "display: none;padding: 10px;"
            }
        }
    };

    let get_style_buttons = {
        let selected_tab = selected_tab.clone();
        move || {
            if selected_tab() == "gpu" {
                "display: flex;justify-content: space-between;margin-top: 10px;"
            } else {
                "display: none;justify-content: space-between;"
            }
        }
    };

    rsx! {
        div { class: "window", style: get_style(), role: "tabpanel",
            div {
                class: "flex flex-row",
                style: "justify-content: space-between;",
                div {
                    h4 { "GPU Details" }
                    p {
                        "GPUs: {format_comma_seperator(MINING_RIG().get_filled_gpu_slots(), 0)} / {format_comma_seperator(MINING_RIG().get_max_gpu_slots(), 0)}"
                    }
                    p { "Hash Rate: {format_comma_seperator(MINING_RIG().get_gpu_hash_rate(), 0)}" }
                    p { "Power: {format_comma_seperator(MINING_RIG().get_gpu_power_usage(), 0)}" }
                }
                div {
                    h4 { "GPU Upgrade" }
                    br {}
                    p {
                        "Upgrade Cost: ${format_comma_seperator(MINING_RIG().get_gpu_upgrade_cost(), 2)}"
                    }
                }
            }
        }

        div { class: "flex flex-row", style: get_style_buttons(),
            button {
                class: "",
                disabled: upgrade_available,
                onclick: upgrade_gpu,
                "Upgrade GPU"
            }
            button {
                class: "",
                disabled: upgrade_available,
                onclick: upgrade_max,
                "Upgrade Max"
            }
        }
    }
}

#[component]
pub fn RigCPUTab(selected_tab: Signal<String>) -> Element {
    let get_style = {
        let selected_tab = selected_tab.clone();
        move || {
            if selected_tab() == "cpu" {
                "display: block;padding: 10px;"
            } else {
                "display: none;padding: 10px;"
            }
        }
    };

    let get_style_buttons = {
        let selected_tab = selected_tab.clone();
        move || {
            if selected_tab() == "cpu" {
                "display: flex;justify-content: space-between;margin-top: 10px;"
            } else {
                "display: none;justify-content: space-between;"
            }
        }
    };

    let upgrade_available = {
        if (MARKET().bank.balance < MINING_RIG().get_cpu_upgrade_cost())
            || MINING_RIG().get_cpu_level() >= 5
        {
            true
        } else {
            false
        }
    };

    rsx! {
        div { class: "window", style: get_style(), role: "tabpanel",
            div {
                class: "flex flex-row",
                style: "justify-content: space-between;",
                div {
                    h4 { "CPU Details" }
                    p { "Level: {MINING_RIG().get_cpu_level()} / 5" }
                    p { "Hash Rate: {MINING_RIG().get_cpu_hash_rate()}" }
                    p { "Power: {MINING_RIG().get_cpu_power_usage()}" }
                }
                div {
                    h4 { "CPU Upgrade" }
                    br {}

                    if MINING_RIG().get_cpu_level() < 5 {
                        p {
                            "Upgrade Cost: ${format_comma_seperator(MINING_RIG().get_cpu_upgrade_cost(), 2)}"
                        }
                    } else {
                        p { "Max Level" }
                    }
                }
            }
        }

        div { class: "flex flex-row", style: get_style_buttons(),
            button {
                class: "",
                disabled: upgrade_available,
                onclick: |_| {
                    let cost = MINING_RIG().get_cpu_upgrade_cost();
                    if MARKET.write().bank.withdraw(cost) {
                        MINING_RIG.write().upgrade_cpu();
                        let cpu_lvl = MINING_RIG().get_cpu_level();
                        let msg = format!("CPU upgrade successful, new level {cpu_lvl}");
                        spawn_local(async move {
                            command_line_output(&msg).await;
                        });
                    }
                    DO_SAVE.write().save = true;
                },
                "Upgrade CPU"
            }
        }
    }
}

#[component]
pub fn RigDetailsTab(selected_tab: Signal<String>) -> Element {
    let upgrade_auto_power_fill = {
        move |_| {
            let cost = MINING_RIG().get_auto_power_fill_upgrade_cost();

            if MARKET.write().bank.withdraw(cost) {
                MINING_RIG.write().upgrade_auto_power_fill();

                let auto_fill_level = MINING_RIG().get_auto_power_fill_level();
                let msg =
                    format!("Auto-power fill upgrade successful, new level {auto_fill_level}");
                spawn_local(async move {
                    command_line_output(&msg).await;
                });
            }

            DO_SAVE.write().save = true;
        }
    };

    let get_style = {
        let selected_tab = selected_tab.clone();
        move || {
            if selected_tab() == "details" {
                "display: block;padding: 10px;"
            } else {
                "display: none;padding: 10px;"
            }
        }
    };
    let get_style_buttons = {
        let selected_tab = selected_tab.clone();
        move || {
            if selected_tab() == "details" {
                "display: flex;justify-content: space-between;margin-top: 10px;"
            } else {
                "display: none;justify-content: space-between;"
            }
        }
    };

    let enable_or_upgrade = {
        if MINING_RIG().get_auto_power_fill_level() > 0 {
            "Upgrade"
        } else {
            "Enable"
        }
    };

    let fill_delay = {
        let delay = MINING_RIG().get_auto_power_fill_delay() as f64 / 20.0;
        format!("{delay}s")
    };

    let auto_fill_level = MINING_RIG().get_auto_power_fill_level();

    let can_upgrade_auto_fill = {
        if (MARKET().bank.balance < MINING_RIG().get_auto_power_fill_upgrade_cost())
            || auto_fill_level >= 13
        {
            true
        } else {
            false
        }
    };

    rsx! {
        div { class: "window", style: get_style(), role: "tabpanel",
            div {
                class: "flex flex-row",
                style: "justify-content: space-between;",
                div {
                    h4 { "Mining Rig Details" }
                    p { "Level: {MINING_RIG().get_level()}" }
                    p {
                        "Power Capacity: {format_comma_seperator(MINING_RIG().get_power_capacity(), 2)}"
                    }
                    p {
                        "GPU Slots: {MINING_RIG().get_filled_gpu_slots()} / {MINING_RIG().get_max_gpu_slots()}"
                    }
                    p {
                        "ASIC Slots: {MINING_RIG().get_filled_asic_slots()} / {MINING_RIG().get_max_asic_slots()}"
                    }
                    br {}
                    p { "Current Hash Rate: {format_comma_seperator(MINING_RIG().get_hash_rate(), 2)}" }
                    p { "Power Usage: {format_comma_seperator(MINING_RIG().get_power_usage(), 2)}" }
                    br {}
                    p {
                        "Rig Upgrade Cost: ${format_comma_seperator(MINING_RIG().get_rig_upgrade_cost(), 2)}"
                    }
                }
                if auto_fill_level > 0 {
                    div { style: "text-align: end;",
                        h4 { "Auto Power Fill" }
                        p { "Level: {MINING_RIG().get_auto_power_fill_level()}" }
                        p { "Fill Amount: {MINING_RIG().get_auto_power_fill_amount() * 100.0:.0}%" }
                        p { "Fill Delay: {fill_delay}" }
                        p {
                            "Fill Cost: ${format_comma_seperator(MINING_RIG().get_auto_power_fill_cost(GAME_TIME().day), 2)}"
                        }
                        br {}

                        if MINING_RIG().get_auto_power_fill_level() < 13 {
                            p {
                                "Upgrade Cost: ${format_comma_seperator(MINING_RIG().get_auto_power_fill_upgrade_cost(), 2)}"
                            }
                        } else {
                            p { "Max Level" }
                        }
                    }
                } else {
                    div { style: "text-align: end;",
                        h4 { "Auto Power Fill" }
                        br {}
                        p {
                            "Enable Cost: ${format_comma_seperator(MINING_RIG().get_auto_power_fill_upgrade_cost(), 2)}"
                        }
                    }
                }
            }
        }

        div { class: "flex flex-row", style: get_style_buttons(),
            button {
                class: "",
                disabled: can_upgrade_rig(),
                onclick: |_| {
                    let cost = MINING_RIG().get_rig_upgrade_cost();
                    if MARKET.write().bank.withdraw(cost) {
                        MINING_RIG.write().upgrade();
                        let rig_lvl = MINING_RIG().get_level();
                        let msg = format!("Rig upgrade successful, new level {rig_lvl}");
                        spawn_local(async move {
                            command_line_output(&msg).await;
                        });
                    }
                    DO_SAVE.write().save = true;
                },
                "Upgrade Rig"
            }
            button {
                class: "",
                disabled: can_upgrade_auto_fill,
                onclick: upgrade_auto_power_fill,
                "{enable_or_upgrade} Auto-power fill"
            }
        }
    }
}

#[component]
pub fn RigMiningTab(selected_tab: Signal<String>, index: usize) -> Element {
    let toggle_auto_power_fill = {
        move |_| {
            MINING_RIG.write().toggle_auto_power_fill();
            DO_SAVE.write().save = true;
        }
    };

    let selected_coin_name = {
        let sel = SELECTION().clone();

        let selected_coin = sel.selection_by_index(index);
        match selected_coin {
            Some(selected) => selected.name.to_string(),
            None => "Not Mining".to_string(),
        }
    };

    let class_from_name = move |name: String| {
        if name == "Not Mining" {
            return "".to_string();
        }
        let mkt = MARKET.read();
        let coin = mkt.coin_by_name(&name);
        match coin {
            Some(coin) => {
                format!("selected-name-{}", coin.index)
            }
            None => "".to_string(),
        }
    };

    let get_style = {
        let selected_tab = selected_tab.clone();
        move || {
            if selected_tab() == format!("mining-{}", index) {
                "display: block;padding: 10px;"
            } else {
                "display: none;padding: 10px;"
            }
        }
    };

    let get_style_buttons = {
        let selected_tab = selected_tab.clone();
        move || {
            if selected_tab() == format!("mining-{}", index) {
                "display: flex;margin-top: 25px;justify-content: space-between;"
            } else {
                "display: none;"
            }
        }
    };

    let get_style_status_bar = {
        let selected_tab = selected_tab.clone();
        move || {
            if selected_tab() == format!("mining-{}", index) {
                "display: flex;"
            } else {
                "display: none;"
            }
        }
    };

    let can_do_fill_power = {
        let power_cost = MINING_RIG().get_power_fill_cost(GAME_TIME().day);
        let fill_amount = MINING_RIG().get_power_fill();
        if MARKET().bank.balance >= power_cost && fill_amount < 1.0 {
            true
        } else {
            false
        }
    };

    rsx! {
        div { class: "window", style: get_style(), role: "tabpanel",
            p {
                style: "font-size: medium;float:right;",
                class: "{class_from_name(selected_coin_name)} selected-name",
                "{selected_coin_name}"
            }
            h4 { "Share Progress" }
            ProgressBar { progress_id: format!("share-progress-{}", index), progress_message: "".to_string() }
            h4 { "Block Progress" }
            ProgressBar { progress_id: format!("block-progress-{}", index), progress_message: "".to_string() }
            h4 { "Power Level" }
            ProgressBar {
                progress_id: &format!("power_available-progress-{}", index),
                progress_message: if MINING_RIG().get_auto_power_refill_time() != Some(0)
                    && MINING_RIG().get_auto_power_fill_active()
                {
                    let refill_time = MINING_RIG().get_auto_power_refill_time();
                    if MINING_RIG().get_power_fill() <= 0.2 && refill_time.is_some() {
                        match refill_time {
                            Some(refill_time) => {
                                let refill_time = refill_time as f64 / 20.0;
                                format!("Power refill in {:.1}s", refill_time)
                            }
                            None => "".to_string(),
                        }
                    } else {
                        "".to_string()
                    }
                } else {
                    "".to_string()
                }
            }
        }

        div { class: "flex flex-row", style: get_style_buttons(),
            button {
                class: "",
                onclick: |_| async {
                    MINING_RIG.write().add_click_power();
                    let power_available = MINING_RIG().get_power_fill();
                    for i in 0..SELECTION().max_selectable {
                        update_progess_bar(
                                &format!("power_available-progress-{}", i),
                                power_available * 100.0,
                            )
                            .await;
                    }
                },
                "Click Power"
            }

            div { class: "flex flex-col",
                if MINING_RIG().get_auto_power_fill_level() > 0 {
                    div { class: "mt-[10px]", style: get_style_status_bar(),
                        input {
                            id: "auto-power-fill",
                            class: "",
                            style: "",
                            r#type: "checkbox",
                            checked: MINING_RIG().get_auto_power_fill_active(),
                            onchange: toggle_auto_power_fill
                        }
                        label { class: "", r#for: "auto-power-fill", "Enable Auto-power fill" }
                    }
                }
                button {
                    class: "",
                    style: "margin-top: 10px;",
                    disabled: !can_do_fill_power,
                    onclick: move |_| async move {
                        do_fill_power().await;
                    },
                    "Fill Power"
                }
            }
        }
        div { class: "gap-px mt-[30px]", style: get_style_status_bar(),

            if MINING_RIG().get_auto_power_fill_level() > 0 {
                div { class: "w-full",
                    p {
                        class: "status-bar-field font-mono",
                        style: "padding:4px;",
                        "Auto Power Cost"
                    }
                    p {
                        class: "status-bar-field font-mono",
                        style: "padding:4px;text-align: center;",
                        "${format_comma_seperator(MINING_RIG().get_auto_power_fill_cost(GAME_TIME().day), 2)}"
                    }
                }

                div { class: "w-full",
                    p {
                        class: "status-bar-field font-mono",
                        style: "padding:4px;",
                        "Auto Fill Amount"
                    }
                    p {
                        class: "status-bar-field font-mono",
                        style: "padding:4px;text-align: center;",
                        "{MINING_RIG().get_auto_power_fill_amount() * 100.0:.0}%"
                    }
                }
            }

            div { class: "w-full",
                p {
                    class: "status-bar-field font-mono",
                    style: "padding:4px;",
                    "Power Cost"
                }
                p {
                    class: "status-bar-field font-mono",
                    style: "padding:4px;text-align: center;",
                    "${format_comma_seperator(MINING_RIG().get_power_fill_cost(GAME_TIME().day), 2)}"
                }
            }
        }
    }
}

#[component]
pub fn Paint(confirm_modal: Signal<ConfirmModal>) -> Element {
    let mut is_drawing = use_signal(|| false);
    let mut last_position = use_signal(|| Position {
        x: 0.0,
        y: 0.0,
        color: "black".to_string(),
        bg_color: "white".to_string(),
        line_width: 3.0,
    });

    let mut drawing_color = use_signal(|| "#000".to_string());
    let mut bg_color = use_signal(|| "#ffffff".to_string());

    let mut paint_undo = use_signal(|| PaintUndo::new());
    let mut line_width = use_signal(|| 3.0);

    // Utility function to get position from MouseEvent
    let get_mouse_position = move |e: &MouseEvent| -> Position {
        let document = window().document().unwrap();
        let canvas = document
            .get_element_by_id("paint-canvas")
            .unwrap()
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .unwrap();
        let rect = canvas.get_bounding_client_rect();

        Position {
            x: e.data.client_coordinates().x as f64 - rect.left(),
            y: e.data.client_coordinates().y as f64 - rect.top(),
            color: drawing_color(),
            bg_color: bg_color(),
            line_width: line_width(),
        }
    };

    // Utility function to get position from TouchEvent
    let get_touch_position = move |e: &TouchEvent| -> Position {
        let document = window().document().unwrap();
        let canvas = document
            .get_element_by_id("paint-canvas")
            .unwrap()
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .unwrap();

        let rect = canvas.get_bounding_client_rect();
        let touch = &e.touches()[0];

        Position {
            x: touch.client_coordinates().x as f64 - rect.left(),
            y: touch.client_coordinates().y as f64 - rect.top(),
            color: drawing_color(),
            bg_color: bg_color(),
            line_width: line_width(),
        }
    };

    // Mouse down handler
    let on_mouse_down = move |e: MouseEvent| {
        is_drawing.set(true);
        let position = get_mouse_position(&e);
        last_position.set(position.clone());

        paint_undo.write().add_position(position.clone());

        let document = window().document().unwrap();
        let canvas = document
            .get_element_by_id("paint-canvas")
            .unwrap()
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .unwrap();
        let context = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .unwrap();

        context.begin_path();
        context.move_to(position.x, position.y);
    };

    // Touch start handler
    let on_touch_start = move |e: TouchEvent| {
        is_drawing.set(true);
        let position = get_touch_position(&e);
        last_position.set(position.clone());

        paint_undo.write().add_position(position.clone());

        let document = window().document().unwrap();
        let canvas = document
            .get_element_by_id("paint-canvas")
            .unwrap()
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .unwrap();
        let context = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .unwrap();

        context.begin_path();
        context.move_to(position.x, position.y);
    };

    // Mouse up handler
    let on_mouse_up = move |_| {
        is_drawing.set(false);
        paint_undo.write().add_path();
    };

    // Touch end handler
    let on_touch_end = move |_| {
        is_drawing.set(false);
        paint_undo.write().add_path();
    };

    let on_mouse_enter = move |e: MouseEvent| {
        e.held_buttons().iter().for_each(|button| {
            if button == MouseButton::Primary {
                is_drawing.set(true);
                let position = get_mouse_position(&e);
                last_position.set(position.clone());

                paint_undo.write().add_position(position.clone());
            }
        });
        if is_drawing() {
            let document = window().document().unwrap();
            let canvas = document
                .get_element_by_id("paint-canvas")
                .unwrap()
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .unwrap();
            let context = canvas
                .get_context("2d")
                .unwrap()
                .unwrap()
                .dyn_into::<web_sys::CanvasRenderingContext2d>()
                .unwrap();

            context.begin_path();
            context.move_to(last_position().x, last_position().y);
        }
    };

    // Mouse move handler
    let on_mouse_move = move |e: MouseEvent| {
        if is_drawing() {
            let position = get_mouse_position(&e);

            let document = window().document().unwrap();
            let canvas = document
                .get_element_by_id("paint-canvas")
                .unwrap()
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .unwrap();
            let context = canvas
                .get_context("2d")
                .unwrap()
                .unwrap()
                .dyn_into::<web_sys::CanvasRenderingContext2d>()
                .unwrap();

            context.set_stroke_style(&JsValue::from_str(&drawing_color()));
            context.set_line_width(line_width());
            context.line_to(position.x, position.y);
            context.stroke();

            last_position.set(position.clone());

            paint_undo.write().add_position(position.clone());

            context.begin_path();
            context.move_to(position.x, position.y);
        }
    };

    // Touch move handler
    let on_touch_move = move |e: TouchEvent| {
        if is_drawing() {
            let position = get_touch_position(&e);

            let document = window().document().unwrap();
            let canvas = document
                .get_element_by_id("paint-canvas")
                .unwrap()
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .unwrap();
            let context = canvas
                .get_context("2d")
                .unwrap()
                .unwrap()
                .dyn_into::<web_sys::CanvasRenderingContext2d>()
                .unwrap();

            context.set_stroke_style(&JsValue::from_str(&drawing_color()));
            context.set_line_width(line_width());
            context.line_to(position.x, position.y);
            context.stroke();

            last_position.set(position.clone());

            paint_undo.write().add_position(position.clone());

            context.begin_path();
            context.move_to(position.x, position.y);
        }
    };

    use_effect(move || {
        let win = window();

        let document = win.document().unwrap();
        let paint_window = document.get_element_by_id("paint-window").unwrap();
        let paint_canvas = document.get_element_by_id("paint-canvas").unwrap();

        let paint_window_width = paint_window.get_bounding_client_rect().width();

        let paint_canvas = paint_canvas
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .unwrap();

        let buffer = match paint_window_width {
            num if num >= 377.0 => 0,
            _ => 26,
        };

        paint_canvas.set_width(paint_window_width as u32 - buffer);

        let context = paint_canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .unwrap();

        context.set_fill_style(&JsValue::from_str("white"));
        context.fill_rect(
            0.0,
            0.0,
            paint_canvas.width() as f64,
            paint_canvas.height() as f64,
        );

        let resize_listener = Closure::wrap(Box::new(move |_: web_sys::Event| {
            info!("Resize event");
            let win = window();
            let document = win.document().unwrap();
            let paint_window = document.get_element_by_id("paint-window").unwrap();
            let paint_canvas = document.get_element_by_id("paint-canvas").unwrap();

            let paint_window_width = paint_window.get_bounding_client_rect().width();

            let paint_canvas = paint_canvas
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .unwrap();
            paint_canvas.set_width(paint_window_width as u32 - 26);

            spawn_local(async move {
                set_canvas_background_from_local().await;
            });
        }) as Box<dyn FnMut(_)>);

        let res = win
            .add_event_listener_with_callback("resize", resize_listener.as_ref().unchecked_ref());

        resize_listener.forget();

        match res {
            Ok(_) => {
                info!("Resize listener added");
            }
            Err(e) => info!("Error adding resize listener: {:?}", e),
        }
    });

    use_future(move || {
        let mut paint_undo = paint_undo.clone();

        let mut bg_color = bg_color.clone();
        let mut drawing_color = drawing_color.clone();
        let mut line_width = line_width.clone();

        async move {
            let saved_paint_res = get_paint_undo().await;

            let saved_paint = match saved_paint_res {
                Ok(saved_paint) => match saved_paint {
                    Some(saved_paint) => saved_paint,
                    None => PaintUndo::new(),
                },
                Err(_) => PaintUndo::new(),
            };

            *paint_undo.write() = saved_paint;

            set_canvas_background_last(
                paint_undo,
                &mut bg_color,
                &mut drawing_color,
                &mut line_width,
            );

            let mut last_undo = paint_undo().clone();

            loop {
                let paint_undo = paint_undo().clone();

                if paint_undo != last_undo {
                    info!("Saving paint undo");

                    set_paint_undo(&paint_undo).await;
                    last_undo = paint_undo;
                }

                TimeoutFuture::new(1000).await;
            }
        }
    });

    let mut open_file_menu = use_signal(|| false);
    let mut open_edit_menu = use_signal(|| false);

    let show_paint_save_modal = use_signal(|| false);
    let show_nft_mint_modal = use_signal(|| false);

    let line_width_options: Vec<f64> = vec![
        0.5, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 20.0, 30.0,
    ];

    let _max_hype_available = {
        let last_nft = NFT_STUDIO().last_release;
        let day = GAME_TIME().day;

        let last_days_ago = day - last_nft;

        last_days_ago > 0
    };

    rsx! {
        div { class: "relative items-center justify-center container",
            div { id: "paint-window", class: "overflow-hidden window h-fit",
                //style: "height: 350px;",

                div { class: "title-bar",
                    div { class: "title-bar-text", "NFT Studio 2000" }
                    div { class: "title-bar-controls",
                        button {
                            class: "close",
                            aria_label: "Close",
                            onclick: move |_| async move {
                                clear_canvas(&mut paint_undo, &mut bg_color, &mut drawing_color, &mut line_width)
                                    .await;
                            },
                            ""
                        }
                    }
                }
                div { class: "window-body h-full",
                    div {
                        class: "flex flex-row",
                        style: "font-size: small;padding: 2px;margin-left:15px;margin-bottom: 5px;",
                        div { class: "status-bar",
                            p {
                                class: if open_file_menu() {
                                    "status-bar-field menu-dropdown noselect"
                                } else {
                                    "menu-dropdown noselect"
                                },
                                style: "margin-right: 10px;padding-left: 10px;padding-right: 10px;",
                                onclick: move |_| {
                                    open_file_menu.set(!open_file_menu());
                                    open_edit_menu.set(false);
                                },
                                u { "F" }
                                "ile"
                            }
                            if open_file_menu() {
                                PaintFileMenuDropdown {
                                    open_file_menu: open_file_menu.clone(),
                                    paint_undo: paint_undo.clone(),
                                    show_paint_save_modal: show_paint_save_modal.clone(),
                                    bg_color: bg_color.clone(),
                                    drawing_color: drawing_color.clone(),
                                    line_width: line_width.clone(),
                                    show_nft_mint_modal: show_nft_mint_modal.clone()
                                }
                            }
                            p {
                                style: "margin-right: 10px;padding-left: 10px;padding-right: 10px;",
                                class: if open_edit_menu() {
                                    "status-bar-field menu-dropdown noselect"
                                } else {
                                    "menu-dropdown noselect"
                                },
                                onclick: move |_| {
                                    open_edit_menu.set(!open_edit_menu());
                                    open_file_menu.set(false);
                                },
                                u { "E" }
                                "dit"
                            }
                            if open_edit_menu() {
                                PaintEditMenuDropdown {
                                    open_edit_menu: open_edit_menu.clone(),
                                    paint_undo: paint_undo.clone(),
                                    bg_color: bg_color.clone(),
                                    drawing_color: drawing_color.clone(),
                                    line_width: line_width.clone()
                                }
                            }

                            p { style: "margin-right: 10px;padding-left: 10px;padding-right: 10px;",
                                "Score: {format_comma_seperator(NFT_STUDIO().mint_nft_dry_run(String::new(),paint_undo().calculate_score(), GAME_TIME().day).score, 2)}"
                            }
                        }
                    }

                    div {
                        class: "window flex flex-col",
                        style: "padding: 10px;",
                        div {
                            class: "flex flex-row",
                            style: "justify-content: space-between;",
                            h4 { "Hype" }
                            h4 { "Studio Rep: {format_comma_seperator(NFT_STUDIO().rep, 0)}" }
                        }
                        ProgressBar { progress_id: "paint-progress".to_string(), progress_message: "".to_string() }

                        div { style: "text-align: center;margin-top: 10px;",
                            p { "{NFT_STUDIO().hype:.2} / {NFT_STUDIO().next_rep():.0}" }
                        }

                        h4 { "Popularity" }
                        ProgressBar { progress_id: "popularity-progress".to_string(), progress_message: "".to_string() }

                        div {
                            class: "flex flex-row",
                            style: "justify-content: space-between;margin-top: 10px;",
                            p { style: "font-size: small;margin-top: 10px;",
                                "Income: ${format_comma_seperator(NFT_STUDIO().money_per_second_adjusted(), 2)} / s"
                            }
                            p { style: "font-size: small;margin-top: 10px;",
                                "NFTs Minted: {format_comma_seperator(NFT_STUDIO().nft_drawn, 0)}"
                            }
                        }
                    }

                    div { class: "sunken-panel", style: "margin-top: 10px;",
                        canvas {
                            id: "paint-canvas",
                            class: "paint-canvas",
                            style: "width: 100%;max-width: 377px;",
                            height: "275",
                            width: "377",
                            onmousedown: on_mouse_down,
                            onmouseup: on_mouse_up,
                            onmousemove: on_mouse_move,
                            onmouseleave: on_mouse_up,
                            onmouseenter: on_mouse_enter,
                            ontouchstart: on_touch_start,
                            ontouchend: on_touch_end,
                            ontouchmove: on_touch_move,
                            prevent_default: "ontouchmove"
                        }
                    }

                    div {
                        class: "sunken-panel flex flex-row",
                        style: "background-color: unset;justify-content: space-between;padding: 10px;",
                        div {
                            class: "flex flex-col",
                            style: "text-align: center;",
                            p { style: "", "Color" }
                            input {
                                r#type: "color",
                                style: "",
                                value: drawing_color(),
                                oninput: move |e| {
                                    let color = e.data.value();
                                    drawing_color.set(color.clone());
                                }
                            }
                        }

                        div { class: "flex flex-col",
                            p { style: "text-align: center;", "Line Width" }
                            select {
                                class: "select",
                                style: "",
                                onchange: move |e| {
                                    let lw = e.data.value().parse::<f64>().unwrap();
                                    line_width.set(lw);
                                },
                                for lw in line_width_options {
                                    option {
                                        value: lw.to_string(),
                                        selected: lw == line_width(),
                                        "{lw.to_string()}"
                                    }
                                }
                            }
                        }

                        div {
                            class: "flex flex-col",
                            style: "text-align: center;",
                            p { style: "", "BG Color" }
                            input {
                                r#type: "color",
                                value: bg_color(),
                                style: "",
                                oninput: move |e| {
                                    let color = e.data.value();
                                    set_canvas_background(&color.clone(), paint_undo.clone());
                                    bg_color.set(color.clone());
                                    let position = Position {
                                        x: 0.0,
                                        y: 0.0,
                                        color: drawing_color(),
                                        bg_color: color.clone(),
                                        line_width: line_width(),
                                    };
                                    last_position.set(position.clone());
                                    paint_undo.write().add_position(position.clone());
                                    paint_undo.write().add_path();
                                }
                            }
                        }
                    }
                }
            }

            if show_paint_save_modal() {
                PaintSaveModal { show_paint_save_modal: show_paint_save_modal.clone() }
            }

            if show_nft_mint_modal() {
                NftMintModal {
                    show_nft_mint_modal: show_nft_mint_modal.clone(),
                    paint_undo: paint_undo.clone(),
                    bg_color: bg_color.clone(),
                    drawing_color: drawing_color.clone(),
                    line_width: line_width.clone(),
                    confirm_modal: confirm_modal.clone()
                }
            }
        }
    }
}

#[component]
pub fn PaintEditMenuDropdown(
    open_edit_menu: Signal<bool>,
    paint_undo: Signal<PaintUndo>,
    bg_color: Signal<String>,
    drawing_color: Signal<String>,
    line_width: Signal<f64>,
) -> Element {
    let undo_enabled = move || {
        let paint_undo = paint_undo.clone();
        paint_undo().can_undo()
    };

    let redo_enabled = move || {
        let paint_undo = paint_undo.clone();
        paint_undo().can_redo()
    };

    let redo_move = {
        move |_| {
            if !redo_enabled() {
                return;
            }

            paint_undo.write().redo();

            set_canvas_background_last(
                paint_undo,
                &mut bg_color,
                &mut drawing_color,
                &mut line_width,
            );
        }
    };

    let undo_move = {
        move |_| {
            if !undo_enabled() {
                return;
            }

            paint_undo.write().undo();

            set_canvas_background_last(
                paint_undo,
                &mut bg_color,
                &mut drawing_color,
                &mut line_width,
            );
        }
    };

    rsx! {
        div {
            class: "dropdown",
            style: "top: 25px;right:50px;",
            onmouseleave: move |_| {
                open_edit_menu.set(false);
            },
            div { class: "dropdown-menu window",
                p {
                    class: if !undo_enabled() {
                        "dropdown-item disabled noselect"
                    } else {
                        "dropdown-item noselect"
                    },
                    onclick: undo_move,
                    u { "U" }
                    "ndo"
                }
                p {
                    class: if !redo_enabled() {
                        "dropdown-item disabled noselect"
                    } else {
                        "dropdown-item noselect"
                    },
                    onclick: redo_move,
                    u { "R" }
                    "edo"
                }
                p {
                    class: "dropdown-item noselect",
                    onclick: move |_| async move {
                        clear_canvas(&mut paint_undo, &mut bg_color, &mut drawing_color, &mut line_width)
                            .await;
                        open_edit_menu.set(false);
                    },
                    u { "C" }
                    "lear"
                }
            }
        }
    }
}

#[component]
pub fn PaintFileMenuDropdown(
    open_file_menu: Signal<bool>,
    paint_undo: Signal<PaintUndo>,
    show_paint_save_modal: Signal<bool>,
    bg_color: Signal<String>,
    drawing_color: Signal<String>,
    line_width: Signal<f64>,
    show_nft_mint_modal: Signal<bool>,
) -> Element {
    let print_to_nft = move || async move {
        let nft = NFT_STUDIO().mint_nft_dry_run(
            "test".to_string(),
            paint_undo().calculate_score(),
            GAME_TIME().day,
        );

        if nft.price < 0.01 {
            return;
        }

        show_nft_mint_modal.set(true);
    };

    rsx! {
        div {
            class: "dropdown",
            style: "top: 25px;right:50px;",
            onmouseleave: move |_| {
                open_file_menu.set(false);
            },
            div { class: "dropdown-menu window",
                p {
                    class: "dropdown-item noselect",
                    onclick: move |_| async move {
                        clear_canvas(&mut paint_undo, &mut bg_color, &mut drawing_color, &mut line_width)
                            .await;
                        open_file_menu.set(false);
                    },
                    u { "N" }
                    "ew"
                }
                p {
                    class: "dropdown-item noselect",
                    onclick: move |_| {
                        show_paint_save_modal.set(true);
                    },
                    u { "S" }

                    "ave"
                }
                p {
                    class: "dropdown-item noselect",
                    onclick: move |_| async move {
                        print_to_nft().await;
                        open_file_menu.set(false);
                    },
                    u { "M" }
                    "int NFT"
                }
                p {
                    class: "dropdown-item noselect",
                    onclick: move |_| {
                        open_file_menu.set(false);
                    },
                    u { "C" }
                    "lose"
                }
            }
        }
    }
}

#[component]
pub fn NftMintModal(
    show_nft_mint_modal: Signal<bool>,
    paint_undo: Signal<PaintUndo>,
    bg_color: Signal<String>,
    drawing_color: Signal<String>,
    line_width: Signal<f64>,
    confirm_modal: Signal<ConfirmModal>,
) -> Element {
    let save_paint = move || {
        let win = window();
        let document = win.document().unwrap();

        let canvas = document
            .get_element_by_id("paint-canvas")
            .unwrap()
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .unwrap();

        let save_img = document.get_element_by_id("save-img").unwrap();

        let save_opt = dump_canvas_to_image(&canvas);

        match save_opt {
            Some(save) => {
                save_img.set_attribute("src", &save).unwrap();
            }
            None => {
                save_img.set_attribute("src", "paint-canvas").unwrap();
            }
        }
    };

    let nft = NFT_STUDIO().mint_nft_dry_run(
        "test".to_string(),
        paint_undo().calculate_score(),
        GAME_TIME().day,
    );

    let close_modal = move |_| {
        show_nft_mint_modal.set(false);
    };

    use_effect(move || {
        save_paint();
    });

    let mint_nft = move || async move {
        if nft.price < 0.01 {
            let msg = "NFT value too low to mint.";
            command_line_output(&msg).await;
            return;
        }

        let msg = format!(
            "Are you sure you want to mint this NFT?\nYou will be paid ${}\nYou will no longer be able to save or edit this image after this action.",
            format_comma_seperator(nft.price, 2)
        );

        confirm_modal.write().msg = msg;
        confirm_modal.write().show = true;

        let confirm = loop {
            let confirm = confirm_modal().confirm;
            if confirm.is_some() {
                confirm_modal.write().confirm = None;
                break confirm.unwrap();
            }
            TimeoutFuture::new(100).await;
        };

        if !confirm {
            return;
        }

        let score = paint_undo().calculate_score();

        let day = GAME_TIME().day;

        let name = format!("Painting - Day {day} - Score {score:.2}");

        let nft = NFT_STUDIO.write().mint_nft(day, name, score);

        let next_rep = NFT_STUDIO().next_rep();
        let hype = NFT_STUDIO().hype;

        let completed = hype / next_rep as f64;

        update_progess_bar("paint-progress", completed * 100.0).await;

        MARKET.write().bank.deposit(nft.price);

        clear_canvas(
            &mut paint_undo,
            &mut bg_color,
            &mut drawing_color,
            &mut line_width,
        )
        .await;

        let msg = format!("NFT Minted: {}", nft.name);
        command_line_output(&msg).await;

        show_nft_mint_modal.set(false);

        DO_SAVE.write().save = true;
    };

    rsx! {

        // Backdrop
        div { class: "backdrop", onclick: close_modal }
        // Modal content
        div {
            class: "window modal pauseModal",
            style: "max-width: 350px;min-width:225px;min-height: 300px;text-align:center;",
            div { class: "title-bar",
                div { class: "title-bar-text", "Mint NFT" }
                div { class: "title-bar-controls",
                    button {
                        class: "close",
                        onclick: close_modal,
                        aria_label: "Close",
                        ""
                    }
                }
            }
            h4 { "Value: ${ format_comma_seperator(nft.price,2) }" }
            br {}
            p { "Right Click NFT to save." }
            div { class: "window-body sunken-panel",
                img {
                    id: "save-img",
                    src: "",
                    style: "min-width: 100%;min-height:300px;"
                }
            }
            br {}
            button {
                class: "",
                style: "margin-bottom: 10px;",
                onclick: move |_| async move {
                    mint_nft().await;
                },
                "Mint NFT"
            }
        }
    }
}

#[component]
pub fn PaintSaveModal(show_paint_save_modal: Signal<bool>) -> Element {
    let save_paint = move || {
        let win = window();
        let document = win.document().unwrap();

        let canvas = document
            .get_element_by_id("paint-canvas")
            .unwrap()
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .unwrap();

        let save_img = document.get_element_by_id("save-img").unwrap();

        let save_opt = dump_canvas_to_image(&canvas);

        match save_opt {
            Some(save) => {
                save_img.set_attribute("src", &save).unwrap();
            }
            None => {
                save_img.set_attribute("src", "paint-canvas").unwrap();
            }
        }
    };

    let close_modal = move |_| {
        show_paint_save_modal.set(false);
    };

    use_effect(move || {
        save_paint();
    });

    rsx! {

        // Backdrop
        div { class: "backdrop", onclick: close_modal }
        // Modal content
        div {
            class: "window modal pauseModal",
            style: "max-width: 350px;min-width:225px;min-height: 300px;text-align:center;",
            div { class: "title-bar",
                div { class: "title-bar-text", "Save Painting" }
                div { class: "title-bar-controls",
                    button {
                        class: "close",
                        onclick: close_modal,
                        aria_label: "Close",
                        ""
                    }
                }
            }
            p { "Right Click image to save." }
            div { class: "window-body sunken-panel",
                img {
                    id: "save-img",
                    src: "",
                    style: "min-width: 100%;min-height:300px;"
                }
            }
        }
    }
}

fn set_canvas_background(color: &str, paint_undo: Signal<PaintUndo>) {
    let win = window();
    let document = win.document().unwrap();

    let canvas = document
        .get_element_by_id("paint-canvas")
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .unwrap();

    let context = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();

    context.set_fill_style(&JsValue::from_str(color));
    context.fill_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);

    paint_undo().paths.iter().for_each(|path| {
        if !path.is_empty() {
            context.begin_path();
            path.iter().for_each(|position| {
                context.set_stroke_style(&JsValue::from_str(&position.color));
                context.set_line_width(position.line_width);
                context.line_to(position.x, position.y);
                context.stroke();
                context.begin_path(); // Begin a new path for each segment
                context.move_to(position.x, position.y);
            });
            context.stroke(); // Ensure the last segment is drawn
        }
    });
}

async fn set_canvas_background_from_local() {
    let paint_undo_res = get_paint_undo().await;

    let paint_undo = match paint_undo_res {
        Ok(paint_undo) => match paint_undo {
            Some(paint_undo) => paint_undo,
            None => PaintUndo::new(),
        },
        Err(_) => PaintUndo::new(),
    };

    let win = window();
    let document = win.document().unwrap();

    let canvas = document
        .get_element_by_id("paint-canvas")
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .unwrap();

    let context = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();

    let last_bg_color = match paint_undo.paths.last() {
        Some(path) => match path.last() {
            Some(position) => position.bg_color.clone(),
            None => "white".to_string(),
        },
        None => "white".to_string(),
    };

    context.set_fill_style(&JsValue::from_str(&last_bg_color));
    context.fill_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);

    paint_undo.paths.iter().for_each(|path| {
        if !path.is_empty() {
            context.begin_path();
            path.iter().for_each(|position| {
                context.set_stroke_style(&JsValue::from_str(&position.color));
                context.set_line_width(position.line_width);
                context.line_to(position.x, position.y);
                context.stroke();
                context.begin_path(); // Begin a new path for each segment
                context.move_to(position.x, position.y);
            });
            context.stroke(); // Ensure the last segment is drawn
        }
    });
}

fn set_canvas_background_last(
    paint_undo: Signal<PaintUndo>,
    bg_color: &mut Signal<String>,
    drawing_color: &mut Signal<String>,
    line_width: &mut Signal<f64>,
) {
    let win = window();
    let document = win.document().unwrap();

    let canvas = document
        .get_element_by_id("paint-canvas")
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .unwrap();

    let context = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();

    let (last_bg_color, last_color, last_line_width) = match paint_undo().paths.last() {
        Some(path) => match path.last() {
            Some(position) => (
                position.bg_color.clone(),
                position.color.clone(),
                position.line_width.clone(),
            ),
            None => ("#ffffff".to_string(), "#000".to_string(), 3.0),
        },
        None => ("#ffffff".to_string(), "#000".to_string(), 3.0),
    };

    context.set_fill_style(&JsValue::from_str(&last_bg_color));
    context.fill_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);

    paint_undo().paths.iter().for_each(|path| {
        if !path.is_empty() {
            context.begin_path();
            path.iter().for_each(|position| {
                context.set_stroke_style(&JsValue::from_str(&position.color));
                context.set_line_width(position.line_width);
                context.line_to(position.x, position.y);
                context.stroke();
                context.begin_path(); // Begin a new path for each segment
                context.move_to(position.x, position.y);
            });
            context.stroke(); // Ensure the last segment is drawn
        }
    });

    bg_color.set(last_bg_color);
    drawing_color.set(last_color);
    line_width.set(last_line_width);
}

#[component]
pub fn ProgressBar(progress_id: String, progress_message: String) -> Element {
    rsx! {
        div { class: "progress-bar sunken-panel", style: "overflow: hidden;",
            div {
                id: format!("{}-pbar", progress_id),
                class: "progress",
                style: "width: 0%;",
                span {
                    id: format!("{}-pbar-text", progress_id),
                    class: "progress-text",
                    style: "",
                    aria_label: "{progress_message}",
                    "{progress_message}"
                }
            }
        }
    }
}

async fn clear_canvas(
    paint_undo: &mut Signal<PaintUndo>,
    bg_color: &mut Signal<String>,
    drawing_color: &mut Signal<String>,
    line_width: &mut Signal<f64>,
) {
    let document = window().document().unwrap();
    let canvas = document
        .get_element_by_id("paint-canvas")
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .unwrap();
    let context = canvas
        .get_context("2d")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()
        .unwrap();

    context.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);
    paint_undo.write().clear();

    set_paint_undo(&paint_undo().clone()).await;
    bg_color.set("#ffffff".to_string());
    drawing_color.set("#000".to_string());
    line_width.set(3.0);
}

#[component]
pub fn Header(ticks_per_second: Signal<TpsCounter>, selected_tab: Signal<String>) -> Element {
    let pause_game = {
        move |_| async move {
            if let Some(mut galaxy_save_details) = GALAXY_SAVE_DETAILS() {
                if galaxy_save_details.active && galaxy_save_details.slot.is_some() {
                    galaxy_save_details.force_save = true;
                    *GALAXY_SAVE_DETAILS.write() = Some(galaxy_save_details);
                }
            }
            IS_PAUSED.write().toggle();
        }
    };

    let hash_rate = {
        let sel = SELECTION().clone();
        let coin_selections = sel.get_selected();

        let mineable = coin_selections
            .iter()
            .filter(|c| {
                let mkt = MARKET().clone();

                let coin = mkt.coin_by_name(&c.name);
                match coin {
                    Some(coin) => coin.active && coin.blocks < coin.max_blocks,
                    None => false,
                }
            })
            .count()
            .max(1) as u64;

        let rig_hash = MINING_RIG().get_hash_rate() / mineable;

        let coin_hash = {
            let selected_tab: String = match selected_tab().as_str() {
                tab if tab.starts_with("mining") => {
                    let tab = tab.split("-").collect::<Vec<&str>>();
                    let sel = SELECTION().clone();
                    let selected_coin = sel.selection_by_index(tab[1].parse::<usize>().unwrap());
                    match selected_coin {
                        Some(coin) => coin.name.to_owned(),
                        None => "Not Mining".to_string(),
                    }
                }
                "details" => match SELECTION().get_first_selection() {
                    Some(coin) => coin.name.to_owned(),
                    None => "Not Mining".to_string(),
                },
                _ => "Not Mining".to_string(),
            };

            let mkt = MARKET().clone();

            let coin = mkt.coin_by_name(&selected_tab);
            match coin {
                Some(coin) => coin.get_effective_hash(rig_hash),
                None => 0.0,
            }
        };
        format!(
            "{} | Effective {}",
            format_comma_seperator(rig_hash, 0),
            format_comma_seperator(coin_hash, 2)
        )
    };

    let coin_balance = {
        let selected_tab: String = match selected_tab().as_str() {
            tab if tab.starts_with("mining") => {
                let tab = tab.split("-").collect::<Vec<&str>>();
                let sel = SELECTION().clone();
                let selected_coin = sel.selection_by_index(tab[1].parse::<usize>().unwrap());
                match selected_coin {
                    Some(coin) => coin.name.to_owned(),
                    None => "Not Mining".to_string(),
                }
            }
            "details" => match SELECTION().get_first_selection() {
                Some(coin) => coin.name.to_owned(),
                None => "Not Mining".to_string(),
            },
            _ => "Not Mining".to_string(),
        };

        let mkt = MARKET().clone();

        let coin = mkt.coin_by_name(&selected_tab);
        match coin {
            Some(coin) => coin.balance,
            None => 0.0,
        }
    };

    let get_currently_mining = {
        let selected_tab: String = match selected_tab().as_str() {
            tab if tab.starts_with("mining") => {
                let tab = tab.split("-").collect::<Vec<&str>>();
                let sel = SELECTION().clone();
                let selected_coin = sel.selection_by_index(tab[1].parse::<usize>().unwrap());
                match selected_coin {
                    Some(coin) => coin.name.to_owned(),
                    None => "Not Mining".to_string(),
                }
            }
            "details" => match SELECTION().get_first_selection() {
                Some(coin) => coin.name.to_owned(),
                None => "Not Mining".to_string(),
            },
            _ => "Not Mining".to_string(),
        };

        selected_tab
    };

    let get_coin_blocks = {
        let selected_tab: String = match selected_tab().as_str() {
            tab if tab.starts_with("mining") => {
                let tab = tab.split("-").collect::<Vec<&str>>();
                let sel = SELECTION().clone();
                let selected_coin = sel.selection_by_index(tab[1].parse::<usize>().unwrap());
                match selected_coin {
                    Some(coin) => coin.name.to_owned(),
                    None => "Not Mining".to_string(),
                }
            }
            "details" => match SELECTION().get_first_selection() {
                Some(coin) => coin.name.to_owned(),
                None => "Not Mining".to_string(),
            },

            _ => "Not Mining".to_string(),
        };

        let mkt = MARKET().clone();

        let coin = mkt.coin_by_name(&selected_tab);
        let blocks = match coin {
            Some(coin) => coin.blocks,
            None => 0,
        };

        let max_blocks = match coin {
            Some(coin) => coin.max_blocks,
            None => 0,
        };

        format!("{blocks} / {max_blocks}")
    };

    let get_shares = {
        let selected_tab: String = match selected_tab().as_str() {
            tab if tab.starts_with("mining") => {
                let tab = tab.split("-").collect::<Vec<&str>>();
                let sel = SELECTION().clone();
                let selected_coin = sel.selection_by_index(tab[1].parse::<usize>().unwrap());
                match selected_coin {
                    Some(coin) => coin.name.to_owned(),
                    None => "Not Mining".to_string(),
                }
            }
            "details" => match SELECTION().get_first_selection() {
                Some(coin) => coin.name.to_owned(),
                None => "Not Mining".to_string(),
            },
            _ => "Not Mining".to_string(),
        };

        let mkt = MARKET().clone();

        let coin = mkt.coin_by_name(&selected_tab);
        let shares = match coin {
            Some(coin) => coin.shares,
            None => 0.0,
        };

        let shares_per_block = match coin {
            Some(coin) => coin.shares_per_block,
            None => 0,
        };

        format!("{shares:.0} / {shares_per_block:.0}")
    };

    let show_help_modal = {
        move || {
            IS_PAUSED.write().btn_text = "Resume".to_string();
            IS_PAUSED.write().toggle();
            HELP_MODAL.write().show = true;
        }
    };

    rsx! {
        div { class: "relative items-center justify-center container",
            div { class: "aspect-w-1 aspect-h-1 overflow-hidden window h-fit",

                div { class: "title-bar",
                    div { class: "title-bar-text", "Hash Quest" }
                    div { class: "title-bar-controls",
                        button {
                            class: "close",
                            aria_label: "Help",
                            onclick: move |_| {
                                show_help_modal();
                            },
                            ""
                        }
                        button {
                            class: "close",
                            aria_label: "Close",
                            onclick: |_| {
                                info!("Closing window");
                            },
                            ""
                        }
                    }
                }

                div { class: "window-body",
                    div {
                        class: "flex flex-row",
                        style: "justify-content: space-between;",
                        div {
                            h4 { "Bank: ${format_comma_seperator(MARKET().bank.balance, 2)}" }
                            h5 { "Currently Mining: {get_currently_mining}" }
                            p { "Coins: {format_comma_seperator(coin_balance, 5)}" }
                            p { "Shares: {get_shares}" }
                            p { "Blocks: {get_coin_blocks}" }
                            p { "Hash Rate: {hash_rate}" }
                        }
                        div {
                            img {
                                class: "",
                                width: "100",
                                src: "/android-chrome-192x192.png",
                                alt: "Hash Quest Logo"
                            }
                        }
                    }
                }

                div {
                    class: "flex flex-row",
                    style: "justify-content: space-between;margin:3px;",
                    div { class: "status-bar",
                        p {
                            class: "status-bar-field p-1 font-mono p-2",
                            style: "font-family: 'Courier New', Courier, monospace;padding:4px;",
                            "{format_game_time(&GAME_TIME())}"
                        }
                        p {
                            class: "status-bar-field p-1 font-mono p-2",
                            style: "font-family: 'Courier New', Courier, monospace;padding:4px;",
                            "{ticks_per_second().tps:.2} TPS"
                        }
                    }

                    div { class: "ml-auto",
                        p { class: "",
                            div { class: "justify-end w-full mt-2",
                                button {
                                    class: "",
                                    style: "",
                                    onclick: pause_game,
                                    "{IS_PAUSED().btn_text}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn CommandLine() -> Element {
    rsx! {
        div { class: "relative items-center justify-center container",
            div {
                class: "aspect-w-1 aspect-h-1 overflow-hidden window h-fit",
                style: "height: 292px;",
                div { class: "title-bar",
                    div { class: "title-bar-text", "Command Line" }
                    div { class: "title-bar-controls",
                        button {
                            class: "close",
                            aria_label: "Close",
                            onclick: |_| {
                                info!("Closing window");
                            },
                            ""
                        }
                    }
                }
                div { class: "window-body ",
                    textarea {
                        id: "command-line",
                        class: "w-full text-white",
                        style: "background-color: #000;height: 247px;font-family: 'Consolas', 'Courier New', Courier, monospace;padding: 10px;line-height: 1.75;",
                        disabled: true,
                        resize: "none"
                    }
                }
            }
        }
    }
}

#[component]
pub fn WelcomeModal() -> Element {
    let close_modal = {
        move |_| {
            WELCOME_MODAL.write().show = false;
        }
    };

    rsx! {
        if WELCOME_MODAL().show {
            // Backdrop
            div { class: "backdrop" }
            // Modal content
            div {
                class: "window modal container m-3 overflow-hidden h-fit",
                style: "max-width: 350px;min-width:225px;",
                div { class: "title-bar",
                    div { class: "title-bar-text", "Welcome" }
                    div { class: "title-bar-controls",
                        button {
                            class: "close",
                            aria_label: "Close",
                            onclick: close_modal,
                            ""
                        }
                    }
                }
                div { class: "window-body ",
                    div { class: "p-6  mx-auto",
                        h3 { "Welcome to HashQuest" }

                        br {}

                        p { "HashQuest is a cryptocurrency mining simulation game." }
                        p { "You will start with a basic click powered mining rig." }
                        p { "You can upgrade your rig with new gear to mine more cois faster." }
                        p { "Before long you will be the Hash Rate Lord!" }

                        br {}

                        p { "But watch out for rug pulls!" }
                        p {
                            "Rug pulls can happen at any time, and any balance of that coin is wiped out."
                        }
                        p { "The higher a coins age, the higher the chance of a rug pull." }

                        br {}

                        p { "For more information, click the ? button in the title card" }

                        h4 { "Good luck!" }
                    }

                    button {
                        class: "",
                        style: "margin-top: 10px;",
                        onclick: close_modal,
                        "Start Game"
                    }
                }
            }
        }
    }
}

#[component]
pub fn HelpModal() -> Element {
    let close_modal = {
        move |_| {
            HELP_MODAL.write().show = false;
        }
    };

    rsx! {
        if HELP_MODAL().show {
            // Backdrop
            div { class: "backdrop" }
            // Modal content
            div {
                class: "window modal pauseModal",
                style: "max-width: 350px;min-width:225px;",
                div { class: "title-bar",
                    div { class: "title-bar-text", "Help" }
                    div { class: "title-bar-controls",
                        button {
                            class: "close",
                            aria_label: "Close",
                            onclick: close_modal,
                            ""
                        }
                    }
                }
                div { class: "window-body ",
                    div { class: "p-6  mx-auto",
                        h3 { "How to Play HashQuest" }

                        br {}

                        h4 { "Getting Started" }

                        p { "To start playing HashQuest, you will need to mine a cryptocurrency." }
                        p { "To mine a cryptocurrency, you will need to select a coin to mine." }
                        p {
                            "Once you have selected a coin to mine, mining will begin so long as there is power available."
                        }

                        br {}

                        p {
                            "To power your rig, you can use the 'Click Power' button to charge your power level."
                        }
                        p {
                            "You can also use the 'Fill Power' button to fill your power level to 100% for a fee."
                        }

                        br {}

                        p {
                            "You can sell the coins that you mine for money that is used to upgrade your mining rig."
                        }
                        p {
                            "Upgrades do things like increase hashrate, lower cooldowns, rug pull protection, and automatically refill your power."
                        }

                        br {}

                        p {
                            "Rug pulls can happen at any time. Any balance of a rug pulled coin is lost, so make sure to sell before a rug."
                        }
                        p { "The higher a coins age, the higher the chance of a rug pull." }

                        br {}

                        h4 { "Good Luck!" }
                    }

                    button {
                        class: "",
                        style: "margin-top: 10px;",
                        onclick: close_modal,
                        "Close"
                    }
                }
            }
        }
    }
}

#[component]
pub fn Modal(confirm_modal: Signal<ConfirmModal>) -> Element {
    let close_modal = {
        move |_| {
            IS_PAUSED.write().toggle();
            DO_SAVE.write().save = true;
            if let Some(mut galaxy_save_details) = GALAXY_SAVE_DETAILS() {
                if galaxy_save_details.active && galaxy_save_details.slot.is_some() {
                    galaxy_save_details.force_save = true;
                    *GALAXY_SAVE_DETAILS.write() = Some(galaxy_save_details);
                }
            }
        }
    };

    let new_game = {
        move |_| {
            use_future(move || async move {
                let mut confirm_modal = confirm_modal.clone();

                let msg = "Are you sure you want to start a new game?".to_string();

                confirm_modal.write().msg = msg;
                confirm_modal.write().show = true;

                let confirm = loop {
                    let conf = confirm_modal().confirm;
                    match conf {
                        Some(conf) => {
                            confirm_modal.write().confirm = None;
                            break conf;
                        }
                        None => TimeoutFuture::new(100).await,
                    }
                };

                if confirm {
                    clear_game_state().await;
                    clear_paint_undo().await;

                    let galaxy_host = get_galaxy_host().await;

                    match galaxy_host {
                        Ok(galaxy_host) => match galaxy_host {
                            Some(galaxy_host) => {
                                if galaxy_host.galaxy && galaxy_host.logged_in {
                                    if let Some(galaxy_save_details) = GALAXY_SAVE_DETAILS() {
                                        if galaxy_save_details.active
                                            && galaxy_save_details.slot.is_some()
                                        {
                                            info!("Deleting cloud save");

                                            let save_slot = galaxy_save_details.slot.unwrap();
                                            delete_cloud_save(save_slot).await;
                                        }
                                    };
                                }
                            }
                            None => {}
                        },
                        Err(_) => {}
                    }

                    let win = window();

                    win.location().reload().unwrap();
                }
            });
        }
    };

    let show_help_modal = {
        move || {
            HELP_MODAL.write().show = true;
        }
    };

    let show_import_export_modal = {
        move || {
            IMPORT_EXPORT_MODAL.write().show = true;
        }
    };

    let auto_save_time_opts: Vec<u64> = Vec::from([5, 10, 15, 20, 30, 60, 90, 120, 180, 240, 300]);
    let mut selected_time: Signal<u64> = use_signal(|| 30);

    rsx! {
        if IS_PAUSED().paused {
            // Backdrop
            div { class: "backdrop" }
            // Modal content
            div { class: "window modal pauseModal",
                div { class: "title-bar",
                    div { class: "title-bar-text", "Paused" }
                    div { class: "title-bar-controls",
                        button {
                            class: "close",
                            aria_label: "Help",
                            onclick: move |_| {
                                show_help_modal();
                            },
                            ""
                        }
                        button {
                            class: "close",
                            aria_label: "Close",
                            onclick: close_modal,
                            ""
                        }
                    }
                }
                div { class: "window-body ",
                    div {
                        class: "window",
                        style: "margin-bottom: 10px;padding: 10px;text-align: center;min-width: 225px;",
                        h3 { "Game Paused" }

                        br {}

                        h4 { "Hint" }
                        p { "Add to your home screen to play offline." }

                        br {}

                        if GALAXY_SAVE_DETAILS().is_some() {
                            div { class: "flex flex-col",
                                div {
                                    input {
                                        id: "cloud-save",
                                        class: "",
                                        style: "",
                                        r#type: "checkbox",
                                        checked: GALAXY_SAVE_DETAILS().as_ref().unwrap().active,
                                        onclick: move |_| {
                                            let toggle_autosave = toggle_autosave.clone();
                                            async move {
                                                toggle_autosave().await;
                                            }
                                        },
                                        prevent_default: "onclick"
                                    }
                                    label { class: "", r#for: "cloud-save",
                                        "Autosave to Galaxy.click Cloud"
                                    }
                                }
                                if GALAXY_SAVE_DETAILS().as_ref().unwrap().active {
                                    div {
                                        style: "margin-top: 10px;",
                                        class: "flex flex-col",
                                        label { r#for: "auto-save-time",
                                            "Auto Cloud Save Time (seconds): "
                                        }
                                        select {
                                            id: "auto-save-time",
                                            value: "{GALAXY_SAVE_DETAILS().unwrap().save_interval}",
                                            oninput: move |event| {
                                                if let Ok(value) = event.value().parse::<u64>() {
                                                    selected_time.set(value);
                                                    if let Some(mut galaxy_save_details) = GALAXY_SAVE_DETAILS() {
                                                        if galaxy_save_details.active && galaxy_save_details.slot.is_some() {
                                                            let save_interval = selected_time();
                                                            galaxy_save_details.save_interval = save_interval;
                                                            galaxy_save_details.force_save = true;
                                                            *GALAXY_SAVE_DETAILS.write() = Some(galaxy_save_details);
                                                            DO_SAVE.write().save = true;
                                                        }
                                                    }
                                                }
                                            },
                                            // Generate the dropdown options
                                            for time in auto_save_time_opts.iter() {
                                                option {
                                                    value: "{time}",
                                                    selected: *time == selected_time(),
                                                    "{time}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        div {
                            class: "flex flex-row",
                            style: "justify-content: space-between;",
                            button {
                                class: "",
                                style: "margin-top: 10px;",
                                onclick: move |_| {
                                    show_help_modal();
                                },
                                "Help"
                            }
                            button {
                                class: "",
                                style: "margin-top: 10px;",
                                onclick: move |_| {
                                    show_import_export_modal();
                                },
                                "Import/Export"
                            }
                        }

                        p { "Click Resume to continue your game." }
                    }
                    div {
                        class: "flex flex-row",
                        style: "justify-content: space-between;",
                        button { class: "", onclick: close_modal, "Resume" }
                        button { class: "", onclick: new_game, "New Game" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn ImportExportModal(
    series: Signal<Vec<Vec<f64>>>,
    series_labels: Signal<Vec<String>>,
    labels: Signal<Vec<String>>,
) -> Element {
    let close_modal = {
        move |_| {
            IMPORT_EXPORT_MODAL.write().show = false;
        }
    };

    let clear_textarea = {
        move || {
            let window = window();
            let document = window.document().expect("document not found");
            let textarea = document
                .get_element_by_id("import-export-textarea")
                .expect("textarea not found")
                .dyn_into::<web_sys::HtmlTextAreaElement>()
                .expect("textarea not found");
            textarea.set_value("");
        }
    };

    let export_game = {
        move || {
            use_future(move || async move {
                let game_state_res = get_game_state().await;

                let game_state_opt = match game_state_res {
                    Ok(game_state) => game_state,
                    Err(_) => None,
                };

                let game_state = match game_state_opt {
                    Some(game_state) => game_state,
                    None => {
                        let _ = window().alert_with_message("Failed to export game data.");
                        return;
                    }
                };

                let game_state = export_game_state(&game_state).await;

                match game_state {
                    Some(game_state) => {
                        let window = window();
                        let clipboard = window.navigator().clipboard();

                        let result: js_sys::Promise = clipboard.write_text(&game_state);
                        let future = JsFuture::from(result);

                        match future.await {
                            Ok(_) => {
                                spawn_local(async move {
                                    command_line_output("Game data copied to clipboard.").await;
                                });

                                let document = window.document().expect("document not found");
                                let export_button = document
                                    .get_element_by_id("export-button")
                                    .expect("export button not found")
                                    .dyn_into::<web_sys::HtmlButtonElement>()
                                    .expect("export button not found");

                                let _ = window.alert_with_message("Game data copied to clipboard.\nUse this data to import your game later.\n\nKeep it safe!");

                                export_button.set_disabled(true);
                                export_button.set_inner_text("Copied");

                                TimeoutFuture::new(2000).await;

                                export_button.set_disabled(false);
                                export_button.set_inner_text("Export");
                            }
                            Err(_) => {
                                let _ = window
                                    .alert_with_message("Failed to copy game data to clipboard.");
                            }
                        }
                    }
                    None => {}
                }
            })
        }
    };

    let import_game_data = {
        move || {
            let win = window();
            let document = win.document().expect("document not found");
            let textarea = document
                .get_element_by_id("import-export-textarea")
                .expect("textarea not found")
                .dyn_into::<web_sys::HtmlTextAreaElement>()
                .expect("textarea not found");

            let import_button = document
                .get_element_by_id("import-button")
                .expect("import button not found")
                .dyn_into::<web_sys::HtmlButtonElement>()
                .expect("import button not found");

            import_button.set_disabled(true);
            import_button.set_inner_text("Importing...");

            let game_data = textarea.value();
            let game_data = game_data.trim().to_string();
            let game_clone = game_data.clone();

            if game_data.is_empty() {
                spawn_local(async move {
                    command_line_output("No Game data to import.").await;
                });

                import_button.set_disabled(false);
                import_button.set_inner_text("Import");

                return;
            }

            use_future(move || {
                let game_clone = game_clone.clone();
                async move {
                    let res = load_game_from_string(game_clone).await;
                    let win = window();

                    match res {
                        true => {
                            let _ = win.alert_with_message(
                                "Game data imported successfully!\nThe game will now reload.",
                            );
                            win.location().reload().unwrap();
                        }
                        false => {
                            let _ = win.alert_with_message(
                                "Failed to import game data.\nPlease check the data and try again.",
                            );

                            let document = win.document().expect("document not found");

                            let import_button = document
                                .get_element_by_id("import-button")
                                .expect("import button not found")
                                .dyn_into::<web_sys::HtmlButtonElement>()
                                .expect("import button not found");

                            import_button.set_disabled(false);
                            import_button.set_inner_text("Import");
                        }
                    }
                }
            });
        }
    };

    rsx! {
        if IMPORT_EXPORT_MODAL().show {
            // Backdrop
            div { class: "backdrop" }
            // Modal content
            div { class: "window modal pauseModal",
                div { class: "title-bar",
                    div { class: "title-bar-text", "Import/Export" }
                    div { class: "title-bar-controls",
                        button {
                            class: "close",
                            aria_label: "Close",
                            onclick: close_modal,
                            ""
                        }
                    }
                }
                div { class: "window-body ",
                    div {
                        class: "window",
                        style: "margin-bottom: 10px;padding: 10px;text-align: center;min-width: 225px;",
                        h3 { "Import/Export Game" }

                        br {}

                        p { style: "font-size: small;",
                            "To import a game, paste your game data below."
                        }
                        textarea {
                            id: "import-export-textarea",
                            class: "w-full",
                            style: "font-family: 'Consolas', 'Courier New', Courier, monospace;padding: 10px;line-height: 1.75;",
                            cols: "30",
                            resize: "none"
                        }

                        div {
                            class: "flex flex-row",
                            style: "justify-content: space-between;",
                            button {
                                id: "import-button",
                                class: "",
                                style: "margin-top: 10px;",
                                onclick: move |_| {
                                    import_game_data();
                                },
                                "Import"
                            }
                            button {
                                class: "",
                                style: "margin-top: 10px;",
                                onclick: move |_| {
                                    clear_textarea();
                                },
                                "Clear"
                            }
                        }

                        br {}

                        p { style: "font-size: small;", "To export a game, click the button below." }
                        p { style: "font-size: small;",
                            "Save the copied data in a safe place to import your game later."
                        }

                        div {
                            class: "flex flex-row",
                            style: "justify-content: space-between;",
                            button {
                                id: "export-button",
                                class: "",
                                style: "margin-top: 10px;",
                                onclick: move |_| {
                                    export_game();
                                },
                                "Export"
                            }
                        }
                        p { style: "font-size: small;margin-top: 10px;",
                            span { "We recommend using " }
                            span {
                                a {
                                    href: "https://e2epaste.xyz",
                                    target: "_blank",
                                    "e2epaste.xyz"
                                }
                            }
                            span { " to securly transfer your game data to a different device." }
                        }
                    }
                    div {
                        class: "flex flex-row",
                        style: "justify-content: space-between;",
                        button { class: "", onclick: close_modal, "Close" }
                    }
                }
            }
        }
    }
}

#[component]
pub fn BuyModal(
    series: Signal<Vec<Vec<f64>>>,
    series_labels: Signal<Vec<String>>,
    labels: Signal<Vec<String>>,
    confirm_modal: Signal<ConfirmModal>,
) -> Element {
    let close_modal = {
        move |_| {
            BUY_MODAL.write().show = false;
            BUY_MODAL.write().coin = None;
        }
    };

    let coin_name = {
        let coin = BUY_MODAL().coin.clone();

        match coin {
            Some(coin) => coin.name,
            None => "No Coin".to_string(),
        }
    };

    let coin_name_buy = coin_name.clone();
    let coin_name_sell = coin_name.clone();
    let coin_name_can_sell = coin_name.clone();
    let coin_name_replace = coin_name.clone();
    let coin_name_can_sell_max = coin_name.clone();

    let max_buyable = {
        let mkt = MARKET().clone();
        let coin = mkt.coin_by_name(&coin_name);
        let max_buyable = match coin {
            Some(coin) => {
                let amt = mkt.get_max_buyable(&coin);
                if amt < 0.00001 {
                    0.0
                } else {
                    amt
                }
            }
            None => 0.0,
        };
        max_buyable
    };

    let can_buy_amount = move |amount| {
        if max_buyable < 0.00001 {
            return false;
        }
        max_buyable >= amount
    };

    let do_buy = move |amount, do_max| {
        let mkt = MARKET();

        let coin = mkt.coin_by_name(&coin_name_buy);
        let mut mkt_mut = MARKET.write();
        match coin {
            Some(coin) => {
                let buy_res = if do_max {
                    mkt_mut.buy_max_coin(&coin)
                } else {
                    mkt_mut.buy_coin(&coin, amount)
                };

                let msg = if buy_res {
                    format!("Purchase of {amount} {coin_name_buy} successful.")
                } else {
                    format!("Purchase of {amount} {coin_name_buy} failed.")
                };
                spawn_local(async move {
                    command_line_output(&msg).await;
                });

                DO_SAVE.write().save = true;
            }
            None => {}
        }
    };

    let can_sell_amount = move |amount| {
        let mkt = MARKET();
        let coin = mkt.coin_by_name(&coin_name_can_sell);
        match coin {
            Some(coin) => coin.balance >= amount,
            None => false,
        }
    };

    let do_sell = move |amount, do_max| {
        let mkt = MARKET();
        let mut mut_mkt = MARKET.write();
        let coin = mkt.coin_by_name(&coin_name_sell);

        match coin {
            Some(coin) => {
                let amount = if do_max { coin.balance } else { amount };
                let amount_opt = if do_max { None } else { Some(amount) };

                let price = coin.current_price;
                let total = amount * price;
                let name = coin.name.clone();
                if amount > 0.0 {
                    mut_mkt.sell_coins(&coin, amount_opt);
                    DO_SAVE.write().save = true;
                    let msg = format!("Sold {amount} {name} for ${total}");
                    spawn_local(async move {
                        command_line_output(&msg).await;
                    });

                    DO_SAVE.write().save = true;
                }
            }
            None => {}
        }
    };

    let coin_balance = {
        let mkt = MARKET().clone();
        let coin = mkt.coin_by_name(&coin_name);
        let coin_balance = match coin {
            Some(coin) => coin.balance,
            None => 0.0,
        };
        coin_balance
    };

    let coin_price = {
        let mkt = MARKET().clone();
        let coin = mkt.coin_by_name(&coin_name);
        let coin_price = match coin {
            Some(coin) => coin.current_price,
            None => 0.0,
        };
        coin_price
    };

    rsx! {
        if BUY_MODAL().show {
            // Backdrop
            div { class: "backdrop", onclick: close_modal }
            // Modal content
            div { class: "window modal pauseModal",
                div { class: "title-bar",
                    div { class: "title-bar-text", "Market" }
                    div { class: "title-bar-controls",
                        button {
                            class: "close",
                            aria_label: "Close",
                            onclick: close_modal,
                            ""
                        }
                    }
                }
                div { class: "window-body ",
                    div {
                        class: "window",
                        style: "margin-bottom: 10px;padding: 10px;text-align: center;min-width: 225px;",
                        h3 { "{coin_name} Market" }
                        br {}
                        p { style: "font-size:small;",
                            "Current Price: ${format_comma_seperator(coin_price, 2)}"
                        }
                        p { style: "font-size:small;",
                            "Bank Balance: ${format_comma_seperator(MARKET().bank.balance, 5)}"
                        }
                        p { style: "font-size:small;",
                            "Max Purchase: {format_comma_seperator(max_buyable, 5)}"
                        }
                        p { style: "font-size:small;",
                            "Coin Balance: {format_comma_seperator(coin_balance, 5)}"
                        }
                        br {}
                        p { style: "font-size: medium;", "Buy" }
                        div {
                            class: "market-buttons",
                            style: "justify-content: space-between;margin-bottom: 10px;",
                            button {
                                class: "sell-btn market",
                                disabled: !can_buy_amount(1.0),
                                onclick: {
                                    let do_buy = do_buy.clone();
                                    move |_| {
                                        do_buy(1.0, false);
                                    }
                                },
                                "+1"
                            }
                            button {
                                class: "sell-btn market",
                                disabled: !can_buy_amount(10.0),
                                onclick: {
                                    let do_buy = do_buy.clone();
                                    move |_| {
                                        do_buy(10.0, false);
                                    }
                                },
                                "+10"
                            }
                            button {
                                class: "sell-btn market",
                                disabled: !can_buy_amount(100.0),
                                onclick: {
                                    let do_buy = do_buy.clone();
                                    move |_| {
                                        do_buy(100.0, false);
                                    }
                                },
                                "+100"
                            }
                            button {
                                class: "sell-btn market",
                                disabled: !can_buy_amount(max_buyable),
                                onclick: {
                                    let do_buy = do_buy.clone();
                                    move |_| {
                                        do_buy(max_buyable, true);
                                    }
                                },
                                "Max"
                            }
                        }
                        p { style: "font-size: medium;", "Sell" }
                        div {
                            class: "market-buttons",
                            style: "justify-content: space-between;",
                            button {
                                class: "sell-btn market",
                                disabled: !can_sell_amount(1.0),
                                onclick: {
                                    let do_sell = do_sell.clone();
                                    move |_| {
                                        do_sell(1.0, false);
                                    }
                                },
                                "-1"
                            }
                            button {
                                class: "sell-btn market",
                                disabled: !can_sell_amount(10.0),
                                onclick: {
                                    let do_sell = do_sell.clone();
                                    move |_| {
                                        do_sell(10.0, false);
                                    }
                                },
                                "-10"
                            }
                            button {
                                class: "sell-btn market",
                                disabled: !can_sell_amount(100.0),
                                onclick: {
                                    let do_sell = do_sell.clone();
                                    move |_| {
                                        do_sell(100.0, false);
                                    }
                                },
                                "-100"
                            }
                            button {
                                class: "sell-btn market",
                                disabled: {
                                    let coin_name = coin_name_can_sell_max.clone();
                                    let mkt = MARKET().clone();
                                    let coin = mkt.coin_by_name(&coin_name);
                                    match coin {
                                        Some(coin) => coin.balance <= 0.0,
                                        None => true,
                                    }
                                },
                                onclick: {
                                    let do_sell = do_sell.clone();
                                    move |_| {
                                        do_sell(max_buyable, true);
                                    }
                                },
                                "Max"
                            }
                        }
                    }
                    div {
                        class: "flex flex-row",
                        style: "justify-content: space-between;",
                        button { class: "", onclick: close_modal, "Close" }
                        button {
                            class: "",
                            disabled: {
                                let coin_name = coin_name_replace.clone();
                                let mkt = MARKET().clone();
                                let coin = mkt.coin_by_name(&coin_name);
                                if coin.is_some() {
                                    let coin = coin.unwrap();
                                    let new_coin_cooldown = MINING_RIG().get_new_coin_cooldown();
                                    if new_coin_cooldown == 0 {
                                        if coin.balance > 0.0 { true } else { false }
                                    } else {
                                        true
                                    }
                                } else {
                                    true
                                }
                            },
                            onclick: move |_| {
                                let coin_name_replace = coin_name_replace.clone();
                                let mut confirm_modal = confirm_modal.clone();
                                async move {
                                    let msg = format!(
                                        "Are you sure you want to dismiss {}?\n\nThis action cannot be undone.",
                                        coin_name_replace,
                                    );
                                    confirm_modal.write().msg = msg;
                                    confirm_modal.write().show = true;
                                    let confirm = loop {
                                        let conf = confirm_modal().confirm;
                                        match conf {
                                            Some(confirm) => {
                                                confirm_modal.write().confirm = None;
                                                break confirm;
                                            }
                                            None => {
                                                TimeoutFuture::new(100).await;
                                            }
                                        };
                                    };
                                    if confirm {
                                        let mut series_labels = series_labels.clone();
                                        let mut series = series.clone();
                                        let mut labels = labels.clone();
                                        let rig_lvl = MINING_RIG().get_level();
                                        let day = GAME_TIME().day;
                                        let coin_name = coin_name_replace.clone();
                                        let mkt = MARKET().clone();
                                        let coin = mkt.coin_by_name(&coin_name);
                                        let coin = match coin {
                                            Some(coin) => coin,
                                            None => return,
                                        };
                                        replace_coin(&coin, &mut series_labels, &mut series, rig_lvl, day);
                                        MINING_RIG.write().set_new_coin_cooldown();
                                        let latest_coin = MARKET().get_newest_coin();
                                        if let Some(coin) = latest_coin {
                                            run_sim_one_day_single(&mut series, &mut labels, &coin);
                                        }
                                        let msg = format!("Dismissed {coin_name}");
                                        spawn_local(async move {
                                            command_line_output(&msg).await;
                                        });
                                        BUY_MODAL.write().show = false;
                                        BUY_MODAL.write().coin = None;
                                        DO_SAVE.write().save = true;
                                    }
                                }
                            },
                            "Dismiss Coin"
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn ConfirmModal(confirm_modal: Signal<ConfirmModal>) -> Element {
    let close_modal = {
        move |_| {
            confirm_modal.write().show = false;
            confirm_modal.write().confirm = Some(false);
        }
    };

    let confirm = {
        move |_| {
            confirm_modal.write().show = false;
            confirm_modal.write().confirm = Some(true);
        }
    };

    rsx! {
        if confirm_modal().show {
            // Backdrop
            div { class: "backdrop", style: "z-index: 1000;" }
            // Modal content
            div { class: "window modal pauseModal", style: "z-index: 1001;",
                div { class: "title-bar",
                    div { class: "title-bar-text", "Confirm" }
                    div { class: "title-bar-controls",
                        button {
                            class: "close",
                            aria_label: "Close",
                            onclick: close_modal,
                            ""
                        }
                    }
                }
                div { class: "window-body ",
                    div {
                        class: "window",
                        style: "margin-bottom: 10px;padding: 10px;text-align: center;min-width: 225px;",
                        h3 { "Confirm" }
                        br {}
                        p { style: "font-size:small;", "{confirm_modal().msg}" }
                        br {}
                        div {
                            class: "flex flex-row",
                            style: "justify-content: space-between;",
                            button { class: "", onclick: close_modal, "Cancel" }
                            button { class: "", onclick: confirm, "Confirm" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn CatchupModal() -> Element {
    let close_modal = {
        move |_| {
            CATCHUP_MODAL.write().cancel = true;
        }
    };

    rsx! {
        if CATCHUP_MODAL().show {
            // Backdrop
            div { class: "backdrop" }
            // Modal content
            div {
                class: "window modal container m-3 overflow-hidden h-fit",
                style: "max-width: 350px;min-width:225px;",
                div { class: "title-bar",
                    div { class: "title-bar-text", "Copying..." }
                    div { class: "title-bar-controls",
                        button {
                            class: "close",
                            aria_label: "Close",
                            onclick: close_modal,
                            ""
                        }
                    }
                }
                div { class: "window-body ",
                    div { class: "p-6  mx-auto",

                        div { class: "file-animation",
                            div { class: "folder" }
                            div { class: "paper",

                                img { src: "/file_windows-2.png" }
                            }
                            div { class: "folder" }
                        }

                        p {
                            class: "",
                            style: "margin-top: 10px;margin-bottom:10px;",
                            "Making up for lost time."
                        }

                        p {
                            "Market simulation {CATCHUP_MODAL().current_sim} of {CATCHUP_MODAL().total_sim}"
                        }

                        p { "ETA: {CATCHUP_MODAL().eta}" }
                        p { style: "margin-bottom:10px;",
                            "Speed up factor: {CATCHUP_MODAL().speed_up:.2}x"
                        }

                        ProgressBar { progress_id: "catch-up".to_string(), progress_message: "".to_string() }
                        div {
                            class: "flex flex-row",
                            style: "justify-content: space-between;margin:3px;",
                            div {
                                style: "margin-top:10px;",
                                class: "status-bar",
                                p { class: "status-bar-field p-1", style: "",
                                    "You may cancel this operation at any time."
                                }
                            }

                            div { class: "ml-auto",
                                p { class: "",
                                    div { class: "justify-end w-full mt-2",
                                        button {
                                            style: "margin-top:10px;",
                                            class: "",
                                            onclick: close_modal,
                                            "Cancel"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn GalaxyLoadingModal() -> Element {
    rsx! {
        if GALAXY_LOADING_MODAL().show {
            // Backdrop
            div { class: "backdrop" }
            // Modal content
            div {
                class: "window modal container m-3 overflow-hidden h-fit",
                style: "max-width: 350px;min-width:225px;",
                div { class: "title-bar",
                    div { class: "title-bar-text", "Copying..." }
                    div { class: "title-bar-controls",
                        button { class: "close", aria_label: "Close", "" }
                    }
                }
                div { class: "window-body ",
                    div { class: "p-6  mx-auto",

                        div { class: "file-animation",
                            div { class: "folder" }
                            div { class: "paper",

                                img { src: "/file_windows-2.png" }
                            }
                            div { class: "folder" }
                        }

                        p {
                            class: "",
                            style: "margin-top: 10px;margin-bottom:10px;",
                            "Loading Galaxy API..."
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn Chart(
    labels: Signal<Vec<String>>,
    series: Signal<Vec<Vec<f64>>>,
    series_labels: Signal<Vec<String>>,
) -> Element {
    let padding_left = use_signal(|| 0);
    let padding_bottom = use_signal(|| 0);

    use_effect(move || {
        let series = series.clone();
        let mut padding_left = padding_left.clone();

        calc_padding(&series, &mut padding_left);
    });

    use_effect(move || {
        let labels = labels.clone();
        let mut padding_bottom = padding_bottom.clone();

        calc_padding_labels(&labels, &mut padding_bottom);
    });

    rsx! {
        div { class: "flex flex-col items-center justify-center",
            div { class: "aspect-w-1 aspect-h-1  overflow-hidden window h-fit",

                div { class: "title-bar",
                    div { class: "title-bar-text", "Market Watch" }
                    div { class: "title-bar-controls",
                        button {
                            class: "close",
                            aria_label: "Close",
                            onclick: |_| {
                                info!("Closing window");
                            },
                            ""
                        }
                    }
                }

                div { class: "window-body text-md status-bar-field",
                    if series().iter().all(|s| s.len() > 0) {
                        LineChart {
                            padding_top: 20,
                            padding_left: padding_left(),
                            padding_right: 100,
                            padding_bottom: padding_bottom(),
                            height: "250px",
                            series: series().into_iter().map(|s| s.into_iter().map(|v| v as f32).collect()).collect(),
                            labels: labels(),
                            label_interpolation: (|v| format!("${}", format_comma_seperator(v, 2))) as fn(f32) -> String,
                            series_labels: series_labels(),
                            show_labels: true,
                            show_lines: false,
                            show_dotted_grid: false,
                            show_grid: false,
                            line_width: "0.25%",
                            dot_size: "0.5%",
                            max_ticks: 12
                        }
                    }
                }
            }
        }
    }
}

fn format_game_time(game_time: &GameTime) -> String {
    let day = if game_time.day < 10 {
        format!("0{}", game_time.day)
    } else {
        game_time.day.to_string()
    };

    let hour = if game_time.hour < 10 {
        format!("0{}", game_time.hour)
    } else {
        game_time.hour.to_string()
    };

    let minute = if game_time.minute < 10 {
        format!("0{}", game_time.minute)
    } else {
        game_time.minute.to_string()
    };

    format!("Day: {}, Time {}:{}", day, hour, minute)
}

fn format_comma_seperator<T: std::fmt::Display + std::str::FromStr>(
    v: T,
    decimals: usize,
) -> String {
    let val = format!("{v:.decimals$}", decimals = decimals);

    let mut final_val = String::new();

    let val_split = val.split('.').collect::<Vec<&str>>();

    let val_iter = val_split[0].chars().rev().enumerate();

    for (i, c) in val_iter {
        if i > 0 && i % 3 == 0 {
            final_val.insert(0, ',');
        }
        final_val.insert(0, c);
    }

    let result = if val_split.len() == 1 {
        final_val
    } else if val_split[1] == "00" {
        final_val
    } else {
        format!("{}.{}", final_val, val_split[1])
    };

    result
}

fn calc_padding_labels(labels: &Signal<Vec<String>>, padding_bottom: &mut Signal<i32>) {
    let mut p_left = 0;

    for i in labels.iter() {
        if i.len() > p_left {
            p_left = i.len();
        }
    }

    padding_bottom.set(p_left as i32 * 20);
}

fn calc_padding(series: &Signal<Vec<Vec<f64>>>, padding_left: &mut Signal<i32>) {
    let mut p_left = 0;

    for i in series.iter() {
        i.iter().for_each(|v| {
            let val = format!("${:.2}", v);

            let val_split = val.split('.').collect::<Vec<&str>>();

            if val.len() > p_left {
                p_left = if val_split[1] == "00" {
                    val.len() - 3
                } else {
                    val.len()
                };
            }
        });
    }

    padding_left.set(p_left as i32 * 10);
}

fn can_upgrade_rig() -> bool {
    let rig = MINING_RIG();

    if MARKET().bank.balance < rig.get_rig_upgrade_cost() {
        true
    } else {
        false
    }
}

async fn update_progess_bar(progress_id: &str, progress: f64) {
    let progress_bar = loop {
        let document = window().document().expect("should have document");

        let p_bar = document.get_element_by_id(&format!("{}-pbar", progress_id));

        match p_bar {
            Some(p_bar) => {
                break p_bar;
            }
            None => {
                TimeoutFuture::new(100).await;
            }
        }
    };

    progress_bar
        .set_attribute("style", &format!("width: {}%", progress))
        .unwrap();
}

async fn toggle_autosave() {
    let save_details = GALAXY_SAVE_DETAILS().clone();

    if let Some(mut galaxy_save_details) = save_details {
        if !galaxy_save_details.active {
            GALAXY_LOADING_MODAL.write().show = true;
            fetch_save_list().await;
            let save_slot = find_save_slot().await;

            GALAXY_LOADING_MODAL.write().show = false;

            if let Some(save_slot) = save_slot {
                galaxy_save_details.slot = Some(save_slot);
                galaxy_save_details.active = true;

                *GALAXY_SAVE_DETAILS.write() = Some(galaxy_save_details.clone());
                DO_SAVE.write().save = true;

                do_cloud_save(save_slot).await;
            } else {
                let win = window();
                let msg = "No save slot found. Please delete a save slot and refresh the page.";
                let _ = win.alert_with_message(msg);
            }
        } else {
            galaxy_save_details.active = false;

            if let Some(save_slot) = galaxy_save_details.slot.take() {
                delete_cloud_save(save_slot).await;
            }

            *GALAXY_SAVE_DETAILS.write() = Some(galaxy_save_details.clone());
            DO_SAVE.write().save = true;
        }
    } else {
        info!("No save details found");
    }
}

fn run_sim_one_day(series: &mut Signal<Vec<Vec<f64>>>, labels: &mut Signal<Vec<String>>) {
    let mut mkt = MARKET.write();
    mkt.simulate_day();

    {
        let mut current_series = series.write();
        for coin in mkt.coins.iter() {
            if !coin.active {
                continue;
            }

            let index = coin.index;

            current_series[index].push(coin.current_price);
            if current_series[index].len() > MAX_SERIES_LENGTH {
                current_series[index].remove(0);
            }
        }
    }

    {
        let mut current_labels = labels.write();
        current_labels.push("|".to_string());
        if current_labels.len() > MAX_SERIES_LENGTH {
            current_labels.remove(0);
        }
    }
}

fn run_sim_one_day_single(
    series: &mut Signal<Vec<Vec<f64>>>,
    labels: &mut Signal<Vec<String>>,
    coin: &CryptoCoin,
) {
    let mut mkt = MARKET.write();
    mkt.simulate_day_single(coin);

    {
        let mut current_series = series.write();
        let index = coin.index;

        current_series[index].push(coin.current_price);
        if current_series[index].len() > MAX_SERIES_LENGTH {
            current_series[index].remove(0);
        }
    }

    {
        let mut current_labels = labels.write();
        current_labels.push("|".to_string());
        if current_labels.len() > MAX_SERIES_LENGTH {
            current_labels.remove(0);
        }
    }
}

async fn do_mining() {
    let mut sel = SELECTION.write().clone();

    let selected_coins = sel.get_selected();

    let mut mkt = MARKET.write();

    for selection in selected_coins.iter() {
        let c_name = selection.clone().name;

        let coin = mkt.coin_by_name(&c_name);

        match coin {
            Some(coin) => {
                if !coin.active {
                    sel.unmake_selection(coin.index);
                    sel.update_ui();

                    DO_SAVE.write().save = true;
                }
            }
            None => {}
        }
    }

    let has_power = MINING_RIG.write().consume_power();
    let hash_rate = MINING_RIG().get_hash_rate();

    if !has_power {
        info!("no power available");

        if MINING_RIG().get_auto_power_fill_active() {
            let refill_time = MINING_RIG().get_auto_power_refill_time();

            let refill_time = match refill_time {
                Some(refill_time) => refill_time,
                None => {
                    let delay = MINING_RIG().get_auto_power_fill_delay() as i64;
                    if delay > 0 {
                        MINING_RIG.write().set_auto_power_refill_time(Some(delay));
                        DO_SAVE.write().save = true;
                        return;
                    } else {
                        0
                    }
                }
            };

            if refill_time == 0 {
                let auto_fill_cost = MINING_RIG().get_auto_power_fill_cost(GAME_TIME().day);

                if mkt.bank.withdraw(auto_fill_cost) {
                    let fill_amount = MINING_RIG().get_auto_power_fill_amount();
                    MINING_RIG.write().fill_to_percent(fill_amount);
                    let power_available = MINING_RIG().get_power_fill();
                    for i in 0..SELECTION().max_selectable {
                        update_progess_bar(
                            &format!("power_available-progress-{}", i),
                            power_available * 100.0,
                        )
                        .await;
                    }
                    MINING_RIG.write().set_auto_power_refill_time(None);
                    DO_SAVE.write().save = true;
                } else {
                    return;
                }
            } else {
                MINING_RIG.write().decrement_auto_power_refill_time();
                return;
            }
        } else {
            return;
        }
    }

    let coin_selections = sel.get_selected();

    let mineable = coin_selections
        .iter()
        .filter(|c| {
            let coin = mkt.coin_by_name(&c.name);
            match coin {
                Some(coin) => coin.active && coin.blocks < coin.max_blocks,
                None => false,
            }
        })
        .count()
        .max(1);

    for selection in coin_selections.iter() {
        let c_name = selection.clone().name;
        let c_index = selection.clone().selection_index;

        let coin = mkt.mut_coin_by_name(&c_name);

        match coin {
            Some(coin) => {
                if coin.active {
                    coin.hash_coin(hash_rate / mineable as u64);

                    let share_progress = coin.get_share_progress() as f64;
                    let block_progress = coin.get_block_progress() as f64;

                    let c_index = c_index.clone();

                    update_progess_bar(
                        &format!("share-progress-{}", c_index.clone()),
                        share_progress * 100.0,
                    )
                    .await;
                    update_progess_bar(
                        &format!("block-progress-{}", c_index.clone()),
                        block_progress * 100.0,
                    )
                    .await;
                }
            }
            None => {
                sel.unmake_selection(selection.index);
                sel.update_ui();
                // update_progess_bar("share-progress", 0.0).await;
                // update_progess_bar("block-progress", 0.0).await;
                DO_SAVE.write().save = true;
            }
        }
        let power_available = MINING_RIG().get_power_fill();
        for i in 0..SELECTION().max_selectable {
            update_progess_bar(
                &format!("power_available-progress-{}", i),
                power_available * 100.0,
            )
            .await;
        }
    }
}

async fn do_fill_power() {
    let day = GAME_TIME().day;
    let power_cost = MINING_RIG().get_power_fill_cost(day);

    if MARKET.write().bank.withdraw(power_cost) {
        MINING_RIG.write().fill_power();
    }

    let power_available = MINING_RIG().get_power_fill();
    for i in 0..SELECTION().max_selectable {
        update_progess_bar(
            &format!("power_available-progress-{}", i),
            power_available * 100.0,
        )
        .await;
    }
}

async fn game_loop(
    series: &mut Signal<Vec<Vec<f64>>>,
    labels: &mut Signal<Vec<String>>,
    series_labels: &mut Signal<Vec<String>>,
    ticks_per_second: &mut Signal<TpsCounter>,
) {
    info!("game loop started");
    let is_save_data = recover_game_state(series, labels, series_labels).await;

    if !is_save_data {
        let mut mkt = MARKET.write().clone();
        let rig_lvl = MINING_RIG().level;

        for i in 0..10 {
            let coin = gen_random_coin_with_set_index(i, rig_lvl);

            mkt.add_coin(coin.clone());
            series_labels.write().push(coin.name.clone());
            let mut current_series = series.write();
            if current_series.len() < i + 1 {
                current_series.push(Vec::new());
            }
        }

        *MARKET.write() = mkt;

        run_sim_one_day(series, labels);
        MARKET.write().set_profit_factor(1);

        let seen_welcome = get_seen_welcome().await.unwrap_or_else(|_| false);
        if !seen_welcome {
            WELCOME_MODAL.write().show = true;
            set_seen_welcome().await;
        }
    }

    let mut iter = 0;

    use_future(move || async move {
        save_game_loop().await;
    });

    let power_available = MINING_RIG().get_power_fill();
    for i in 0..SELECTION().max_selectable {
        update_progess_bar(
            &format!("power_available-progress-{}", i),
            power_available * 100.0,
        )
        .await;
    }

    let next_rep = NFT_STUDIO().next_rep();
    let hype = NFT_STUDIO().hype;

    let completed = hype / next_rep as f64;

    update_progess_bar("paint-progress", completed * 100.0).await;

    loop {
        let is_paused = IS_PAUSED().paused;

        ticks_per_second.write().set_paused(is_paused);

        if is_paused {
            TimeoutFuture::new(100).await;
            continue;
        }

        iter += 1;

        if iter % 4 == 0 {
            GAME_TIME.write().increment();
        }

        if iter >= 60 {
            let rig_lvl = MINING_RIG().level;
            let day = GAME_TIME().day;
            cull_market(series_labels, series, rig_lvl, day.clone());
            run_sim_one_day(series, labels);
            MARKET.write().run_rug_pull(day.clone());

            let sel = SELECTION().clone();
            let coin_selections = sel.get_selected();

            let mineable = coin_selections
                .iter()
                .filter(|c| {
                    let mkt = MARKET().clone();

                    let coin = mkt.coin_by_name(&c.name);
                    match coin {
                        Some(coin) => coin.active && coin.blocks < coin.max_blocks,
                        None => false,
                    }
                })
                .count()
                .max(1);

            MARKET.write().set_profit_factor(mineable);

            iter = 0;
        }

        do_mining().await;

        let new_coin_cooldown = MINING_RIG().get_new_coin_cooldown();

        if new_coin_cooldown > 0 {
            MINING_RIG.write().decrement_new_coin_cooldown();
        }

        let amount_per_tick = NFT_STUDIO().money_per_tick();

        MARKET.write().bank.deposit(amount_per_tick);

        ticks_per_second.write().tick();
        let popularity = NFT_STUDIO.write().decriment_popularity(GAME_TIME().day);

        update_progess_bar("popularity-progress", popularity * 100.0).await;

        let delay = ticks_per_second().delay;

        TimeoutFuture::new(delay).await;
    }
}

async fn save_game_loop() {
    let do_save = || async {
        info!("saving game state");

        use_future(move || async move {
            save_game_state().await;
        });
    };

    let mut pause_save = false;

    let mut count = 0;

    loop {
        if DO_SAVE().save {
            do_save().await;
            DO_SAVE.write().save = false;
        }

        if IS_PAUSED().paused {
            if !pause_save {
                do_save().await;
                pause_save = true;
            }
            TimeoutFuture::new(100).await;
            continue;
        }

        count += 1;

        if count >= 12 || count == 1 {
            do_save().await;
            count = 1;
        }

        pause_save = false;

        TimeoutFuture::new(500).await;
    }
}

async fn recover_game_state(
    series: &mut Signal<Vec<Vec<f64>>>,
    labels: &mut Signal<Vec<String>>,
    series_labels: &mut Signal<Vec<String>>,
) -> bool {
    let mut galaxy_save_data: Option<GameState> = None;

    let galaxy_host = get_galaxy_host().await.unwrap_or_else(|_| None);

    let galaxy_save = match galaxy_host {
        Some(host) => {
            let fetching_galaxy_info = host.info_check_status;

            match fetching_galaxy_info {
                Some(_) => {
                    let time = web_sys::js_sys::Date::new_0();
                    let time_now = time.get_time();
                    let info_check_time = host.info_check_time;
                    let galaxy = host;

                    if galaxy.galaxy && galaxy.logged_in {
                        true
                    } else if !galaxy.galaxy || !galaxy.logged_in {
                        false
                    } else {
                        match info_check_time {
                            Some(info_check_time) if time_now - info_check_time < 300000.0 => false,
                            _ => false,
                        }
                    }
                }
                None => false,
            }
        }
        None => false,
    };

    if galaxy_save {
        fetch_save_list().await;

        let galaxy_data = {
            let galaxy_data = get_galaxy_save_data().await;

            match galaxy_data {
                Some(galaxy_data) => galaxy_data,
                None => String::new(),
            }
        };

        if !galaxy_data.is_empty() {
            let decoded_string = decode_game_string(galaxy_data);

            match game_state_from_string(&decoded_string) {
                Ok(game_state) => {
                    galaxy_save_data = Some(game_state);
                }
                Err(_) => {}
            }
        }
    }

    let game_state = if galaxy_save_data.is_none() {
        if galaxy_save {
            let game_state_opt = get_game_state().await.unwrap_or_else(|_| None);

            match game_state_opt {
                Some(mut game_state) => {
                    let galaxy_save_details = game_state.galaxy_save_details.clone();

                    if galaxy_save_details.is_none() {
                        let win = window();
                        let msg = "You appear to be running from Galaxy.click. Would you like to autosave your game to the cloud?";

                        let confirm = win.confirm_with_message(msg).unwrap_or_else(|_| false);

                        if confirm {
                            let slot_opt = find_save_slot().await;

                            if slot_opt.is_none() {
                                let msg = "No save slots available. Please delete a save slot and refresh the page.";
                                let _ = win.alert_with_message(msg);
                                get_game_state().await.unwrap_or_else(|_| None)
                            } else {
                                let game_slot = slot_opt.unwrap();
                                let active = true;

                                let galaxy_save_details = GalaxySaveDetails {
                                    active,
                                    slot: Some(game_slot),
                                    save_interval: 30,
                                    last_save: 0.0,
                                    force_save: false,
                                };

                                *GALAXY_SAVE_DETAILS.write() = Some(galaxy_save_details.clone());

                                game_state.galaxy_save_details = Some(galaxy_save_details.clone());
                                Some(game_state)
                            }
                        } else {
                            let galaxy_save_details = GalaxySaveDetails {
                                active: false,
                                slot: None,
                                save_interval: 30,
                                last_save: 0.0,
                                force_save: false,
                            };

                            *GALAXY_SAVE_DETAILS.write() = Some(galaxy_save_details.clone());

                            Some(game_state)
                        }
                    } else {
                        *GALAXY_SAVE_DETAILS.write() = galaxy_save_details.clone();

                        Some(game_state)
                    }
                }
                None => {
                    // No local save data

                    let win = window();

                    let msg = "You appear to be running from Galaxy.click. Would you like to autosave your game to the cloud?";

                    let confirm = win.confirm_with_message(msg).unwrap_or_else(|_| false);

                    if confirm {
                        let slot_opt = find_save_slot().await;

                        if slot_opt.is_none() {
                            let msg = "No save slots available. Please delete a save slot and refresh the page.";
                            let _ = win.alert_with_message(msg);
                            get_game_state().await.unwrap_or_else(|_| None)
                        } else {
                            let game_slot = slot_opt.unwrap();
                            let active = true;

                            let galaxy_save_details = GalaxySaveDetails {
                                active,
                                slot: Some(game_slot),
                                save_interval: 30,
                                last_save: 0.0,
                                force_save: false,
                            };

                            *GALAXY_SAVE_DETAILS.write() = Some(galaxy_save_details.clone());
                            GALAXY_LOADING_MODAL.write().show = false;
                            None
                        }
                    } else {
                        let galaxy_save_details = GalaxySaveDetails {
                            active: false,
                            slot: None,
                            save_interval: 30,
                            last_save: 0.0,
                            force_save: false,
                        };

                        *GALAXY_SAVE_DETAILS.write() = Some(galaxy_save_details.clone());
                        get_game_state().await.unwrap_or_else(|_| None)
                    }
                }
            }
        } else {
            get_game_state().await.unwrap_or_else(|_| None)
        }
    } else {
        match galaxy_save_data.clone() {
            Some(game_state) => match game_state.galaxy_save_details {
                Some(galaxy_save_details) => {
                    let do_autosave = galaxy_save_details.active
                        && galaxy_save_details.slot.is_some()
                        && galaxy_save;
                    if do_autosave {
                        let galaxy_save_time = game_state.real_time;

                        let local_save_res = get_game_state().await.unwrap_or_else(|_| None);

                        let local_save_time = match local_save_res.clone() {
                            Some(local_save) => local_save.real_time,
                            None => 0,
                        };

                        if galaxy_save_time > local_save_time {
                            // Galaxy save is newer
                            info!("Galaxy save is newer");
                            *GALAXY_SAVE_DETAILS.write() = Some(galaxy_save_details);
                            galaxy_save_data
                        } else {
                            info!("Local save is newer");
                            // Local save is newer
                            let galaxy_save_details = match local_save_res {
                                Some(local_save) => local_save.galaxy_save_details,
                                None => None,
                            };

                            *GALAXY_SAVE_DETAILS.write() = galaxy_save_details.clone();
                            get_game_state().await.unwrap_or_else(|_| None)
                        }
                    } else {
                        get_game_state().await.unwrap_or_else(|_| None)
                    }
                }
                None => {
                    let galaxy_save_details = GalaxySaveDetails {
                        active: false,
                        slot: None,
                        save_interval: 30,
                        last_save: 0.0,
                        force_save: false,
                    };

                    *GALAXY_SAVE_DETAILS.write() = Some(galaxy_save_details.clone());

                    get_game_state().await.unwrap_or_else(|_| None)
                }
            },

            None => {
                let galaxy_save_details = GalaxySaveDetails {
                    active: false,
                    slot: None,
                    save_interval: 30,
                    last_save: 0.0,
                    force_save: false,
                };

                *GALAXY_SAVE_DETAILS.write() = Some(galaxy_save_details.clone());

                None
            }
        }
    };

    let mut game_state = match game_state {
        Some(game_state) => game_state,
        None => return false,
    };

    GALAXY_LOADING_MODAL.write().show = false;

    command_line_output("Loading saved game...").await;

    if game_state.version.is_none() {
        game_state.market.reverse_price_history();
    }

    match game_state.selection {
        Some(selection) => {
            let sel_name = selection.name.clone();
            let sel_index = selection.index;

            let mut sel_multi = match game_state.selection_multi.clone() {
                Some(sel_multi) => sel_multi,
                None => SelectionMultiList::new(),
            };

            if sel_name.is_some() && sel_index.is_some() {
                sel_multi.make_selection(sel_index.unwrap(), &sel_name.unwrap(), false);
            }

            game_state.selection = None;
        }
        None => {}
    }

    let market_chart_data = game_state.market.get_chart();

    let nft_studio = match game_state.nft_studio {
        Some(nft_studio) => nft_studio,
        None => NftStudio::new(),
    };

    let selection_multi = match game_state.selection_multi {
        Some(selection_multi) => selection_multi,
        None => SelectionMultiList::new(),
    };

    *MARKET.write() = game_state.market;
    *series.write() = market_chart_data.series;
    *labels.write() = market_chart_data.labels;
    *series_labels.write() = market_chart_data.series_labels;
    *GAME_TIME.write() = game_state.game_time;
    *SELECTION.write() = selection_multi;
    *MINING_RIG.write() = game_state.mining_rig;
    *NFT_STUDIO.write() = nft_studio;

    SELECTION().update_ui();

    if game_state.paused.paused {
        IS_PAUSED.write().toggle();
    }

    return true;
}

fn dump_canvas_to_image(canvas: &web_sys::HtmlCanvasElement) -> Option<String> {
    let data_url = canvas.to_data_url_with_type("image/png").ok()?;

    Some(data_url)
}

async fn load_game_from_string(data: String) -> bool {
    let game_state_str = decode_game_string(data);

    let game_state = game_state_from_string(&game_state_str);

    match game_state {
        Ok(game_state) => {
            set_game_state(&game_state).await;

            let galaxy = get_galaxy_host().await.unwrap_or_else(|_| None);

            match galaxy {
                Some(galaxy) => {
                    if galaxy.galaxy && galaxy.logged_in {
                        let do_autosave = match GALAXY_SAVE_DETAILS() {
                            Some(galaxy_save_details) => {
                                galaxy_save_details.active && galaxy_save_details.slot.is_some()
                            }
                            None => false,
                        };

                        if do_autosave {
                            if let Some(galaxy_save_details) = GALAXY_SAVE_DETAILS() {
                                let save_slot = galaxy_save_details.slot.unwrap();
                                do_cloud_save(save_slot).await;
                            };
                        }
                    }
                }
                None => {}
            }

            true
        }
        Err(e) => {
            command_line_output("Failed to load game state.").await;
            info!("Failed to load game state: {:?}", e);
            false
        }
    }
}

fn decode_game_string(data: String) -> String {
    let win = window();

    let game_state_res = win.atob(&data);

    let game_state_str = match game_state_res {
        Ok(game_state_str) => game_state_str,
        Err(_) => {
            spawn_local(async move {
                command_line_output("Failed to load game state.").await;
            });

            return "".to_string();
        }
    };

    game_state_str
}

async fn export_game_state(game_state: &GameState) -> Option<String> {
    let game_state_str = game_state.to_string();

    let window = window();

    let base64 = window.btoa(&game_state_str);

    match base64 {
        Ok(base64) => Some(base64),
        Err(_) => None,
    }
}

async fn save_game_state() {
    let real_time = web_sys::js_sys::Date::new_0();
    let real_time_secs = real_time.get_time() as i64 / 1000;

    let game_state = GameState {
        market: MARKET.read().clone(),
        game_time: GAME_TIME.read().clone(),
        paused: IS_PAUSED.read().clone(),
        real_time: real_time_secs,
        selection: None,
        mining_rig: MINING_RIG.read().clone(),
        galaxy_save_details: GALAXY_SAVE_DETAILS.read().clone(),
        version: Some(1),
        nft_studio: Some(NFT_STUDIO().clone()),
        selection_multi: Some(SELECTION().clone()),
    };

    set_game_state(&game_state).await;

    let galaxy = get_galaxy_host().await.unwrap_or_else(|_| None);

    match galaxy {
        Some(galaxy) => {
            if galaxy.galaxy && galaxy.logged_in {
                let do_autosave = match GALAXY_SAVE_DETAILS() {
                    Some(galaxy_save_details) => {
                        galaxy_save_details.active && galaxy_save_details.slot.is_some()
                    }
                    None => false,
                };

                if do_autosave {
                    let last_save_and_interval = match GALAXY_SAVE_DETAILS() {
                        Some(galaxy_save_details) => {
                            let last_save = (galaxy_save_details.last_save / 1000.0) as i64;
                            let save_interval = galaxy_save_details.save_interval as i64;

                            (last_save, save_interval)
                        }
                        None => (0, 0),
                    };

                    let force_save = match GALAXY_SAVE_DETAILS() {
                        Some(galaxy_save_details) => galaxy_save_details.force_save,
                        None => false,
                    };

                    if real_time_secs > last_save_and_interval.0 + last_save_and_interval.1
                        || force_save
                    {
                        if let Some(galaxy_save_details) = GALAXY_SAVE_DETAILS() {
                            info!("Saving game state to galaxy.");

                            let save_slot = galaxy_save_details.slot.unwrap();
                            do_cloud_save(save_slot).await;

                            let mut galaxy_save_details = galaxy_save_details.clone();

                            if force_save {
                                galaxy_save_details.force_save = false;
                            }

                            galaxy_save_details.last_save = real_time.get_time();
                            *GALAXY_SAVE_DETAILS.write() = Some(galaxy_save_details);
                        };
                    }
                }
            }
        }
        None => {}
    }
}
