use std::{
    net::{SocketAddr, TcpListener},
    ops::Deref,
    sync::Arc,
};

use axum::{response::IntoResponse, routing::get, Extension, Json, Router};

use log::{debug, info};

use tokio::sync::{broadcast::Receiver, mpsc::Sender};

use zbus::{dbus_interface, ConnectionBuilder};

use crate::{capture::Desktop, input::InputManagerEvent};

use super::Result;

#[derive(Debug)]
pub struct ApiManager {
    pub port: u16,
    ds_rx: Receiver<()>,
    stream: Option<TcpListener>,
    event_notifier: Arc<Sender<InputManagerEvent>>,
}

impl ApiManager {
    pub async fn new(
        ds_rx: Receiver<()>,
        event_notifier: Sender<InputManagerEvent>,
    ) -> Result<Self> {
        let stream = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))?;
        let port = stream.local_addr()?.port();

        info!("Bound to port {}", port);

        let api = Self {
            port,
            ds_rx,
            stream: Some(stream),
            event_notifier: Arc::new(event_notifier),
        };
        let api_announcer = ApiManagerAnnouncer { port };

        let _api_announcer_server = ConnectionBuilder::session()?
            .name("com.github.jess4tech.rdesktopd")?
            .serve_at("/com/github/jess4tech/rdesktopd", api_announcer)?
            .build()
            .await?;

        info!("Started DBus Service");

        Ok(api)
    }

    pub async fn run(&mut self, desktops: Vec<Desktop>) -> Result<()> {
        let router = Router::new()
            .route("/desktops", get(get_desktops))
            .layer(Extension(self.event_notifier.clone()))
            .layer(Extension(Arc::new(desktops)));
        let server = axum::Server::from_tcp(self.stream.take().unwrap())?
            .serve(router.into_make_service())
            .with_graceful_shutdown(async {
                self.ds_rx
                    .recv()
                    .await
                    .expect("Failed to receive shutdown signal");
            });

        info!("Starting server");

        server.await.map_err(|e| e.into())
    }
}

#[derive(Debug)]
pub struct ApiManagerAnnouncer {
    pub port: u16,
}

#[dbus_interface(name = "com.github.jess4tech.rdesktopdimpl")]
impl ApiManagerAnnouncer {
    fn get_address(&self) -> u16 {
        debug!("Received request for port, sending {}", self.port);
        self.port
    }
}

async fn get_desktops(Extension(desktops): Extension<Arc<Vec<Desktop>>>) -> impl IntoResponse {
    let raw_vec = desktops.deref().clone();
    info!("Got request for available desktops");
    Json(raw_vec)
}
