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
    clear_game_state, game_state_from_string, get_galaxy_host, get_game_state, get_seen_welcome,
    set_galaxy_host, set_galaxy_response_queue, set_galaxy_save_list, set_game_state,
    set_seen_welcome, GalaxyHost, GalaxyResponseQueue, GalaxySaveList, GameState,
};

mod crypto_coin;
mod galaxy_api;
mod market;
mod mining_rig;
mod utils;

use crypto_coin::CryptoCoin;
use galaxy_api::{
    delete_cloud_save, do_cloud_save, fetch_save_list, find_save_slot, galaxy_info,
    galaxy_response, get_galaxy_save_data,
};
use market::{
    clear_selected_coin, cull_market, gen_random_coin_with_set_index, replace_coin, GAME_TIME,
    MARKET, MAX_SERIES_LENGTH, SELECTION,
};
use mining_rig::MINING_RIG;
use utils::{
    command_line_output, BuyModal, CatchupModal, DoSave, GalaxyLoadingModal, GalaxySaveDetails,
    GameTime, HelpModal, ImportExportModal, Paused, WelcomeModal,
};

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

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(PartialEq, Clone, Debug)]
struct Position {
    x: f64,
    y: f64,
}

fn main() {
    // Init logger
    dioxus_logger::init(Level::INFO).expect("failed to init logger");
    info!("starting app");
    launch(App);
}

#[component]
fn App() -> Element {
    // State to store the series data for the chart
    let series: Signal<Vec<Vec<f32>>> = use_signal(|| vec![vec![]]);
    let labels: Signal<Vec<String>> = use_signal(|| vec![String::new()]);

    let series_labels: Signal<Vec<String>> = use_signal(|| Vec::new());

    let mut game_ready: Signal<bool> = use_signal(|| false);

    let listener = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
        let msg_origin: String = event.origin();

        info!("Message from: {}", msg_origin);

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

                    info!("Referrer: {}", referrer);

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

            async move {
                loop {
                    if game_ready() {
                        break;
                    }
                    TimeoutFuture::new(100).await;
                }

                game_loop(&mut series, &mut labels, &mut series_labels).await;
            }
        });
    });

    rsx! {
        link { rel: "stylesheet", href: "/98css/98.css" }
        link { rel: "stylesheet", href: "main.css?v=1.0" }
        div {
            id: "content",
            class: "flex flex-col items-center justify-center relative",
            style: "margin-top: 15px;margin-bottom: 15px;",

            div { class: " grid grid-cols-1 sm:grid-cols-2 gap-4 px-2 w-5/6",

                div { class: "grid grid-cols-1 w-full gap-4",
                    div { class: "flex-1", Header {} }
                    div { class: "flex-1", HeaderBelow {} }
                }
                div { class: "grid grid-cols-1 w-full gap-4",
                    div { class: "flex-1",
                        Chart { labels, series, series_labels }
                    }
                    div { class: "flex-1", CommandLine {} }
                }
                div { class: "flex-1",
                    Coins { series_labels: series_labels.clone(), series: series.clone(), labels: labels.clone() }
                }
                div { class: "flex-1", Paint {} }
            }
            Footer {}
        }
        Modal {}
        CatchupModal {}
        HelpModal {}
        WelcomeModal {}
        BuyModal { series_labels: series_labels.clone(), series: series.clone(), labels: labels.clone() }
        ImportExportModal { series_labels: series_labels.clone(), series: series.clone(), labels: labels.clone() }
        GalaxyLoadingModal {}
    }
}

