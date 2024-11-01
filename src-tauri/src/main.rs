// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use futures::executor::block_on;
use futures_util::StreamExt;
use rodio::{Decoder, OutputStream, Sink};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Cursor;
use std::sync::{Arc, Mutex};
use tauri::Manager;
use tauri::State;
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::task;
use tokio_tungstenite::connect_async;
use tauri::path::BaseDirectory;
use tauri::{AppHandle, Emitter};



#[derive(Debug, Deserialize)]
struct TickerStream {
    #[serde(rename = "s")]
    symbol: String,
    #[serde(rename = "c")]
    price: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PriceAlert {
    price: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct TickerState {
    symbol: String,
    alerts: Vec<PriceAlert>,
    last_price: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AppSettings {
    tickers: Vec<TickerState>,
    selected_sound: String,
    ticker_order: Vec<String>,
}

struct AppState {
    tickers: HashMap<String, TickerState>,
    selected_sound: String,
    websocket_senders: HashMap<String, mpsc::Sender<()>>,
    ticker_order: Vec<String>,
}

#[tauri::command]
fn add_alert(
    symbol: String,
    price: f64,
    state: State<'_, Arc<Mutex<AppState>>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let symbol = symbol.to_uppercase();
    let mut state_guard = state.lock().unwrap();
    let ticker_state = state_guard
        .tickers
        .entry(symbol.clone())
        .or_insert(TickerState {
            symbol: symbol.clone(),
            alerts: Vec::new(),
            last_price: 0.0,
        });

    if !ticker_state.alerts.iter().any(|alert| alert.price == price) {
        ticker_state.alerts.push(PriceAlert { price });
        println!("Added alert for {}: {}", symbol, price);

        // Save tickers after adding an alert
        drop(state_guard); // Release the lock before the async call
        if let Err(e) = block_on(save_app_settings(app, state)) {
            println!("Failed to save settings after adding alert: {}", e);
        }

        Ok(())
    } else {
        Err("Alert already exists".into())
    }
}

#[tauri::command]
fn remove_alert(
    symbol: String,
    price: f64,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let symbol = symbol.to_uppercase();
    let mut state = state.lock().unwrap();
    if let Some(ticker_state) = state.tickers.get_mut(&symbol) {
        ticker_state.alerts.retain(|alert| alert.price != price);
        println!("Removed alert for {}: {}", symbol, price);
        Ok(())
    } else {
        Err("Ticker not found".into())
    }
}

#[tauri::command]
async fn start_websocket(
    symbol: String,
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let symbol = symbol.to_uppercase();
    let (tx, mut rx): (Sender<(String, f64)>, Receiver<(String, f64)>) = mpsc::channel(100);
    let (stop_tx, mut stop_rx) = mpsc::channel(1);
    let state_clone = state.inner().clone();

    {
        let mut state = state_clone.lock().unwrap();
        state.websocket_senders.insert(symbol.clone(), stop_tx);
    }

    task::spawn(async move {
        let ws_url = format!(
            "wss://stream.binance.com:9443/ws/{}@ticker",
            symbol.to_lowercase()
        );
        let (ws_stream, _) = connect_async(&ws_url).await.map_err(|e| e.to_string())?;
        let (_, mut read) = ws_stream.split();

        loop {
            tokio::select! {
                message = read.next() => {
                    if let Some(Ok(msg)) = message {
                        if msg.is_text() {
                            if let Ok(ticker) = serde_json::from_str::<TickerStream>(&msg.to_text().unwrap()) {
                                if let Ok(price_f64) = ticker.price.parse::<f64>() {
                                    if tx.send((symbol.clone(), price_f64)).await.is_err() {
                                        break;
                                    }
                                }
                            }
                        }
                    } else {
                        break;
                    }
                }
                _ = stop_rx.recv() => {
                    println!("Stopping WebSocket for {}", symbol);
                    break;
                }
            }
        }
        Ok::<(), String>(())
    });

    task::spawn(async move {
        while let Some((symbol, price)) = rx.recv().await {
            println!("Received price update for {}: {}", symbol, price);

            if let Err(e) = app.emit(&format!("price-update-{}", symbol), price) {
                println!("Failed to emit price-update-{}: {}", symbol, e);
            }

            let mut state = state_clone.lock().unwrap();
            let ticker_state = state.tickers.entry(symbol.clone()).or_insert(TickerState {
                symbol: symbol.clone(),
                alerts: Vec::new(),
                last_price: price,
            });

            let last_price = ticker_state.last_price;
            let triggered_alerts: Vec<f64> = ticker_state
                .alerts
                .iter()
                .filter(|alert| {
                    (last_price < alert.price && price >= alert.price)
                        || (last_price > alert.price && price <= alert.price)
                })
                .map(|alert| alert.price)
                .collect();

            if !triggered_alerts.is_empty() {
                println!("Triggered alerts for {}: {:?}", symbol, triggered_alerts);
                if let Err(e) = app.emit(
                    &format!("alert-triggered-{}", symbol),
                    triggered_alerts.clone(),
                ) {
                    println!("Failed to emit alert-triggered-{}: {}", symbol, e);
                }
                ticker_state
                    .alerts
                    .retain(|alert| !triggered_alerts.contains(&alert.price));
            }

            ticker_state.last_price = price;
        }
    });

    Ok(())
}

#[tauri::command]
fn play_alert_sound(filename: String) -> Result<(), String> {
    println!("Attempting to play sound: {}", filename);

    let audio_data = match filename.as_str() {
        "beep.mp3" => include_bytes!("../assets/beep.mp3").to_vec(),
        "beep2_loud.mp3" => include_bytes!("../assets/beep2_loud.mp3").to_vec(),
        "beep3_vibish.mp3" => include_bytes!("../assets/beep3_vibish.mp3").to_vec(),
        "beep4_voice.mp3" => include_bytes!("../assets/beep4_voice.mp3").to_vec(),
        "beep5_reporter.mp3" => include_bytes!("../assets/beep5_reporter.mp3").to_vec(),
        "beep6_dingding.mp3" => include_bytes!("../assets/beep6_dingding.mp3").to_vec(),
        _ => {
            println!("Unknown sound file: {}, defaulting to beep.mp3", filename);
            include_bytes!("../assets/beep.mp3").to_vec()
        }
    };

    println!("Audio data loaded, size: {} bytes", audio_data.len());

    let filename_clone = filename.clone();

    std::thread::spawn(move || {
        println!("Audio playback thread started");
        match OutputStream::try_default() {
            Ok((_stream, stream_handle)) => {
                println!("OutputStream created successfully");
                match Sink::try_new(&stream_handle) {
                    Ok(sink) => {
                        println!("Sink created successfully");
                        let cursor = Cursor::new(audio_data);
                        match Decoder::new(cursor) {
                            Ok(source) => {
                                println!("Audio source decoded successfully");
                                sink.append(source);
                                sink.sleep_until_end();
                                println!("Played sound: {}", filename_clone);
                            }
                            Err(e) => {
                                println!("Failed to decode audio source: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("Failed to create sink: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("Failed to get default OutputStream: {}", e);
            }
        }
        println!("Audio playback thread finished");
    });

    Ok(())
}

#[tauri::command]
fn get_alerts(symbol: String, state: State<'_, Arc<Mutex<AppState>>>) -> Vec<PriceAlert> {
    let symbol = symbol.to_uppercase();
    let state = state.lock().unwrap();
    state
        .tickers
        .get(&symbol)
        .map(|ticker_state| ticker_state.alerts.clone())
        .unwrap_or_default()
}

#[tauri::command]
async fn get_app_settings(app: AppHandle) -> Result<AppSettings, String> {
    let app_dir = app.path().app_data_dir().unwrap();
    let settings_file = app_dir.join("settings.json");
    println!("Attempting to read settings from: {:?}", settings_file);

    match fs::read_to_string(&settings_file) {
        Ok(contents) => {
            println!("Successfully read settings file. Contents: {}", contents);
            let mut settings: AppSettings =
                serde_json::from_str(&contents).map_err(|e| e.to_string())?;
            
            // Ensure ticker_order contains all tickers
            let ticker_symbols: Vec<String> = settings.tickers.iter()
                .map(|t| t.symbol.clone())
                .collect();
            
            // If ticker_order is empty or doesn't match tickers, rebuild it
            if settings.ticker_order.len() != ticker_symbols.len() {
                settings.ticker_order = ticker_symbols;
            }
            
            println!("Parsed settings: {:?}", settings);
            Ok(settings)
        }
        Err(e) => {
            println!("Failed to read settings file: {}", e);
            Ok(AppSettings {
                tickers: vec![],
                selected_sound: "beep.mp3".to_string(),
                ticker_order: Vec::new(),
            })
        }
    }
}

#[tauri::command]
async fn save_app_settings(
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let app_dir = app.path().app_data_dir().unwrap();
    let settings_file = app_dir.join("settings.json");
    println!("Attempting to save settings to: {:?}", settings_file);

    let settings = {
        let state_guard = state.lock().unwrap();
        AppSettings {
            tickers: state_guard.tickers.values().cloned().collect(),
            selected_sound: state_guard.selected_sound.clone(),
            ticker_order: state_guard.ticker_order.clone(),
        }
    };

    println!("Settings to save: {:?}", settings);
    let settings_json = serde_json::to_string(&settings).map_err(|e| e.to_string())?;
    
    fs::create_dir_all(&app_dir).map_err(|e| e.to_string())?;
    fs::write(&settings_file, settings_json).map_err(|e| e.to_string())?;
    println!("Settings saved successfully");
    Ok(())
}

#[tauri::command]
async fn stop_websocket(
    symbol: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let mut state = state.lock().unwrap();
    state.tickers.remove(&symbol);
    println!("Stopped WebSocket for {}", symbol);
    Ok(())
}

#[tauri::command]
async fn set_alerts(
    symbol: String,
    alerts: Vec<PriceAlert>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let symbol = symbol.to_uppercase();
    let mut state = state.lock().unwrap();
    let ticker_state = state.tickers.entry(symbol.clone()).or_insert(TickerState {
        symbol: symbol.clone(),
        alerts: Vec::new(),
        last_price: 0.0,
    });
    ticker_state.alerts = alerts.clone(); // Clone alerts before assigning
    println!("Set alerts for {}: {:?}", symbol, alerts);
    Ok(())
}

#[tauri::command]
async fn remove_ticker(
    symbol: String,
    state: State<'_, Arc<Mutex<AppState>>>,
    app: AppHandle,
) -> Result<(), String> {
    let symbol = symbol.to_uppercase();
    let stop_sender = {
        let mut state_guard = state.lock().unwrap();
        state_guard.tickers.remove(&symbol);
        state_guard.websocket_senders.remove(&symbol)
    };

    if let Some(sender) = stop_sender {
        let _ = sender.send(()).await;
    }

    println!("Removed ticker: {}", symbol);
    save_app_settings(app, state).await?;
    println!("Settings saved after removal");
    Ok(())
}

#[tauri::command]
async fn set_last_price(
    symbol: String,
    price: f64,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let symbol = symbol.to_uppercase();
    let mut state = state.lock().unwrap();
    if let Some(ticker_state) = state.tickers.get_mut(&symbol) {
        ticker_state.last_price = price;
        println!("Set last price for {}: {}", symbol, price);
        Ok(())
    } else {
        Err(format!("Ticker not found: {}", symbol))
    }
}

#[tauri::command]
async fn set_selected_sound(
    sound: String,
    state: State<'_, Arc<Mutex<AppState>>>,
    app: AppHandle,
) -> Result<(), String> {
    {
        let mut state_guard = state.lock().unwrap();
        state_guard.selected_sound = sound;
    } // The MutexGuard is dropped here
    save_app_settings(app, state).await
}

#[tauri::command]
async fn update_ticker_order(
    order: Vec<String>,
    state: State<'_, Arc<Mutex<AppState>>>,
    app: AppHandle,
) -> Result<(), String> {
    {
        let mut state_guard = state.lock().unwrap();
        state_guard.ticker_order = order;
    }
    save_app_settings(app, state).await
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .manage(Arc::new(Mutex::new(AppState {
            tickers: HashMap::new(),
            selected_sound: "beep.mp3".to_string(),
            websocket_senders: HashMap::new(),
            ticker_order: Vec::new(),
        })))
        .setup(|app| {
            let app_handle = app.handle();
            let state = app.state::<Arc<Mutex<AppState>>>();

            // Load settings on startup
            tauri::async_runtime::block_on(async {
                match get_app_settings(app_handle.clone()).await {
                    Ok(settings) => {
                        let mut state_guard = state.lock().unwrap();
                        state_guard.selected_sound = settings.selected_sound.clone();
                        state_guard.ticker_order = settings.ticker_order.clone();
                        drop(state_guard);

                        for ticker in settings.tickers {
                            let symbol = ticker.symbol.clone();
                            if let Err(e) =
                                start_websocket(symbol.clone(), app_handle.clone(), state.clone())
                                    .await
                            {
                                println!("Failed to start WebSocket for ticker: {}", e);
                            }
                            if let Err(e) =
                                set_alerts(symbol.clone(), ticker.alerts, state.clone()).await
                            {
                                println!("Failed to set alerts for ticker {}: {}", symbol, e);
                            }
                            // Update the last_price in the state
                            let mut state_guard = state.lock().unwrap();
                            if let Some(ticker_state) = state_guard.tickers.get_mut(&symbol) {
                                ticker_state.last_price = ticker.last_price;
                            }
                        }
                    }
                    Err(e) => println!("Failed to get settings: {}", e),
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_websocket,
            stop_websocket,
            play_alert_sound,
            add_alert,
            remove_alert,
            get_alerts,
            get_app_settings,
            save_app_settings,
            set_alerts,
            remove_ticker,
            set_last_price,
            set_selected_sound,
            update_ticker_order
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}