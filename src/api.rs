use std::{
    net::{SocketAddr, TcpListener},
    // ops::Deref,
    sync::Arc,
};

// use axum::{response::IntoResponse, routing::get, Extension, Json, Router};

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
        /*
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
        */
        Ok(())
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

/*
async fn get_desktops(Extension(desktops): Extension<Arc<Vec<Desktop>>>) -> impl IntoResponse {
    let raw_vec = desktops.deref().clone();
    info!("Got request for available desktops");
    Json(raw_vec)
}
*/

mod helpers {
    use std::{path::Path, sync::Arc};

    use log::debug;
    use rustls::{Certificate, PrivateKey};

    pub async fn read_certs(
        root: Arc<Path>,
    ) -> Result<(PrivateKey, Certificate), Box<dyn std::error::Error>> {
        let key_p = root.join("key.der");
        let cert_p = root.join("cert.der");

        let key_rr = tokio::fs::read(&key_p).await;
        let cert_rr = tokio::fs::read(&cert_p).await;
        match key_rr.and_then(|k| Ok((k, cert_rr?))) {
            Ok((kv, cv)) => Ok((PrivateKey(kv), Certificate(cv))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                debug!("Generating Self-Signed Key and Certificate");
                let cert = match rcgen::generate_simple_self_signed(vec!["localhost".into()]) {
                    Ok(c) => c,
                    Err(e) => {
                        return Err(e.into());
                    }
                };

                let key = cert.serialize_private_key_der();
                let cert = cert.serialize_der()?;

                let key_wr = tokio::spawn(tokio::fs::write(key_p.clone(), key.clone()));
                let cert_wr = tokio::spawn(tokio::fs::write(cert_p.clone(), cert.clone()));

                let _ = key_wr.await?;
                let _ = cert_wr.await?;

                Ok((PrivateKey(key), Certificate(cert)))
            }
            Err(e) => Err(e.into()),
        }
    }

    pub fn server_config(
        cert: Vec<Certificate>,
        key: PrivateKey,
    ) -> Result<quinn::ServerConfig, Box<dyn std::error::Error>> {
        let mut crypto = rustls::ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(cert, key)?;
        crypto.alpn_protocols = vec![b"hq-29".to_vec()];

        let mut server_config = quinn::ServerConfig::with_crypto(Arc::new(crypto));
        Arc::get_mut(&mut server_config.transport)
            .unwrap()
            .max_concurrent_uni_streams(0u8.into());
        Ok(server_config)
    }
}