#[component]
fn Coins(
    series_labels: Signal<Vec<String>>,
    series: Signal<Vec<Vec<f32>>>,
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
            let seconds = new_coin_cooldown as f32 / 20.0;
            format!("{seconds:.2}s")
        }
    };

    rsx! {
        div { class: "items-center justify-center container",
            div {
                class: "aspect-w-1 aspect-h-1 w-1/2 window ",
                style: "height: 350px;",
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
                div {
                    class: "window-body",
                    style: "max-height: calc(100% - 63px); overflow-y: auto;",

                    div { class: "sunken-panel", style: "",

                        table { class: "interactive w-full",
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
                                style: "height: 262px; overflow-y: auto;",
                                for coin in MARKET().index_sorted_coins(show_inactive()) {
                                    tr {
                                        id: format!("{}-row", coin.name),
                                        onclick: {
                                            let coin = coin.clone();
                                            move |_| do_selection(coin.clone(), true)
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
                                            "${format_chart_price(coin.current_price, 2)}"
                                        }
                                        td { style: "padding: 3px;font-family: 'Courier New', Courier, monospace;",
                                            "{format_chart_price(coin.balance,5)}"
                                        }
                                        td { style: "padding: 3px;",
                                            "${format_chart_price(coin.profit_factor, 2)}"
                                        }
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
                                                    "Sell"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                div {
                    class: "status-bar",
                    style: "width:fit-content;position: relative;bottom: 2px;left: 7px;",
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
pub fn HeaderBelow() -> Element {
    let mut selected_tab: Signal<String> = use_signal(|| "mining".to_string());

    let get_details_tab_class = {
        let can_upgrade_rig = {
            if MARKET().bank.balance > MINING_RIG().get_rig_upgrade_cost() {
                true
            } else {
                false
            }
        };

        let can_upgrade_auto_fill = {
            if MARKET().bank.balance > MINING_RIG().get_auto_power_fill_upgrade_cost()
                && MINING_RIG().get_auto_power_fill_level() < 13
            {
                true
            } else {
                false
            }
        };

        if can_upgrade_rig || can_upgrade_auto_fill {
            "rig-tab upgradeable"
        } else {
            "rig-tab"
        }
    };

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

    rsx! {
        div { class: "items-center justify-center container",
            div { class: "aspect-w-1 aspect-h-1 w-1/2 overflow-hidden window h-fit",
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
                    menu { role: "tablist",
                        li {
                            id: "mining-tab",
                            role: "tab",
                            aria_selected: if selected_tab() == "mining" { "true" } else { "false" },
                            style: "padding:5px;padding-left:10px;padding-right:10px;",
                            onclick: move |_| selected_tab.set("mining".to_string()),
                            p { class: "rig-tab", "Mining" }
                        }
                        li {
                            id: "details-tab",
                            role: "tab",
                            aria_selected: if selected_tab() == "details" { "true" } else { "false" },
                            style: "padding:5px;padding-left:10px;padding-right:10px;",
                            onclick: move |_| selected_tab.set("details".to_string()),
                            p { class: get_details_tab_class, "Details" }
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
                    }

                    RigMiningTab { selected_tab }
                    RigDetailsTab { selected_tab }
                    RigAsicTab { selected_tab }

                    if MINING_RIG().get_level() >= 2 {
                        RigCPUTab { selected_tab }
                    }

                    if MINING_RIG().get_level() >= 5 {
                        RigGPUTab { selected_tab }
                    }

                    if MINING_RIG().get_level() >= 10 {
                        RigRugProtectionTab { selected_tab }
                    }
                }
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
            command_line_output(&msg);
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
                        "Rug Protection Level: {format_chart_price(MINING_RIG().get_rug_protection_level(), 2)}"
                    }
                    p { "Global Share Cooldown Eleminated: {!MINING_RIG().get_global_share_cooldown()}" }
                    p {
                        "Amount Rug Protected: {format_chart_price(MINING_RIG().get_rug_protection_amount() * 100.0, 2)}%"
                    }
                    br {}
                    p { "Rug Protection Upgrade Cost: ${format_chart_price(rug_protection_cost, 2)}" }
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
                command_line_output(&msg);
            }
            DO_SAVE.write().save = true;
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
                    p { "ASICs: {MINING_RIG().get_filled_asic_slots()}" }
                    p { "Hash Rate: {MINING_RIG().get_asic_hash_rate()}" }
                    p { "Power: {MINING_RIG().get_asic_power_usage()}" }
                }
                div {
                    h4 { "ASIC Upgrade" }
                    br {}
                    p { "Upgrade Cost: ${format_chart_price(MINING_RIG().get_asic_upgrade_cost(), 2)}" }
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
                command_line_output(&msg);
            }
            DO_SAVE.write().save = true;
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
                    p { "GPUs: {MINING_RIG().get_filled_gpu_slots()}" }
                    p { "Hash Rate: {MINING_RIG().get_gpu_hash_rate()}" }
                    p { "Power: {MINING_RIG().get_gpu_power_usage()}" }
                }
                div {
                    h4 { "GPU Upgrade" }
                    br {}
                    p { "Upgrade Cost: ${format_chart_price(MINING_RIG().get_gpu_upgrade_cost(), 2)}" }
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
                    p { "Level: {MINING_RIG().get_cpu_level()}" }
                    p { "Hash Rate: {MINING_RIG().get_cpu_hash_rate()}" }
                    p { "Power: {MINING_RIG().get_cpu_power_usage()}" }
                }
                div {
                    h4 { "CPU Upgrade" }
                    br {}

                    if MINING_RIG().get_cpu_level() < 5 {
                        p {
                            "Upgrade Cost: ${format_chart_price(MINING_RIG().get_cpu_upgrade_cost(), 2)}"
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
                        command_line_output(&msg);
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
                command_line_output(&msg);
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
        let delay = MINING_RIG().get_auto_power_fill_delay() as f32 / 20.0;
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

    let can_upgrade_rig = {
        if MARKET().bank.balance < MINING_RIG().get_rig_upgrade_cost() {
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
                    p { "Power Capacity: {format_chart_price(MINING_RIG().get_power_capacity(), 2)}" }
                    p {
                        "GPU Slots: {MINING_RIG().get_filled_gpu_slots()} / {MINING_RIG().get_max_gpu_slots()}"
                    }
                    p {
                        "ASIC Slots: {MINING_RIG().get_filled_asic_slots()} / {MINING_RIG().get_max_asic_slots()}"
                    }
                    br {}
                    p { "Current Hash Rate: {format_chart_price(MINING_RIG().get_hash_rate(), 2)}" }
                    p { "Power Usage: {format_chart_price(MINING_RIG().get_power_usage(), 2)}" }
                    br {}
                    p {
                        "Rig Upgrade Cost: ${format_chart_price(MINING_RIG().get_rig_upgrade_cost(), 2)}"
                    }
                }
                if auto_fill_level > 0 {
                    div { style: "text-align: end;",
                        h4 { "Auto Power Fill" }
                        p { "Level: {MINING_RIG().get_auto_power_fill_level()}" }
                        p { "Fill Amount: {MINING_RIG().get_auto_power_fill_amount() * 100.0:.0}%" }
                        p { "Fill Delay: {fill_delay}" }
                        p {
                            "Fill Cost: ${format_chart_price(MINING_RIG().get_auto_power_fill_cost(GAME_TIME().day), 2)}"
                        }
                        br {}

                        if MINING_RIG().get_auto_power_fill_level() < 13 {
                            p {
                                "Upgrade Cost: ${format_chart_price(MINING_RIG().get_auto_power_fill_upgrade_cost(), 2)}"
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
                            "Enable Cost: ${format_chart_price(MINING_RIG().get_auto_power_fill_upgrade_cost(), 2)}"
                        }
                    }
                }
            }
        }

        div { class: "flex flex-row", style: get_style_buttons(),
            button {
                class: "",
                disabled: can_upgrade_rig,
                onclick: |_| {
                    let cost = MINING_RIG().get_rig_upgrade_cost();
                    if MARKET.write().bank.withdraw(cost) {
                        MINING_RIG.write().upgrade();
                        let rig_lvl = MINING_RIG().get_level();
                        let msg = format!("Rig upgrade successful, new level {rig_lvl}");
                        command_line_output(&msg);
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
pub fn RigMiningTab(selected_tab: Signal<String>) -> Element {
    let toggle_auto_power_fill = {
        move |_| {
            MINING_RIG.write().toggle_auto_power_fill();
            DO_SAVE.write().save = true;
        }
    };

    let selected_coin_name = {
        let selected_coin = get_selected_coin();
        match selected_coin {
            Some(selected_coin) => selected_coin,
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
            if selected_tab() == "mining" {
                "display: block;padding: 10px;"
            } else {
                "display: none;padding: 10px;"
            }
        }
    };

    let get_style_buttons = {
        let selected_tab = selected_tab.clone();
        move || {
            if selected_tab() == "mining" {
                "display: flex;margin-top: 5px;justify-content: space-between;"
            } else {
                "display: none;"
            }
        }
    };

    let get_style_status_bar = {
        let selected_tab = selected_tab.clone();
        move || {
            if selected_tab() == "mining" {
                "display: flex;margin-top: 10px;"
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

    let share_cooldown_details = {
        let mkt = MARKET().clone();

        let selected_coin: Option<String> = get_selected_coin();

        let global_share_cooldown: bool = MINING_RIG().get_global_share_cooldown();

        match selected_coin {
            Some(selected_coin) => {
                let coin = if global_share_cooldown {
                    let cooldown_coin = mkt.get_any_share_cooldown();
                    if cooldown_coin.is_some() {
                        cooldown_coin
                    } else {
                        mkt.coin_by_name(&selected_coin)
                    }
                } else {
                    mkt.coin_by_name(&selected_coin)
                };

                match coin {
                    Some(coin) => (coin.get_share_cooldown(), coin.get_share_cooldown_seconds()),
                    None => (0, 0.0),
                }
            }
            None => (0, 0.0),
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
            ProgressBar {
                progress_id: "share-progress".to_string(),
                progress_message: if share_cooldown_details.0 != 0 {
                    let cooldown_time = share_cooldown_details.1;
                    format!("Cooldown: {:.1}s", cooldown_time)
                } else {
                    "".to_string()
                }
            }
            h4 { "Block Progress" }
            ProgressBar { progress_id: "block-progress".to_string(), progress_message: "".to_string() }
            h4 { "Power Level" }
            ProgressBar {
                progress_id: "power_available-progress".to_string(),
                progress_message: if MINING_RIG().get_auto_power_refill_time() != Some(0)
                    && MINING_RIG().get_auto_power_fill_active()
                {
                    let refill_time = MINING_RIG().get_auto_power_refill_time();
                    if MINING_RIG().get_power_fill() <= 0.2 && refill_time.is_some() {
                        match refill_time {
                            Some(refill_time) => {
                                let refill_time = refill_time as f32 / 20.0;
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
                    update_progess_bar("power_available-progress", power_available * 100.0).await;
                },
                "Click Power"
            }

            div { class: "flex flex-col",
                if MINING_RIG().get_auto_power_fill_level() > 0 {
                    div { style: get_style_status_bar(),
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
        div { class: "status-bar", style: get_style_status_bar(),

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
                        "${format_chart_price(MINING_RIG().get_auto_power_fill_cost(GAME_TIME().day), 2)}"
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
                    "Manual Power Cost"
                }
                p {
                    class: "status-bar-field font-mono",
                    style: "padding:4px;text-align: center;",
                    "${format_chart_price(MINING_RIG().get_power_fill_cost(GAME_TIME().day), 2)}"
                }
            }
        }
    }
}

#[component]
pub fn Paint() -> Element {
    let mut is_drawing = use_signal(|| false);
    let mut last_position = use_signal(|| Position { x: 0.0, y: 0.0 });

    let mut drawing_color = use_signal(|| "black".to_string());

    // Utility function to get position from MouseEvent
    let get_mouse_position = |e: &MouseEvent| -> Position {
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
        }
    };

    // Utility function to get position from TouchEvent
    let get_touch_position = |e: &TouchEvent| -> Position {
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
        }
    };

    // Mouse down handler
    let on_mouse_down = move |e: MouseEvent| {
        is_drawing.set(true);
        let position = get_mouse_position(&e);
        last_position.set(position.clone());

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
    };

    // Touch end handler
    let on_touch_end = move |_| {
        is_drawing.set(false);
    };

    let on_mouse_enter = move |e: MouseEvent| {
        e.held_buttons().iter().for_each(|button| {
            if button == MouseButton::Primary {
                is_drawing.set(true);
                let position = get_mouse_position(&e);
                last_position.set(position.clone());
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
            context.line_to(position.x, position.y);
            context.stroke();

            last_position.set(position.clone());

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

            context.set_stroke_style(&JsValue::from_str("red"));
            context.line_to(position.x, position.y);
            context.stroke();

            last_position.set(position.clone());

            context.begin_path();
            context.move_to(position.x, position.y);
        }
    };

    let clear_canvas = {
        move || {
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
        }
    };

    rsx! {
        div { class: "relative top-8 items-center justify-center container",
            div {
                class: "w-1/2 overflow-hidden window h-fit",
                style: "height: 350px;",
                div { class: "title-bar",
                    div { class: "title-bar-text", "Paint" }
                    div { class: "title-bar-controls",
                        button {
                            class: "close",
                            aria_label: "Close",
                            onclick: move |_| {
                                clear_canvas();
                            },
                            ""
                        }
                    }
                }
                div { class: "window-body h-full",
                    div { class: "sunken-panel",
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
                        class: "palette sunken-panel",
                        style: "display: flex; flex-direction: row; margin-top: 7px;",
                        div {
                            class: "color-button",
                            style: "background-color: black;",
                            onclick: move |_| { drawing_color.set("black".to_string()) },
                            ""
                        }
                        div {
                            class: "color-button",
                            style: "background-color: silver;",
                            onclick: move |_| { drawing_color.set("silver".to_string()) },
                            ""
                        }
                        div {
                            class: "color-button",
                            style: "background-color: gray;",
                            onclick: move |_| { drawing_color.set("gray".to_string()) },
                            ""
                        }
                        div {
                            class: "color-button",
                            style: "background-color: white;",
                            onclick: move |_| { drawing_color.set("white".to_string()) },
                            ""
                        }
                        div {
                            class: "color-button",
                            style: "background-color: maroon;",
                            onclick: move |_| { drawing_color.set("maroon".to_string()) },
                            ""
                        }
                        div {
                            class: "color-button",
                            style: "background-color: red;",
                            onclick: move |_| { drawing_color.set("red".to_string()) },
                            ""
                        }
                        div {
                            class: "color-button",
                            style: "background-color: purple;",
                            onclick: move |_| { drawing_color.set("purple".to_string()) },
                            ""
                        }
                        div {
                            class: "color-button",
                            style: "background-color: fuchsia;",
                            onclick: move |_| { drawing_color.set("fuchsia".to_string()) },
                            ""
                        }
                        div {
                            class: "color-button",
                            style: "background-color: green;",
                            onclick: move |_| { drawing_color.set("green".to_string()) },
                            ""
                        }
                        div {
                            class: "color-button",
                            style: "background-color: lime;",
                            onclick: move |_| { drawing_color.set("lime".to_string()) },
                            ""
                        }
                        div {
                            class: "color-button",
                            style: "background-color: olive;",
                            onclick: move |_| { drawing_color.set("olive".to_string()) },
                            ""
                        }
                        div {
                            class: "color-button",
                            style: "background-color: yellow;",
                            onclick: move |_| { drawing_color.set("yellow".to_string()) },
                            ""
                        }
                        div {
                            class: "color-button",
                            style: "background-color: navy;",
                            onclick: move |_| { drawing_color.set("navy".to_string()) },
                            ""
                        }
                        div {
                            class: "color-button",
                            style: "background-color: blue;",
                            onclick: move |_| { drawing_color.set("blue".to_string()) },
                            ""
                        }
                        div {
                            class: "color-button",
                            style: "background-color: teal;",
                            onclick: move |_| { drawing_color.set("teal".to_string()) },
                            ""
                        }
                        div {
                            class: "color-button",
                            style: "background-color: aqua;",
                            onclick: move |_| { drawing_color.set("aqua".to_string()) },
                            ""
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn ProgressBar(progress_id: String, progress_message: String) -> Element {
    rsx! {
        div { class: "progress-bar sunken-panel", style: "overflow: hidden;",
            div {
                id: format!("{}-pbar", progress_id),
                class: "progress",
                style: "width: 0%",
                span {
                    id: format!("{}-pbar-text", progress_id),
                    style: "position: absolute; width: 100%; text-align: center;",
                    "{progress_message}"
                }
            }
        }
    }
}

#[component]
pub fn Header() -> Element {
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
        let rig_hash = MINING_RIG().get_hash_rate();
        let coin_hash = {
            let selection = SELECTION.read();

            let selection = match &selection.name {
                Some(name) => name.to_string(),
                None => "Not Mining".to_string(),
            };

            let mkt = MARKET().clone();

            let coin = mkt.coin_by_name(&selection);
            match coin {
                Some(coin) => coin.get_effective_hash(rig_hash),
                None => 0.0,
            }
        };
        format!("{rig_hash} | Effective {coin_hash:.2}")
    };

    let coin_balance = {
        let selection = SELECTION.read();

        let selection = match &selection.name {
            Some(name) => name.to_string(),
            None => "Not Mining".to_string(),
        };

        let mkt = MARKET().clone();

        let coin = mkt.coin_by_name(&selection);
        match coin {
            Some(coin) => coin.balance,
            None => 0.0,
        }
    };

    let get_currently_mining = {
        let selection = SELECTION.read();

        let selection = match &selection.name {
            Some(name) => name.to_string(),
            None => "Not Mining".to_string(),
        };

        selection
    };

    let get_coin_blocks = {
        let selection = SELECTION.read();

        let selection = match &selection.name {
            Some(name) => name.to_string(),
            None => "Not Mining".to_string(),
        };

        let mkt = MARKET().clone();

        let coin = mkt.coin_by_name(&selection);
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
        let selection = SELECTION.read();

        let selection = match &selection.name {
            Some(name) => name.to_string(),
            None => "Not Mining".to_string(),
        };

        let mkt = MARKET().clone();

        let coin = mkt.coin_by_name(&selection);
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
        div { class: "relative top-8 items-center justify-center container",
            div { class: "aspect-w-1 aspect-h-1 w-1/2 overflow-hidden window h-fit",

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
                            h4 { "Bank: ${format_chart_price(MARKET().bank.balance, 2)}" }
                            h5 { "Currently Mining: {get_currently_mining}" }
                            p { "Coins: {format_chart_price(coin_balance, 5)}" }
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
        div { class: "relative top-8 items-center justify-center container",
            div {
                class: "aspect-w-1 aspect-h-1 w-1/2 overflow-hidden window h-fit",
                style: "height: 220px;",
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
                        style: "background-color: #000;height: 177px;font-family: 'Consolas', 'Courier New', Courier, monospace;padding: 10px;line-height: 1.75;",
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
                class: "window modal container m-3 overflow-hidden h-fit",
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
pub fn Modal() -> Element {
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
            use_future(move || async {
                let window = window();
                let confirm =
                    window.confirm_with_message("Are you sure you want to start a new game?");
                let confirm = match confirm {
                    Ok(confirm) => confirm,
                    Err(_) => false,
                };

                if confirm {
                    clear_game_state().await;

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

                    window.location().reload().unwrap();
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

    let auto_save_time_opts: Vec<u32> = Vec::from([5, 10, 15, 20, 30, 60, 90, 120, 180, 240, 300]);
    let mut selected_time: Signal<u32> = use_signal(|| 30);

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
                                                if let Ok(value) = event.value().parse::<u32>() {
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
    series: Signal<Vec<Vec<f32>>>,
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
                                command_line_output("Game data copied to clipboard.");
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
                command_line_output("No game data to import.");

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
    series: Signal<Vec<Vec<f32>>>,
    series_labels: Signal<Vec<String>>,
    labels: Signal<Vec<String>>,
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
                command_line_output(&msg);
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
                    command_line_output(&msg);
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
                            "Current Price: ${format_chart_price(coin_price, 2)}"
                        }
                        p { style: "font-size:small;",
                            "Bank Balance: ${format_chart_price(MARKET().bank.balance, 5)}"
                        }
                        p { style: "font-size:small;",
                            "Max Purchase: {format_chart_price(max_buyable, 5)}"
                        }
                        p { style: "font-size:small;",
                            "Coin Balance: {format_chart_price(coin_balance, 5)}"
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
                                        do_buy(max_buyable as f32, true);
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
                                        do_sell(max_buyable as f32, true);
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
                                let window = window();
                                let confirm = window
                                    .confirm_with_message(
                                        "Are you sure you want to dismiss this coin?\n\nThis action cannot be undone.",
                                    );
                                match confirm {
                                    Ok(confirm) => {
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
                                            command_line_output(&msg);
                                            BUY_MODAL.write().show = false;
                                            BUY_MODAL.write().coin = None;
                                            DO_SAVE.write().save = true;
                                        }
                                    }
                                    Err(_) => {}
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
    series: Signal<Vec<Vec<f32>>>,
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
            div { class: "aspect-w-1 aspect-h-1 w-1/2 overflow-hidden window h-fit",

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
                            series: series(),
                            labels: labels(),
                            label_interpolation: (|v| format!("${}", format_chart_price(v, 2))) as fn(f32) -> String,
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

fn format_chart_price<T: std::fmt::Display + std::str::FromStr>(v: T, decimals: usize) -> String {
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

fn calc_padding(series: &Signal<Vec<Vec<f32>>>, padding_left: &mut Signal<i32>) {
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

async fn update_progess_bar(progress_id: &str, progress: f32) {
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

    info!("Toggling autosave");

    if let Some(mut galaxy_save_details) = save_details {
        if !galaxy_save_details.active {
            info!("Autosave to cloud enabled");

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
            info!("Autosave to cloud disabled");
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

fn run_sim_one_day(series: &mut Signal<Vec<Vec<f32>>>, labels: &mut Signal<Vec<String>>) {
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
    series: &mut Signal<Vec<Vec<f32>>>,
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

fn get_selected_coin() -> Option<String> {
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

        if radio.checked() {
            return Some(radio.value());
        }
    }

    None
}

async fn do_selection(coin: CryptoCoin, do_toggle: bool) {
    let coin = coin.clone();
    let coin_name = coin.name.clone();
    let coin_index = coin.index;

    if (!coin.active || IS_PAUSED().paused) && !do_toggle {
        return;
    }

    let toggle_off = if SELECTION().name == Some(coin_name.clone()) && do_toggle {
        true
    } else {
        false
    };

    SELECTION.write().index = None;
    SELECTION.write().name = None;

    loop {
        let window = window();
        let document = window.document().expect("should have document");

        let radios = document
            .query_selector_all("input[name='coin-selection']")
            .expect("should have radios");

        if radios.length() < 1 {
            TimeoutFuture::new(100).await;
            continue;
        }

        for i in 0..radios.length() {
            let radio = radios.get(i).expect("should have radio");
            let radio = radio
                .dyn_into::<web_sys::HtmlInputElement>()
                .expect("should be a radio");

            radio.set_checked(false);

            if radio.id() == coin_name && !toggle_off {
                radio.set_checked(true);
                SELECTION.write().index = Some(coin_index);
                SELECTION.write().name = Some(coin_name.clone());
            }
        }

        let rows = document.query_selector_all("tr").expect("should have rows");

        for i in 0..rows.length() {
            let row = rows.get(i).expect("should have row");
            let row = row.dyn_into::<web_sys::Element>().expect("should be a row");

            row.set_class_name("");

            if row.id() == format!("{}-row", coin_name) && !toggle_off {
                row.set_class_name(&format!("selected-{}", coin_index));
            }
        }

        break;
    }

    let share_progress = coin.get_share_progress();
    let block_progress = coin.get_block_progress();

    if coin.get_share_cooldown() == 0 {
        update_progess_bar("share-progress", share_progress * 100.0).await;
    }

    update_progess_bar("block-progress", block_progress * 100.0).await;

    DO_SAVE.write().save = true;
}

async fn do_mining() {
    let selected_coin = get_selected_coin();
    let mut mkt = MARKET.write();

    let selected_coin = match selected_coin {
        Some(selected_coin) => selected_coin,
        None => {
            SELECTION.write().index = None;
            SELECTION.write().name = None;
            clear_selected_coin();
            update_progess_bar("share-progress", 0.0).await;
            update_progess_bar("block-progress", 0.0).await;
            return;
        }
    };

    let global_share_cooldown = MINING_RIG().get_global_share_cooldown();

    if global_share_cooldown {
        let cooldown_coin = mkt.mut_get_any_share_cooldown();
        match cooldown_coin {
            Some(coin) => {
                coin.decrement_share_cooldown();
                update_progess_bar("share-progress", 0.0).await;
                return;
            }
            None => {
                //
            }
        };
    }

    {
        let coin = mkt.mut_coin_by_name(&selected_coin);
        match coin {
            Some(coin) => {
                let blocks = coin.blocks;
                let max_blocks = coin.max_blocks;
                if blocks >= max_blocks {
                    return;
                }

                if coin.get_share_cooldown() > 0 {
                    mkt.decrement_all_share_cooldowns();
                    update_progess_bar("share-progress", 0.0).await;
                    return;
                }
            }
            None => {
                //
            }
        }
    }

    if !global_share_cooldown {
        mkt.decrement_all_share_cooldowns();
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
                    update_progess_bar("power_available-progress", power_available * 100.0).await;
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

    let coin = mkt.mut_coin_by_name(&selected_coin);

    let coin: &mut CryptoCoin = match coin {
        Some(coin) => coin,
        None => {
            SELECTION.write().index = None;
            SELECTION.write().name = None;
            clear_selected_coin();
            update_progess_bar("share-progress", 0.0).await;
            update_progess_bar("block-progress", 0.0).await;
            return;
        }
    };

    coin.hash_coin(hash_rate);

    let power_available = MINING_RIG().get_power_fill();

    let share_progress = coin.get_share_progress();
    let block_progress = coin.get_block_progress();

    update_progess_bar("share-progress", share_progress * 100.0).await;
    update_progess_bar("block-progress", block_progress * 100.0).await;
    update_progess_bar("power_available-progress", power_available * 100.0).await;
}

async fn do_fill_power() {
    let day = GAME_TIME().day;
    let power_cost = MINING_RIG().get_power_fill_cost(day);

    if MARKET.write().bank.withdraw(power_cost) {
        MINING_RIG.write().fill_power();
    }

    let power_available = MINING_RIG().get_power_fill();
    update_progess_bar("power_available-progress", power_available * 100.0).await;
}

async fn game_loop(
    series: &mut Signal<Vec<Vec<f32>>>,
    labels: &mut Signal<Vec<String>>,
    series_labels: &mut Signal<Vec<String>>,
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
        MARKET.write().set_profit_factor();

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
    update_progess_bar("power_available-progress", power_available * 100.0).await;

    loop {
        if IS_PAUSED().paused {
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
            MARKET.write().set_profit_factor();

            iter = 0;
        }

        do_mining().await;

        let new_coin_cooldown = MINING_RIG().get_new_coin_cooldown();

        if new_coin_cooldown > 0 {
            MINING_RIG.write().decrement_new_coin_cooldown();
        }

        TimeoutFuture::new(50).await;
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
    series: &mut Signal<Vec<Vec<f32>>>,
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

    command_line_output("Loading saved game...");

    if game_state.version.is_none() {
        game_state.market.reverse_price_history();
    }

    let market_chart_data = game_state.market.get_chart();

    *MARKET.write() = game_state.market;
    *series.write() = market_chart_data.series;
    *labels.write() = market_chart_data.labels;
    *series_labels.write() = market_chart_data.series_labels;
    *GAME_TIME.write() = game_state.game_time;
    *SELECTION.write() = game_state.selection;
    *MINING_RIG.write() = game_state.mining_rig;

    if let Some(selection) = SELECTION().name.clone() {
        let mkt = MARKET().clone();
        let coin = mkt.coin_by_name(&selection);

        match coin {
            Some(coin) => {
                do_selection(coin.clone(), false).await;
            }
            None => {
                SELECTION.write().index = None;
                SELECTION.write().name = None;
            }
        }
    }

    if game_state.paused.paused {
        IS_PAUSED.write().toggle();
    }

    return true;
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
                            info!("Saving game state to galaxy.");

                            info!("Galaxy save details: {:?}", GALAXY_SAVE_DETAILS());

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
        Err(_) => {
            command_line_output("Failed to load game state.");
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
            command_line_output("Failed to load game state.");
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
        selection: SELECTION.read().clone(),
        mining_rig: MINING_RIG.read().clone(),
        galaxy_save_details: GALAXY_SAVE_DETAILS.read().clone(),
        version: Some(1),
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
