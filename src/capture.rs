use std::{collections::HashMap, net::TcpListener, process::Command};
// use std::process::Stdio;

use log::{debug, error, info, warn};
use serde::Serialize;
use tokio::{
    io::AsyncWriteExt,
    sync::broadcast::{Receiver, Sender},
};
use zvariant::{ObjectPath, OwnedValue};

use crate::{
    call_and_receive_response,
    screencast::{
        CreateSessionOptions, CreateSessionResponse, CursorMode, PersistMode, ScreencastProxy,
        SelectSourcesOptions, SourceType, StartCastOptions, StartCastResponse,
    },
    session_request::{RequestProxy, SessionProxy},
    unique_token::UniqueToken,
    Result, DESTINATION, PATH,
};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("CaptureManager is already running")]
    AlreadyStarted,
    #[error("An operation on the token failed, this shouldn't occur and should be considered a serious matter")]
    FailedTokenOperation,
}

/// Struct representing a desktop in an easier way
#[derive(Serialize, Debug, Clone)]
pub struct Desktop {
    /// The pipewire internal id of the desktop
    pub id: String,
    /// Loded id
    pub loded_id: u64,
    /// The pipewire path of the desktop
    pub pipewire_path: u32,
    /// The desktop's width
    pub width: i32,
    /// The desktop's height
    pub height: i32,
    /// The port the desktop is being streamed to
    pub port: Option<u16>,
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

    async fn try_get_token(&self) -> Result<String> {
        Ok(tokio::fs::read_to_string("./token").await?)
    }

    async fn try_write_token(&self) -> Result<()> {
        if let Some(token) = self.token.as_ref() {
            let mut file = tokio::fs::File::create("./token").await?;
            file.write_all(token.as_bytes()).await?;
            Ok(())
        } else {
            Err(Error::FailedTokenOperation.into())
        }
    }

    /// Returns desktops and File descriptor
    pub async fn begin_capture(&mut self, ds_tx: &Sender<()>) -> Result<Vec<Desktop>> {
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

        let desktops = start_res.streams.iter().enumerate().filter_map(|(idx, i)| {
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
                    loded_id: idx as u64,
                    pipewire_path: i.pipewire_path(),
                    width,
                    height,
                    port: None,
                }
            )
        }).collect::<Vec<Desktop>>();
        debug!("Filtered Viable Desktops");

        let desktops_with_ports: Vec<Desktop> = desktops
            .iter()
            .flat_map(|d| {
                let port = match Self::stream_desktop_gstreamer(
                    d.pipewire_path,
                    d.width,
                    d.height,
                    ds_tx.subscribe(),
                ) {
                    Ok(v) => v,
                    Err(e) => {
                        warn!(
                            "Failed to spawn stream for Desktop {}: {e}",
                            d.pipewire_path
                        );
                        return None;
                    }
                };
                Some(Desktop {
                    port: Some(port),
                    id: d.id.clone(),
                    ..*d
                })
            })
            .collect::<Vec<Desktop>>();

        Ok(desktops_with_ports)
    }

    fn stream_desktop_gstreamer(
        path: u32,
        width: i32,
        height: i32,
        mut ds_rx: Receiver<()>,
    ) -> Result<u16> {
        let socket = TcpListener::bind("127.0.0.1:0")?;
        let port = socket.local_addr()?.port();
        drop(socket);

        let mut cmd = Command::new("sh");

        /*
        cmd.stderr(Stdio::null());
        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::null());
        */

        cmd.args([
            "-c",
            // &format!(r#"gst-launch-1.0 -vvv pipewiresrc path={path} ! videoconvert ! tee name=split ! queue ! autovideosink split. ! x264enc speed-preset=superfast tune=zerolatency byte-stream=true sliced-threads=true threads=12 ! video/x-h264,stream-format=byte-stream,alignment=au,width={width},height={height} ! rtph264pay ! udpsink host=127.0.0.1 port={port}"#),
            &format!(r#"gst-launch-1.0 pipewiresrc path={path} ! video/x-raw,format=BGRx,width={width},height={height} ! videoconvert ! video/x-raw,format=Y444,width={width},height={height} ! x264enc speed-preset=superfast tune=zerolatency byte-stream=true sliced-threads=true threads=12 ! video/x-h264,stream-format=byte-stream,alignment=au,width={width},height={height} ! rtph264pay ! udpsink host=127.0.0.1 port={port}"#),
            // &format!(r#"gst-launch-1.0 -vvv pipewiresrc path={path} ! queue ! video/x-raw,format=BGRx,width={width},height={height} ! videoconvert ! x264enc speed-preset=superfast tune=zerolatency byte-stream=true sliced-threads=true ! rtph264pay ! udpsink host=127.0.0.1 port={port}"#),
        ]);

        tokio::spawn(async move {
            let mut child = cmd.spawn().expect("Failed to spawn gstreamer instance");

            info!("Started GStreamer Instance");

            if (ds_rx.recv().await).is_ok() {
                if child.kill().is_ok() {
                    info!("Killed GStreamer Pipeline for Path {path}");
                } else {
                    warn!("Failed to kill GSTreamer Pipeline for Path {path}");
                }
            } else {
                warn!("Failed to receive death signal");
            }
        });

        Ok(port)
    }
}
