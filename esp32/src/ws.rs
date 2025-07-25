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
use log::info;
use std::thread;
use std::time::Duration;

use crate::{BoardEvent, BoardInfo, ServerCommand};

pub struct WsModule {
    url: String,
    token: String,
    tx: Sender<BoardEvent>,
    rx: Receiver<WsCommand>,
    client: Option<EspWebSocketClient<'static>>,
    connecting: bool,
}

impl WsModule {
    pub fn new(url: String, token: String, tx: Sender<BoardEvent>) -> (Self, Sender<WsCommand>) {
        let (module_tx, rx) = crossbeam::channel::unbounded::<WsCommand>();
        (
            WsModule {
                url,
                token,
                tx,
                rx,
                client: None,
                connecting: false,
            },
            module_tx,
        )
    }

    /// Start the WebSocket client
    /// bg loop
    pub fn start(mut self) {
        // Try to connect to the WebSocket server
        // let _ = self.connect_ws_with_token();

        // Message buffer
        let mut buffer: Vec<BoardInfo> = Vec::new();

        info!("WebSocket client connected");

        thread::Builder::new()
            .name("schedule_module".into())
            .stack_size(8192)
            .spawn(move || loop {
            select! {
                recv(self.rx) -> msg => {
                    match msg {
                        Ok(cmd) => {
                            match cmd {
                                WsCommand::NewBoardInfo(new_info) => {
                                    if let Ok(data) = serde_json::to_string(&new_info) {
                                        if let Some(client) = &mut self.client {
                                            if client.is_connected() {
                                                if let Ok(()) = client.send(FrameType::Text(false), data.as_bytes()) {
                                                    info!("BoardInfo sent successfully");
                                                } else {
                                                    buffer.push(new_info);
                                                    info!("Failed to send BoardInfo, buffering it");
                                                }
                                            } else {
                                                buffer.push(new_info);
                                                info!("WebSocket client is not connected, buffering BoardInfo");
                                            }
                                        } else {
                                            buffer.push(new_info);
                                            info!("WebSocket client is None, buffering BoardInfo");
                                        }
                                    } else {
                                        info!("Failed to serialize BoardInfo to JSON");
                                    }
                                }
                                WsCommand::Connect => {
                                    // Optionally handle reconnect logic here
                                    info!("Received Connect command");
                                    info!("WebSocket client is not connected, attempting to reconnect");
                                    let _ = self.connect_ws_with_token();
                                }
                                WsCommand::Connected => {
                                    if !buffer.is_empty() {
                                        let drained: Vec<_> = buffer.drain(..).collect();
                                        for info in drained {
                                            if let Ok(data) = serde_json::to_string(&info) {
                                                if let Some(client) = &mut self.client {
                                                    if client.is_connected() {
                                                        if let Ok(()) = client.send(FrameType::Text(false), data.as_bytes()) {
                                                            info!("Buffered BoardInfo sent successfully");
                                                        } else {
                                                            info!("Failed to send buffered BoardInfo, will retry later");
                                                        }
                                                    } else {
                                                        info!("WebSocket client is not connected, will retry sending buffered BoardInfo later");
                                                        buffer.push(info); // Re-buffer if not connected
                                                    }
                                                } else {
                                                    info!("WebSocket client is None, cannot send buffered BoardInfo");
                                                }
                                            } else {
                                                info!("Failed to serialize buffered BoardInfo to JSON");
                                            }
                                        }
                                    }
                                    info!("WebSocket client is connected");
                                }
                                WsCommand::Disconnected => {
                                    self.client = None;
                                    self.connecting = false;
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
                default(Duration::from_secs(3)) => {
                    if let Some(client) = &mut self.client {
                        if !client.is_connected() && !self.connecting {
                            let _ = self.tx.send(BoardEvent::WsStatusChanged { connected: false });
                        }
                    } else if !self.connecting {
                        let _ = self.tx.send(BoardEvent::WsStatusChanged { connected: false });
                    }
                }
            }
        })
            .expect("Failed to spawn schedule thread");
    }

    fn connect_ws_with_token(&mut self) -> Result<(), EspIOError> {
        if self.connecting {
            info!("WebSocket client is already connecting, skipping new connection attempt");
            return Ok(());
        }

        self.client = None; // Reset client before connecting

        self.connecting = true;

        let headers = format!("auth_token: {}\r\n", &self.token);

        // Connect websocket
        let config = EspWebSocketClientConfig {
            headers: Some(&headers),
            ..Default::default()
        };

        let timeout = Duration::from_secs(10);

        let tx_clone = self.tx.clone();

        let client = EspWebSocketClient::new(&self.url, &config, timeout, move |event| {
            handle_event(&tx_clone, event)
        });

        match client {
            Ok(client) => {
                info!("WebSocket client connected successfully");
                self.client = Some(client);
                self.connecting = false;
                let _ = self
                    .tx
                    .send(BoardEvent::WsStatusChanged { connected: true });
                Ok(())
            }
            Err(e) => {
                info!("Failed to connect WebSocket client: {}", e);
                self.connecting = false;
                self.client = None;
                let _ = self
                    .tx
                    .send(BoardEvent::WsStatusChanged { connected: false });
                Err(e)
            }
        }
    }
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
                let _ = tx.send(BoardEvent::WsStatusChanged { connected: true });
            }
            WebSocketEventType::Disconnected => {
                info!("Websocket disconnected");
                let _ = tx.send(BoardEvent::WsStatusChanged { connected: false });
            }
            WebSocketEventType::Close(reason) => {
                info!("Websocket close, reason: {reason:?}");
                let _ = tx.send(BoardEvent::WsStatusChanged { connected: false });
            }
            WebSocketEventType::Closed => {
                info!("Websocket closed");
                let _ = tx.send(BoardEvent::WsStatusChanged { connected: false });
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
