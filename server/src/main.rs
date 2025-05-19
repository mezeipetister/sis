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
use std::time::Duration;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;
use ws::Message;

#[derive(Debug, Serialize, Clone)]
pub enum Command {
    SetNewSchedule(Schedule),
    Stop,
    StartZoneAction(ZoneAction),
    StartProgram(String),
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
    pub zones: Vec<String>,
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
    let mut ping_interval = tokio::time::interval(Duration::from_secs(2));
    let mut last_pong_time = std::time::Instant::now();

    ws.channel(move |mut stream| {
        Box::pin(async move {
            let mut device_id: Option<String> = None;
            loop {
                tokio::select! {
                    // Handle incoming WebSocket messages from client
                    msg = stream.next() => {
                        match msg {
                            Some(Ok(msg)) => {
                                match msg {
                                    ws::Message::Text(text) => {
                                        // Try to parse as BoardInfo
                                    if let Ok(board_info) = serde_json::from_str::<BoardInfo>(&text) {
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
                                    ws::Message::Binary(_) => {
                                        info!("Received binary message from client");
                                    }
                                    ws::Message::Ping(_) => {
                                        info!("Received ping from client");
                                    }
                                    ws::Message::Pong(_) => {
                                        info!("Received pong from client");
                                        last_pong_time = std::time::Instant::now();
                                    }
                                    _ => {}
                                }
                                // Echo or handle other messages as needed
                            }
                            Some(Err(e)) => {
                                info!("WebSocket error: {:?}", e);
                                break;
                            }
                            None => {
                                // Client disconnected
                                break;
                            }
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
                    _ = ping_interval.tick() => {
                        if last_pong_time.elapsed() > Duration::from_secs(5) {
                            info!("No pong received for 10s. Closing connection.");
                            break;
                        }
                        if let Err(e) = stream.send(ws::Message::Ping(vec![])).await {
                            info!("Ping failed: {:?}", e);
                            break;
                        } else {
                            info!("Ping sent");
                        }
                    }
                }
            }

            info!("WebSocket connection closed");

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
    let cmd = Command::Stop;
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
