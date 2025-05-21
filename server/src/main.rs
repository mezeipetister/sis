use log::info;
use mongodb::Collection;
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
#[serde(tag = "type")]
enum ClientCommand {
    StartProgram { program_id: String },
    StartZoneAction { zone_action: ZoneAction },
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Program {
    pub id: String,
    pub name: String,
    pub weekdays: Vec<u8>,
    pub active: bool,
    pub start_time: String,
    pub zones: Vec<ZoneAction>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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

    let client = state.mongo_client.clone();

    ws.channel(move |mut stream| {
        Box::pin(async move {
            let mut device_id: Option<String> = None;

            // Send the initial schedule to the client
            let collection = client
                .database("sis")
                .collection::<Schedule>("schedule");
            
            if let Ok(Some(latest_schedule)) = collection.find_one(doc! {}).await {
                let msg = ServerCommand::SetNewSchedule(latest_schedule);
                let json = serde_json::to_string(&msg).unwrap();
                stream.send(ws::Message::Text(json.into())).await?;
            }
            
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
                                        // Update the board's datetime and schedule_version in MongoDB if it exists
                                        let collection = client
                                            .database("sis")
                                            .collection::<BoardDetails>("boards");
                                        let filter = doc! { "device_id": &board_info.device_id };
                                        let update = doc! {
                                            "$set": {
                                                "datetime": &board_info.datetime,
                                                "schedule_version": board_info.schedule_version,
                                                "running_program": bson::to_bson(&board_info.running_program).unwrap_or(bson::Bson::Null),
                                                "running_zones": bson::to_bson(&board_info.running_zones).unwrap_or(bson::Bson::Null),
                                            }
                                        };
                                        let _ = collection
                                            .update_one(filter, update)
                                            .await
                                            .map_err(|e| info!("MongoDB update error: {:?}", e));
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
        ClientCommand::StartProgram { program_id } => ServerCommand::StartProgram(program_id),
        ClientCommand::StartZoneAction { zone_action } => {
            ServerCommand::StartZoneAction(zone_action)
        }
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
    // Try to update the board if it exists, otherwise insert
    let filter = doc! { "device_id": &details.device_id };
    let update = doc! {
        "$set": bson::to_bson(&details).map_err(|_| Status::InternalServerError)?,
    };

    collection
        .update_one(filter, update)
        .upsert(true)
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
        .delete_many(doc! { "device_id": &device_id })
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

#[get("/schedule")]
async fn get_schedule(state: &State<AppState>) -> Result<Json<Schedule>, Status> {
    let collection = state
        .mongo_client
        .database("sis")
        .collection::<Schedule>("schedule");
    let doc = collection
        .find_one(doc! {})
        .await
        .map_err(|_| Status::InternalServerError)?;
    match doc {
        Some(schedule) => Ok(Json(schedule)),
        None => Err(Status::NotFound),
    }
}

#[derive(Debug, Deserialize)]
struct ProgramInput {
    id: String,
    name: String,
    weekdays: Vec<u8>,
    active: bool,
    start_time: String,
    zones: Vec<ZoneAction>,
}

#[post("/schedule/program", data = "<program>")]
async fn set_program(
    state: &State<AppState>,
    program: Json<ProgramInput>,
) -> Result<Status, Status> {
    let collection = state
        .mongo_client
        .database("sis")
        .collection::<Schedule>("schedule");

    // Get current schedule
    let mut schedule = collection
        .find_one(doc! {})
        .await
        .map_err(|_| Status::InternalServerError)?
        .unwrap_or(Schedule {
            version: 0,
            programs: vec![],
        });

    // Find if program exists
    let idx = schedule.programs.iter().position(|p| p.id == program.id);

    let new_program = Program {
        id: program.id.clone(),
        name: program.name.clone(),
        weekdays: program.weekdays.clone(),
        active: program.active,
        start_time: program.start_time.clone(),
        zones: program.zones.clone(),
    };

    if let Some(i) = idx {
        schedule.programs[i] = new_program;
    } else {
        schedule.programs.push(new_program);
    }

    schedule.version += 1;

    // Upsert the schedule
    collection
        .update_one(
            doc! {},
            doc! { "$set": bson::to_bson(&schedule).map_err(|_| Status::InternalServerError)? },
        )
        .upsert(true)
        .await
        .map_err(|_| Status::InternalServerError)?;

    // Notify clients
    let _ = state
        .cmd_tx
        .send(ServerCommand::SetNewSchedule(schedule.clone()))
        .map_err(|_| Status::InternalServerError)?;

    let _ = state
        .cmd_tx
        .send(ServerCommand::SetNewSchedule(schedule.clone()))
        .map_err(|_| Status::InternalServerError)?;

    Ok(Status::Ok)
}

#[post("/schedule/program/<id>/enable")]
async fn enable_program(state: &State<AppState>, id: String) -> Result<Status, Status> {
    update_program_active(state, id, true).await
}

#[post("/schedule/program/<id>/disable")]
async fn disable_program(state: &State<AppState>, id: String) -> Result<Status, Status> {
    update_program_active(state, id, false).await
}

#[post("/schedule/program/<id>/remove")]
async fn remove_program(state: &State<AppState>, id: String) -> Result<Status, Status> {
    let collection = state
        .mongo_client
        .database("sis")
        .collection::<Schedule>("schedule");

    // Get current schedule
    let mut schedule = collection
        .find_one(doc! {})
        .await
        .map_err(|_| Status::InternalServerError)?
        .ok_or(Status::NotFound)?;

    // Remove the program by id
    schedule.programs.retain(|p| p.id != id);

    schedule.version += 1;

    // Upsert the schedule
    collection
        .update_one(
            doc! {},
            doc! { "$set": bson::to_bson(&schedule).map_err(|_| Status::InternalServerError)? },
        )
        .upsert(true)
        .await
        .map_err(|_| Status::InternalServerError)?;

    // Notify clients
    let _ = state
        .cmd_tx
        .send(ServerCommand::SetNewSchedule(schedule.clone()))
        .map_err(|_| Status::InternalServerError)?;

    let _ = state
        .cmd_tx
        .send(ServerCommand::SetNewSchedule(schedule.clone()))
        .map_err(|_| Status::InternalServerError)?;

    Ok(Status::Ok)
}

async fn update_program_active(
    state: &State<AppState>,
    id: String,
    active: bool,
) -> Result<Status, Status> {
    let collection = state
        .mongo_client
        .database("sis")
        .collection::<Schedule>("schedule");

    let mut schedule = collection
        .find_one(doc! {})
        .await
        .map_err(|_| Status::InternalServerError)?
        .ok_or(Status::NotFound)?;

    let mut found = false;
    for program in &mut schedule.programs {
        if program.id == id {
            program.active = active;
            found = true;
            break;
        }
    }
    if !found {
        return Err(Status::NotFound);
    }

    schedule.version += 1;

    collection
        .update_one(
            doc! {},
            doc! { "$set": bson::to_bson(&schedule).map_err(|_| Status::InternalServerError)? },
        )
        .upsert(true)
        .await
        .map_err(|_| Status::InternalServerError)?;

    let _ = state
        .cmd_tx
        .send(ServerCommand::SetNewSchedule(schedule.clone()))
        .map_err(|_| Status::InternalServerError)?;

    Ok(Status::Ok)
}

#[rocket::main]
async fn main() {
    // Initialize the command channel
    env_logger::init();

    info!("Starting server...");

    let (cmd_tx, cmd_rx) = broadcast::channel(32);

    let mongo_client = mongodb::Client::with_uri_str("mongodb://root:example@mongo:27017")
        .await
        .expect("Failed to initialize MongoDB client");

    let schedule_collection = mongo_client
        .database("sis")
        .collection::<Schedule>("schedule");

    // Check if a schedule exists; if not, insert a default one
    if schedule_collection
        .count_documents(doc! {})
        .await
        .expect("Failed to count schedule documents")
        == 0
    {
        let default_schedule = Schedule {
            version: 1,
            programs: vec![],
        };
        schedule_collection
            .insert_one(default_schedule)
            .await
            .expect("Failed to insert default schedule");
    }

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
                get_schedule,
                set_program,
                enable_program,
                disable_program,
                remove_program,
            ],
        )
        .launch()
        .await
        .unwrap();
}
