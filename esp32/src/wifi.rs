use std::{future, thread, time::Duration};

use crossbeam::{
    channel::{Receiver, Sender},
    select,
};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    hal::task::block_on,
    timer::EspTaskTimerService,
    wifi::{AsyncWifi, AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi},
};
use log::{info, warn};

use crate::{BoardEvent, PASSWORD, SSID};

pub struct WifiModule {
    wifi: AsyncWifi<EspWifi<'static>>,
    inited: bool,
    tx: Sender<BoardEvent>,
    rx: Receiver<WifiCommand>,
    is_online: bool,
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
                is_online: false,
                inited: false,
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
        if !self.inited {
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
            self.inited = true;
        }

        self.wifi.connect().await?;
        info!("Wifi connected");

        self.wifi.wait_netif_up().await?;
        info!("Wifi netif up");

        Ok(())
    }

    pub fn start(mut self) {
        // Start the WiFi connection process
        block_on(self.connect_wifi());

        let mut connecting = false;

        thread::Builder::new()
            .name("schedule_module".into())
            .stack_size(8192) // vagy próbáld: 8192 vagy 16384
            .spawn(move || loop {
                select! {
                    recv(self.rx) -> msg => {
                        match msg {
                            Ok(WifiCommand::Connect) => {
                                info!("Received Wifi connect command");
                                if !connecting {
                                    connecting = true;
                                    let _ = block_on(self.connect_wifi());
                                    connecting = false;
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
                            warn!("WiFi disconnected!");
                            let _ = self.tx.send(BoardEvent::WifiStatusChanged { connected: false });
                            self.is_online = false;
                        } else {
                            info!("WiFi OK");
                            if !self.is_online {
                                self.is_online = true;
                                let _ = self.tx.send(BoardEvent::WifiStatusChanged { connected: true });
                                info!("WiFi connected!");
                            }
                        }
                    }
                }
            }).expect("Failed to spawn schedule thread");
    }
}

pub enum WifiCommand {
    Connect,
}
