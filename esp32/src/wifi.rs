use std::{thread, time::Duration};

use crossbeam::{
    channel::{Receiver, Sender},
    select,
};
use esp_idf_svc::{
    hal::task::block_on,
    wifi::{AsyncWifi, AuthMethod, ClientConfiguration, Configuration, EspWifi},
};
use log::{info, warn};

use crate::{BoardEvent, PASSWORD, SSID};

pub struct WifiModule {
    wifi: AsyncWifi<EspWifi<'static>>,
    tx: Sender<BoardEvent>,
    rx: Receiver<WifiCommand>,
    connected: bool,
    connecting: bool,
}

impl WifiModule {
    pub fn new(
        wifi: AsyncWifi<EspWifi<'static>>,
        tx: Sender<BoardEvent>,
    ) -> anyhow::Result<(Self, Sender<WifiCommand>)> {
        // Create a new WifiModule instance
        let (module_tx, rx) = crossbeam::channel::unbounded::<WifiCommand>();

        Ok((
            Self {
                wifi,
                tx,
                rx,
                connected: false,
                connecting: false,
            },
            module_tx,
        ))
    }

    // Connect to WiFi
    // This function is used to connect to a WiFi network.
    // It takes a mutable reference to a BlockingWifi instance
    // and returns a Result indicating success or failure.
    // The function creates a Configuration object with the SSID and password,
    // sets the configuration on the wifi instance, starts the wifi,
    // and waits for the network interface to be up.
    async fn connect_wifi(&mut self) -> anyhow::Result<()> {
        info!("Connecting to WiFi...");

        self.connecting = true;
        self.connected = false;

        // Disconnect if already connected
        if let Ok(is_connected) = self.wifi.is_connected() {
            if is_connected {
                info!("Already connected to WiFi. Disconnecting...");
                self.wifi.disconnect().await?;
                self.wifi.stop().await?;
                return Ok(());
            }
        }

        info!("Connecting to WiFi...");
        let wifi_configuration: Configuration = Configuration::Client(ClientConfiguration {
            ssid: SSID.try_into().unwrap(),
            bssid: None,
            auth_method: AuthMethod::WPA2Personal,
            password: PASSWORD.try_into().unwrap(),
            channel: None,
            ..Default::default()
        });

        self.wifi.set_configuration(&wifi_configuration)?;

        self.wifi.start().await?;
        info!("Wifi started");

        self.wifi.connect().await?;
        info!("Wifi connected");

        self.connecting = false;

        self.wifi.wait_netif_up().await?;
        info!("Wifi netif up");

        self.connected = true;

        Ok(())
    }

    pub fn start(mut self) {
        thread::Builder::new()
            .name("schedule_module".into())
            .stack_size(8192) // vagy próbáld: 8192 vagy 16384
            .spawn(move || {
                self.run();
            })
            .expect("Failed to spawn schedule thread");
    }

    fn run(mut self) {
        loop {
            select! {
                recv(self.rx) -> msg => {
                    match msg {
                        Ok(WifiCommand::Connect) => {
                            info!("Received Wifi connect command");
                            if !self.connecting {
                                let _ = block_on(self.connect_wifi());
                                // Ensure we reset the connecting state in case of error
                                self.connecting = false;
                            }
                        }
                        Err(_) => {
                            warn!("Wifi command channel closed");
                            break;
                        }
                    }
                }

                default(Duration::from_secs(5)) => {
                    // Periodikus ellenőrzés
                    let is_connected = self.wifi.is_connected().unwrap_or(false);
                    if !is_connected {
                        if !self.connecting {
                            warn!("WiFi disconnected!");
                            self.connected = false;
                            let _ = self.tx.send(BoardEvent::WifiStatusChanged { connected: false });
                        }
                    } else {
                        info!("WiFi OK");
                        if !self.connected {
                            self.connected = true;
                            let _ = self.tx.send(BoardEvent::WifiStatusChanged { connected: true });
                            info!("WiFi connected!");
                        }
                    }
                }
            }
        }
    }
}

pub enum WifiCommand {
    Connect,
}
