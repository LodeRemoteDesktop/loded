use std::{collections::HashMap, io::Write, sync::Arc};

use log::{debug, error, info, warn};
use serde::Serialize;
use session_request::SessionProxy;
use thiserror::Error;
use zvariant::{ObjectPath, OwnedValue};

use crate::{
    handle_token::UniqueToken,
    screencast::{
        CreateSessionOptions, CreateSessionResponse, CursorMode, PersistMode, ScreencastProxy,
        SelectSourcesOptions, SourceType, StartCastOptions, StartCastResponse,
    },
    session_request::RequestProxy,
};

pub mod handle_token;
pub mod screencast;
pub mod session_request;

pub const DESTINATION: &str = "org.freedesktop.portal.Desktop";
pub const PATH: &str = "/org/freedesktop/portal/desktop";

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("DesktopManager is not running")]
    NotStarted,
    #[error("DesktopManager is already running")]
    AlreadyStarted,
    #[error("{0}")]
    Other(String),
    #[error("An operation on the token failed, this shouldn't occur and should be considered a serious matter")]
    FailedTokenOperation,
}

/// Struct representing a desktop in an easier way
#[derive(Serialize, Debug)]
pub struct Desktop {
    /// The internal id of the desktop
    pub id: String,
    /// The pipewire path of the desktop
    pub pipewire_path: u32,
    /// The desktop's width
    pub width: i32,
    /// The desktop's height
    pub height: i32,
}

pub struct CaptureManager<'a> {
    token: Option<String>,
    connection: zbus::Connection,
    session: Option<Box<SessionProxy<'a>>>,
}

impl<'a> CaptureManager<'a> {
    pub async fn new() -> Result<CaptureManager<'a>> {
        Ok(Self {
            token: None,
            connection: zbus::Connection::session().await?,
            session: None,
        })
    }

    pub async fn try_get_token(&self) -> Result<String> {
        Ok(std::fs::read_to_string("./token")?)
    }

    pub async fn try_write_token(&self) -> Result<()> {
        if let Some(token) = self.token.as_ref() {
            let mut file = std::fs::File::create("./token")?;
            file.write_all(token.as_bytes())?;
            Ok(())
        } else {
            Err(Error::FailedTokenOperation.into())
        }
    }

    /// Returns desktops and File descriptor
    pub async fn begin_capture(&mut self) -> Result<Vec<Desktop>> {
        if self.session.is_some() {
            error!("CaptureManager is already running");
            return Err(Error::AlreadyStarted.into());
        }

        info!("Beginning Desktop Capture");

        match self.try_get_token().await {
            Ok(v) => {
                debug!("Refresh token present");
                self.token = Some(v);
            }
            Err(e) => warn!("Failed to read refresh token: {e}"),
        }

        let proxy = ScreencastProxy::builder(&self.connection)
            .path(PATH)?
            .destination(DESTINATION)?
            .build()
            .await?;

        debug!("Getting session");
        let sess_opts = CreateSessionOptions::default();
        let sess_request =
            RequestProxy::from_unique(&self.connection, &sess_opts.handle_token).await;

        let csr: CreateSessionResponse = call_and_receive_response!(
            proxy.create_session(&sess_opts),
            sess_request,
            CreateSessionResponse
        )?;

        let session = ObjectPath::try_from(
            csr.session_handle
                .expect("SessionHandle missing from successful CreateSessionResponse"),
        )
        .expect("Invalid SessionHandle in successful CreateSessionResponse");

        let token = match &self.token {
            Some(v) => {
                info!("Refresh token present, using token");
                Some(v.clone())
            }
            None => {
                warn!("Refresh token not present");
                None
            }
        };

        debug!("Requesting capture sources");
        let src_request_token = UniqueToken::new();
        let src_request = RequestProxy::from_unique(&self.connection, &src_request_token).await;
        let src_opts = SelectSourcesOptions {
            handle_token: src_request_token,
            types: Some(SourceType::MONITOR),
            multiple: Some(true),
            cursor_mode: Some(CursorMode::EMBEDDED),
            restore_token: token,
            persist_mode: Some(PersistMode::ExplicitlyRevoked),
        };

        let _ssr = call_and_receive_response!(proxy.select_sources(&session, &src_opts), src_request, HashMap<String, OwnedValue>)?;

        debug!("Starting stream request");
        let start_req_token = UniqueToken::new();
        let start_req = RequestProxy::from_unique(&self.connection, &start_req_token).await;
        let start_opts = StartCastOptions::new_from(&start_req_token);

        let mut start_res = call_and_receive_response!(
            proxy.start(&session, "RDESKTOPD", &start_opts,),
            start_req,
            StartCastResponse
        )?;

        self.token = Some(
            start_res
                .restore_token
                .take()
                .expect("No refresh token was present"),
        );

        match self.try_write_token().await {
            Ok(_) => info!("Wrote refresh token"),
            Err(e) => warn!("Failed to write refresh token. This will cause another permissions request the next time rdesktopd starts. Error: {e}"),
        }

        let desktops = start_res.streams.iter().filter_map(|i| {
            let (width, height) = match i.properties().size() {
                Some(v) => v,
                None => {
                    warn!("Desktop excluded due to missing size property: {:#?}\n(This probably isn't intentional)", i);
                    return None;
                }
            };
            let id = match i.properties().id() {
                Some(v) => v,
                None => {
                    warn!("Desktop excluded due to missing id: {:#?}\n(This probably isn't intentional)", i);
                    return None;
                }
            };
            Some(
                Desktop {
                    id,
                    pipewire_path: i.pipewire_path(),
                    width,
                    height,
                }
            )
        }).collect::<Vec<Desktop>>();
        debug!("Filtered Viable Desktops");

        Ok(desktops)
    }

    pub async fn end_capture(&mut self) -> Result<()> {
        let session = match self.session.take() {
            Some(v) => v,
            None => return Err(Error::NotStarted.into()),
        };

        let session = Arc::new(session);
        // let session: Arc<SessionProxy<'static>> = unsafe { std::mem::transmute(session) };

        info!("Ending Desktop Capture");

        session.close().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {}
