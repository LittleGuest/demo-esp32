#![feature(lazy_cell)]

use std::{
    sync::{Arc, LazyLock, Mutex},
    time::Duration,
};

use rumqttc::{AsyncClient, MqttOptions, QoS};
use serde::{Deserialize, Serialize};

const TOPIC_TEMP_HUMI: &str = "testtopic/pjq/dht11";

/// temperature and humidity
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct TempHumi {
    #[serde(rename = "t")]
    temperature: f32,
    #[serde(rename = "h")]
    humidity: f32,
}

static STORE: LazyLock<Arc<Mutex<Vec<TempHumi>>>> =
    LazyLock::new(|| Arc::new(Mutex::new(Vec::new())));

#[tauri::command]
fn temp_humi() -> Vec<TempHumi> {
    STORE.lock().unwrap().clone()
}

async fn connect() {
    let mut mqttoptions = MqttOptions::new("rumqtt-async-mqtt", "broker.emqx.io", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
    client
        .subscribe(TOPIC_TEMP_HUMI, QoS::AtLeastOnce)
        .await
        .unwrap();

    while let Ok(notification) = eventloop.poll().await {
        println!("Received = {:?}", notification);
        let th = TempHumi {
            temperature: 14.9,
            humidity: 0.0,
        };
        STORE.lock().unwrap().push(th);
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub async fn run() {
    tokio::task::spawn(connect());

    tauri::Builder::default()
        .plugin(tauri_plugin_websocket::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![temp_humi])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
