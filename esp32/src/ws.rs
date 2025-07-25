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
        let mut client = connect_ws_with_token(&self.url, &self.token, self.tx.clone()).unwrap();

        // Message buffer
        let mut buffer: Vec<BoardInfo> = Vec::new();

        // Connection state
        let mut connected = false;

        info!("WebSocket client connected");

        std::thread::spawn(move || loop {
            select! {
                recv(self.rx) -> msg => {
                    match msg {
                        Ok(cmd) => {
                            match cmd {
                                WsCommand::NewBoardInfo(new_info) => {
                                    if let Ok(data) = serde_json::to_string(&new_info) {
                                        if client.is_connected() && connected {
                                            info!("Sending updated BoardInfo: {}", data);
                                            if let Ok(()) = client.send(FrameType::Text(false), data.as_bytes()) {
                                                info!("BoardInfo sent successfully");
                                            } else {
                                                buffer.push(new_info);
                                                info!("Failed to send BoardInfo, WebSocket client is not connected");
                                            }
                                        } else {
                                            info!("WebSocket client is not connected, cannot send BoardInfo");
                                            buffer.push(new_info);
                                        }
                                    } else {
                                        info!("Failed to serialize BoardInfo to JSON");
                                    }
                                }
                                WsCommand::Connect => {
                                    // Optionally handle reconnect logic here
                                    info!("Received Connect command");
                                    if !client.is_connected() {
                                        info!("WebSocket client is not connected, attempting to reconnect");
                                        client = connect_ws_with_token(&self.url, &self.token, self.tx.clone()).unwrap();
                                        info!("WebSocket client reconnected");
                                    } else {
                                        info!("WebSocket client is already connected");
                                    }
                                }
                                WsCommand::Connected => {
                                    connected = true;
                                    if !buffer.is_empty() {
                                        let drained: Vec<_> = buffer.drain(..).collect();
                                        for info in drained {
                                            if let Ok(data) = serde_json::to_string(&info) {
                                                if let Ok(()) = client.send(FrameType::Text(false), data.as_bytes()) {
                                                    info!("Buffered BoardInfo sent successfully");
                                                } else {
                                                    buffer.push(info);
                                                    info!("Failed to send buffered BoardInfo, still not connected");
                                                }
                                            } else {
                                                info!("Failed to serialize buffered BoardInfo to JSON");
                                            }
                                        }
                                    }
                                    info!("WebSocket client is connected");
                                }
                                WsCommand::Disconnected => {
                                    connected = false;
                                    info!("WebSocket client is disconnected");
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
    Connected,
    Disconnected,
}

fn handle_event(tx: &Sender<BoardEvent>, event: &Result<WebSocketEvent, EspIOError>) {
    use std::sync::OnceLock;
    static TEXT_BUFFER: OnceLock<std::sync::Mutex<String>> = OnceLock::new();

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
            WebSocketEventType::Text(chunk) => {
                let buffer = TEXT_BUFFER.get_or_init(|| std::sync::Mutex::new(String::new()));
                let mut buf = buffer.lock().unwrap();
                buf.push_str(&chunk);

                // Próbáljuk meg parse-olni
                match serde_json::from_str::<ServerCommand>(&buf) {
                    Ok(command) => {
                        tx.send(BoardEvent::ServerCommandArrived { command }).ok();
                        buf.clear(); // sikeres parse után ürítsd a buffert
                    }
                    Err(e) => {
                        if e.is_eof() {
                            // További darabokat várunk
                            info!("Partial WebSocket message received, waiting for more...");
                        } else {
                            // Valódi hiba: logoljuk és ürítjük a buffert
                            log::error!("WebSocket JSON parse error: {e}, dropping buffer");
                            buf.clear();
                        }
                    }
                }
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
