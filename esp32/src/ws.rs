use crossbeam::channel::Sender;
use esp_idf_svc::{
    io::EspIOError,
    ws::client::{
        EspWebSocketClient, EspWebSocketClientConfig, WebSocketEvent, WebSocketEventType,
    },
};
use esp_idf_sys::EspError;
use log::info;
use std::time::Duration;

pub struct WsModule {
    url: String,
    token: String,
}

impl WsModule {
    pub fn new(url: String, token: String) -> Self {
        WsModule { url, token }
    }

    pub fn start(self) {
        let client = connect_ws_with_token(&self.url, &self.token).unwrap();

        std::thread::spawn(move || loop {
            match client.is_connected() {
                true => {
                    info!("WebSocket client is connected");
                }
                false => {
                    info!("WebSocket client is not connected");
                }
            }
            std::thread::sleep(Duration::from_secs(5));
        });
    }
}

fn connect_ws_with_token(url: &str, token: &str) -> Result<EspWebSocketClient<'static>, EspError> {
    let headers = format!("Authorization: Bearer {}\r\n", token);

    // Connect websocket
    let config = EspWebSocketClientConfig {
        headers: Some(&headers),
        ..Default::default()
    };

    let timeout = Duration::from_secs(10);
    let (tx, rx) = crossbeam::channel::unbounded::<ExampleEvent>();

    let mut client =
        EspWebSocketClient::new(url, &config, timeout, move |event| handle_event(&tx, event))
            .unwrap();

    Ok(client)
}

enum ExampleEvent {
    Connected,
    Closed,
    MessageReceived,
}

fn handle_event(tx: &Sender<ExampleEvent>, event: &Result<WebSocketEvent, EspIOError>) {
    if let Ok(event) = event {
        match event.event_type {
            WebSocketEventType::BeforeConnect => {
                info!("Websocket before connect");
            }
            WebSocketEventType::Connected => {
                info!("Websocket connected");
                tx.send(ExampleEvent::Connected).ok();
            }
            WebSocketEventType::Disconnected => {
                info!("Websocket disconnected");
            }
            WebSocketEventType::Close(reason) => {
                info!("Websocket close, reason: {reason:?}");
            }
            WebSocketEventType::Closed => {
                info!("Websocket closed");
                tx.send(ExampleEvent::Closed).ok();
            }
            WebSocketEventType::Text(text) => {
                info!("Websocket recv, text: {text}");
                if text == "Hello, World!" {
                    tx.send(ExampleEvent::MessageReceived).ok();
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
