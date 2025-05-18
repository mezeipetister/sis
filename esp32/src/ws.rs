use crossbeam::channel::Sender;
use esp_idf_svc::{
    io::EspIOError,
    ws::client::{
        EspWebSocketClient, EspWebSocketClientConfig, FrameType, WebSocketEvent, WebSocketEventType,
    },
};
use esp_idf_sys::EspError;
use log::info;
use std::time::Duration;

fn connect_ws_with_token<'a>(
    url: &'a str,
    token: &'a str,
) -> Result<EspWebSocketClient<'a>, EspError> {
    let headers = format!("Authorization: Bearer {}\r\n", token);

    let headers = format!("Authorization: Bearer {}\r\n", token);

    // Connect websocket
    let config = EspWebSocketClientConfig {
        headers: Some(&headers),
        ..Default::default()
    };

    const ECHO_SERVER_URI: &str = "wss://echo.websocket.org";

    let timeout = Duration::from_secs(10);
    let (tx, rx) = crossbeam::channel::unbounded::<ExampleEvent>();

    let mut client = EspWebSocketClient::new(ECHO_SERVER_URI, &config, timeout, move |event| {
        handle_event(&tx, event)
    })
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
