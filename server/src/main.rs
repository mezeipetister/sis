use log::info;
use rocket::State;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::response::stream::EventStream;
use rocket::serde::json::Json;
use rocket::tokio::stream;
use rocket::tokio::sync::Mutex;
use rocket::tokio::sync::broadcast::{self, Receiver, Sender};
use rocket::{get, routes};
use rocket_ws as ws;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;
use ws::Message;

#[derive(Debug, Serialize, Clone)]
pub enum Command {
    NewProgram(String),
    StopProgram,
    ZoneTest {
        zone_id: u8,
        duration_secs: u32,
    },
    WifiStatusChanged {
        connected: bool,
        ssid: Option<String>,
    },
    TimeUpdated {
        utc_timestamp: i64,
    },
    Error(String),
    StatusRequest,
    Shutdown,
}

struct AppState {
    cmd_tx: Sender<Command>,
    cmd_rx: Receiver<Command>,
    online_devices: Arc<Mutex<Vec<BoardInfo>>>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BoardInfo {
    pub device_id: String,
    pub datetime: String,
    pub schedule_version: u32,
    pub running_program: Option<String>,
    pub running_zones: Option<ZoneAction>,
}

#[derive(Debug, Serialize, Clone, Deserialize)]
pub struct ZoneAction {
    pub zone_ids: Vec<String>,
    pub duration_seconds: u32,
}

#[derive(Debug, Serialize, Clone)]
pub struct Program {
    pub id: String,
    pub name: String,
    pub weekdays: Vec<u8>,
    pub start_time: String,
    pub zones: Vec<ZoneAction>,
}

#[derive(Debug, Serialize, Clone)]
pub struct Schedule {
    pub version: u32,
    pub programs: Vec<Program>,
}

struct AuthToken;

#[get("/websocket")]
async fn websocket_handler(
    mut ws: rocket_ws::WebSocket,
    state: &State<AppState>,
) -> ws::Channel<'static> {
    use rocket::futures::SinkExt;

    // Clone only the necessary Arc/Mutex for static lifetime
    let online_devices = state.online_devices.clone();
    let rx = state.cmd_tx.subscribe();
    let mut cmd_stream = BroadcastStream::new(rx);

    ws.channel(move |mut stream| {
        Box::pin(async move {
            let mut device_id: Option<String> = None;
            while let Some(message) = tokio_stream::StreamExt::next(&mut stream).await {
                tokio::select! {
                    // Handle incoming WebSocket messages from client
                    msg = stream.next() => {
                        if let Some(msg) = msg {
                            let msg = msg?;
                            if msg.is_text() {
                                // Try to parse as BoardInfo
                                if let Ok(board_info) = serde_json::from_str::<BoardInfo>(msg.to_text()?) {
                                    info!("Received BoardInfo: {:?}", board_info);
                                    let mut devices = online_devices.lock().await;
                                    // Replace or insert BoardInfo by device_id
                                    if let Some(existing) = devices.iter_mut().find(|b| b.device_id == board_info.device_id) {
                                        *existing = board_info.clone();
                                    } else {
                                        devices.push(board_info.clone());
                                    }
                                    device_id = Some(board_info.device_id.clone());
                                }
                            }
                            // Echo or handle other messages as needed
                        } else {
                            break;
                        }
                    }
                    // Handle commands from server to client
                    cmd = cmd_stream.next() => {
                        if let Some(Ok(cmd)) = cmd {
                            let json = serde_json::to_string(&cmd).unwrap();
                            stream.send(ws::Message::Text(json.into())).await?;
                            info!("Sent command to client: {:?}", cmd);
                        }
                    }
                }
            }

            // Remove device from online_devices on disconnect
            if let Some(id) = device_id {
                let mut devices = online_devices.lock().await;
                devices.retain(|b| b.device_id != id);
                info!("Device {} removed from online devices", id);
            }

            Ok(())
        })
    })
}

// Create a simple endpoint /stop
// that sends a stop command to the command channel
#[get("/stop")]
async fn stop_handler(state: &State<AppState>) -> Result<(), Status> {
    let cmd = Command::StopProgram;
    state
        .cmd_tx
        .send(cmd)
        .map_err(|_| Status::InternalServerError)?;
    info!("Stop command sent");
    Ok(())
}

#[get("/online_devices")]
async fn online_devices_handler(state: &State<AppState>) -> Json<Vec<BoardInfo>> {
    let devices = state.online_devices.lock().await;
    Json(devices.clone())
}

#[rocket::main]
async fn main() {
    // Initialize the command channel
    env_logger::init();

    info!("Starting server...");

    let (cmd_tx, cmd_rx) = broadcast::channel(32);

    let state = AppState {
        cmd_tx,
        cmd_rx,
        online_devices: Arc::new(Mutex::new(Vec::new())),
    };

    rocket::build()
        .manage(state)
        .mount(
            "/",
            routes![websocket_handler, stop_handler, online_devices_handler],
        )
        .launch()
        .await
        .unwrap();
}
