use std::sync::Arc;

use log::{debug, info, warn};
use thiserror::Error;

pub mod handle_token;
pub mod screencast;
pub mod session_request;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/*
use ashpd::{
    desktop::{
   &     screencast::{CursorMode, PersistMode, ScreenCastProxy, SourceType},
        SessionProxy,
    },
    zbus, WindowIdentifier,
};

#[derive(Error, Debug)]
pub enum Error {
    #[error("DesktopManager is not running")]
    NotStarted,
    #[error("DesktopManager is already running")]
    AlreadyStarted,
}

pub struct Desktop {
    pub pipewire_path: u64,
    pub width: i32,
    pub height: i32,
    pub index: u64,
}

pub struct CaptureManager<'a> {
    token: Option<String>,
    connection: Option<zbus::Connection>,
    session: Option<Box<SessionProxy<'a>>>,
}

impl<'a> CaptureManager<'a> {
    pub const fn new() -> Result<CaptureManager<'a>> {
        Ok(Self {
            token: None,
            connection: None,
            session: None,
        })
    }

    /// Returns desktops and File descriptor
    pub async fn begin_capture(&mut self) -> Result<(Vec<Desktop>, i32)> {
        info!("Beginning Desktop Capture");

        self.connection = Some(zbus::Connection::session().await?);

        let proxy = ScreenCastProxy::new(self.connection.as_ref().unwrap()).await?;
        let session = proxy.create_session().await?;
        debug!("Made ScreenCastProxy and SessionProxy");

        let token = &mut self.token;

        proxy
            .select_sources(
                &session,
                CursorMode::Embedded.into(),
                SourceType::Monitor.into(),
                false,
                token.as_deref(),
                PersistMode::Application,
            )
            .await?;
        debug!("Set ScreenCastProxy Source Options");

        let identifier = WindowIdentifier::default();

        let (streams, new_token) = proxy.start(&session, &identifier).await?;
        if let Some(t) = new_token {
            token.replace(t);
        }
        let fd = proxy.open_pipe_wire_remote(&session).await?;

        self.session = Some(session);

        debug!("Started ScreenCastProxy");

        let desktop_vec: Vec<Desktop> = streams
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                let (width, height) = match item.size() {
                    Some(v) => v,
                    None => {
                        warn!("Desktop {index} is missing WIDTH and DEPTH, skipping encoding");
                        return None;
                    }
                };
                Some(Desktop {
                    pipewire_path: item.pipe_wire_node_id().into(),
                    width,
                    height,
                    index: index as u64,
                })
            })
            .collect::<Vec<Desktop>>();

        debug!("Filtered Viable Desktops");

        Ok((desktop_vec, fd))
    }

    pub async fn end_capture(&mut self) -> Result<()> {
        let connection = match self.connection.take() {
            Some(v) => v,
            None => return Err(Error::NotStarted.into()),
        };
        let session = match self.session.take() {
            Some(v) => v,
            None => return Err(Error::NotStarted.into()),
        };

        let session = Arc::new(session);
        // let session: Arc<SessionProxy<'static>> = unsafe { std::mem::transmute(session) };

        info!("Ending Desktop Capture");

        tokio::spawn(async move {
            let _connection = connection;
            session.close().await
        });

        Ok(())
    }
}
*/

pub const DESTINATION: &str = "org.freedesktop.portal.Desktop";
pub const PATH: &str = "/org/freedesktop/portal/desktop";

#[cfg(test)]
mod tests {
    use crate::{screencast::*, session_request::get_path_by_unique_id};

    #[tokio::test]
    async fn opens_menu() {
        let conn = zbus::Connection::session().await.unwrap();

        let mut connection_ops = CreateSessionOptions::default();

        let path = get_path_by_unique_id(&conn, &connection_ops.handle_token).await;

        println!("Path: {}", path);
        let proxy = ScreencastProxy::builder(&conn)
            .path(path)
            .expect("Path was invalid in proxy builder")
            .build()
            .await
            .expect("Failed to build proxy");

        dbg!(&proxy);

        loop {}

        let val = proxy
            .create_session(&connection_ops)
            .await
            .expect("Failed to create session");
        dbg!(&val);
        panic!()
    }
}
