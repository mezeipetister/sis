use log::info;
use mongodb::bson::{self, doc};
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::response::stream::EventStream;
use rocket::serde::json::Json;
use rocket::tokio::stream;
use rocket::tokio::sync::Mutex;
use rocket::tokio::sync::broadcast::{self, Receiver, Sender};
use rocket::{State, post};
use rocket::{get, routes};
use rocket_ws as ws;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;
use ws::Message;

#[derive(Debug, Serialize, Clone)]
pub enum ServerCommand {
    SetNewSchedule(Schedule),
    Stop,
    StartZoneAction(ZoneAction),
    StartProgram(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
enum ClientCommand {
    StartProgram(String),
    StartZoneAction(ZoneAction),
    Stop,
}

struct AppState {
    cmd_tx: Sender<ServerCommand>,
    cmd_rx: Receiver<ServerCommand>,
    online_devices: Arc<Mutex<Vec<BoardInfo>>>,
    mongo_client: mongodb::Client,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ZoneInfo {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BoardDetails {
    pub device_id: String,
    pub name: String,
    pub datetime: String,
    pub schedule_version: u32,
    pub running_program: Option<String>,
    pub running_zones: Option<ZoneAction>,
    pub zones: Vec<ZoneInfo>,
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
    use mongodb::bson::{doc, to_document};
    use mongodb::options::FindOptions;
    use rocket::form::Form;
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

#[get("/online_devices")]
async fn online_devices_handler(state: &State<AppState>) -> Json<Vec<BoardInfo>> {
    let devices = state.online_devices.lock().await;
    Json(devices.clone())
}

// Run client commands
#[post("/run_command", data = "<cmd>")]
async fn run_command_handler(
    state: &State<AppState>,
    cmd: Json<ClientCommand>,
) -> Result<(), Status> {
    // Convert the incoming command to ServerCommand
    let cmd = match cmd.into_inner() {
        ClientCommand::StartProgram(program_id) => ServerCommand::StartProgram(program_id),
        ClientCommand::StartZoneAction(zone_action) => ServerCommand::StartZoneAction(zone_action),
        ClientCommand::Stop => ServerCommand::Stop,
    };
    // Send the command to the command channel
    state
        .cmd_tx
        .send(cmd)
        .map_err(|_| Status::InternalServerError)?;
    info!("Command sent");
    Ok(())
}

// List all boards from MongoDB
#[get("/devices")]
async fn list_devices(state: &State<AppState>) -> Result<Json<Vec<BoardDetails>>, Status> {
    // Fetch all boards from MongoDB
    let collection = state
        .mongo_client
        .database("sis")
        .collection::<BoardDetails>("boards");
    // Use a cursor to iterate over the documents
    let mut cursor = collection
        .find(doc! {})
        .await
        .map_err(|_| Status::InternalServerError)?;
    let mut boards = Vec::new();
    while let Some(board) = cursor.next().await {
        boards.push(board.map_err(|_| Status::InternalServerError)?);
    }
    Ok(Json(boards))
}

// Add a board by id (copying BoardInfo from online_devices)
#[post("/boards/add/<device_id>")]
async fn add_board(state: &State<AppState>, device_id: String) -> Result<Status, Status> {
    let devices = state.online_devices.lock().await;
    let board_info = devices.iter().find(|b| b.device_id == device_id).cloned();
    drop(devices);

    let Some(info) = board_info else {
        return Err(Status::NotFound);
    };

    // Create a new BoardDetails object
    let details = BoardDetails {
        device_id: info.device_id.clone(),
        name: "".to_string(),
        datetime: info.datetime,
        schedule_version: info.schedule_version,
        running_program: info.running_program,
        running_zones: info.running_zones,
        zones: info
            .zones
            .into_iter()
            .map(|id| ZoneInfo {
                id: id.clone(),
                name: "".to_string(),
            })
            .collect(),
    };

    // Insert the board details into MongoDB
    let collection = state
        .mongo_client
        .database("sis")
        .collection::<BoardDetails>("boards");

    // Check if the board already exists
    collection
        .insert_one(details)
        .await
        .map_err(|_| Status::InternalServerError)?;

    Ok(Status::Created)
}

// Remove a board by id
#[post("/boards/remove/<device_id>")]
async fn remove_board(state: &State<AppState>, device_id: String) -> Result<(), Status> {
    // Remove the board from MongoDB
    let collection = state
        .mongo_client
        .database("sis")
        .collection::<BoardDetails>("boards");
    // Check if the board exists
    let res = collection
        .delete_one(doc! { "device_id": &device_id })
        .await
        .map_err(|_| Status::InternalServerError)?;

    Ok(())
}

// Set board name and zone names
#[derive(Debug, Deserialize)]
struct BoardMeta {
    name: String,
    zones: Vec<ZoneInfo>,
}

#[post("/boards/update/<device_id>", data = "<update>")]
async fn update_board(
    state: &State<AppState>,
    device_id: String,
    update: Json<BoardMeta>,
) -> Result<Status, Status> {
    // Update the board in MongoDB
    let collection = state
        .mongo_client
        .database("sis")
        .collection::<BoardDetails>("boards");
    // Check if the board exists
    let update_doc = doc! {
        "$set": {
            "name": &update.name,
            "zones": bson::to_bson(&update.zones).map_err(|_| Status::BadRequest)?,
        }
    };
    let res = collection
        .update_one(doc! { "device_id": &device_id }, update_doc)
        .await
        .map_err(|_| Status::InternalServerError)?;
    if res.matched_count == 0 {
        Err(Status::NotFound)
    } else {
        Ok(Status::Ok)
    }
}

#[rocket::main]
async fn main() {
    // Initialize the command channel
    env_logger::init();

    info!("Starting server...");

    let (cmd_tx, cmd_rx) = broadcast::channel(32);

    let mongo_client = mongodb::Client::with_uri_str("mongodb://mongo:27017")
        .await
        .expect("Failed to initialize MongoDB client");

    let state = AppState {
        cmd_tx,
        cmd_rx,
        online_devices: Arc::new(Mutex::new(Vec::new())),
        mongo_client,
    };

    rocket::build()
        .manage(state)
        .mount(
            "/",
            routes![
                websocket_handler,
                run_command_handler,
                online_devices_handler,
                list_devices,
                add_board,
                remove_board,
                update_board,
            ],
        )
        .launch()
        .await
        .unwrap();
}
