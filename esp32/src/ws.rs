use chrono::NaiveTime;
use crossbeam::{
    channel::{Receiver, Sender},
    select,
};
use esp_idf_svc::{
    io::EspIOError,
    ws::{
        client::{
            EspWebSocketClient, EspWebSocketClientConfig, WebSocketEvent, WebSocketEventType,
        },
        FrameType,
    },
};
use esp_idf_sys::EspError;
use log::info;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::{BoardEvent, BoardInfo, ServerCommand};

pub struct WsModule {
    url: String,
    token: String,
    tx: Sender<BoardEvent>,
    rx: Receiver<WsCommand>,
}

impl WsModule {
    pub fn new(url: String, token: String, tx: Sender<BoardEvent>) -> (Self, Sender<WsCommand>) {
        let (module_tx, rx) = crossbeam::channel::unbounded::<WsCommand>();
        (WsModule { url, token, tx, rx }, module_tx)
    }

    /// Start the WebSocket client
    /// bg loop
    pub fn start(self) {
        let mut client = connect_ws_with_token(&self.url, &self.token, self.tx).unwrap();

        info!("WebSocket client connected");

        let mut boardinfo = BoardInfo::default();
        boardinfo.device_id = "demodemo".to_string();

        let data = serde_json::to_string(&boardinfo).unwrap();

        info!("WebSocket client sent data: {}", data);

        std::thread::spawn(move || loop {
            select! {
                recv(self.rx) -> msg => {
                    match msg {
                        Ok(cmd) => {
                            match cmd {
                                WsCommand::NewBoardInfo(new_info) => {
                                    let data = serde_json::to_string(&new_info).unwrap();
                                    if client.is_connected() {
                                        info!("Sending updated BoardInfo: {}", data);
                                    client
                                        .send(FrameType::Text(false), data.as_bytes())
                                        .unwrap();
                                    } else {
                                        info!("WebSocket client is not connected, cannot send BoardInfo");
                                    }
                                }
                                WsCommand::Connect => {
                                    // Optionally handle reconnect logic here
                                    info!("Received Connect command");
                                }
                            }
                        }
                        Err(_) => {
                            info!("Command channel disconnected, exiting thread");
                            break;
                        }
                    }
                }
                // default(Duration::from_secs(5)) => {
                //     // Periodic check
                //     if !client.is_connected() {
                //         info!("WebSocket client is not connected, attempting to reconnect");
                //     }
                // }
            }
        });
    }
}

fn connect_ws_with_token(
    url: &str,
    token: &str,
    tx: Sender<BoardEvent>,
) -> Result<EspWebSocketClient<'static>, EspError> {
    let headers = format!("auth_token: {}\r\n", token);

    // Connect websocket
    let config = EspWebSocketClientConfig {
        headers: Some(&headers),
        ..Default::default()
    };

    let timeout = Duration::from_secs(10);

    let client =
        EspWebSocketClient::new(url, &config, timeout, move |event| handle_event(&tx, event))
            .unwrap();

    Ok(client)
}

pub enum WsCommand {
    NewBoardInfo(BoardInfo),
    Connect,
}

fn handle_event(tx: &Sender<BoardEvent>, event: &Result<WebSocketEvent, EspIOError>) {
    if let Ok(event) = event {
        match event.event_type {
            WebSocketEventType::BeforeConnect => {
                info!("Websocket before connect");
            }
            WebSocketEventType::Connected => {
                info!("Websocket connected");
                tx.send(BoardEvent::WsStatusChanged { connected: true })
                    .ok();
            }
            WebSocketEventType::Disconnected => {
                info!("Websocket disconnected");
                tx.send(BoardEvent::WsStatusChanged { connected: false })
                    .ok();
            }
            WebSocketEventType::Close(reason) => {
                info!("Websocket close, reason: {reason:?}");
                tx.send(BoardEvent::WsStatusChanged { connected: false })
                    .ok();
            }
            WebSocketEventType::Closed => {
                info!("Websocket closed");
                tx.send(BoardEvent::WsStatusChanged { connected: false })
                    .ok();
            }
            WebSocketEventType::Text(text) => {
                info!("Websocket recv, text: {text}");
                let command: ServerCommand = serde_json::from_str(&text).unwrap();
                tx.send(BoardEvent::ServerCommandArrived { command: command })
                    .ok();
            }
            WebSocketEventType::Binary(binary) => {
                info!("Websocket recv, binary: {binary:?}");
            }
            WebSocketEventType::Ping => {
                info!("Websocket ping");
            }
            WebSocketEventType::Pong => {
                info!("Websocket pong");
            }
        }
    }
}
