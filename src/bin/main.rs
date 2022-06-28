use std::{io::BufRead, sync::Arc};

use ashpd::{
    desktop::screencast::{CursorMode, PersistMode, SourceType},
    enumflags2::_internal::RawBitFlags,
};
use rdesktopd::{
    handle_token::UniqueToken,
    screencast::{
        CreateSessionOptions, CreateSessionResponse, ScreencastProxy, SelectSourcesOptions,
        StartCastOptions,
    },
    session_request::{get_path_by_unique_id, RequestProxy, ResponseStream},
    DESTINATION, PATH,
};
use zbus::{export::futures_util::StreamExt, SignalStream};
use zvariant::ObjectPath;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let conn = zbus::Connection::session().await.unwrap();

    let connection_ops = CreateSessionOptions::default();

    let path = get_path_by_unique_id(&conn, &connection_ops.handle_token).await;

    println!("Path: {}", path);
    let proxy = ScreencastProxy::builder(&conn)
        .path(PATH)?
        .destination(DESTINATION)?
        .build()
        .await
        .expect("Failed to build proxy");

    dbg!(&proxy);

    println!("Getting session");
    let sess_opts = CreateSessionOptions::default();
    let request = RequestProxy::from_unique(&conn, &sess_opts.handle_token).await;

    dbg!(&request);

    /*
    let stream = request
        .into_inner()
        .receive_signal("Response")
        .await
        .unwrap();
    */
    let mut stream = request.inner().receive_signal("Response").await.unwrap();

    dbg!(&stream);

    println!("Joining");
    let (csr, _rp): (CreateSessionResponse, RequestProxy) =
        futures::try_join!(async { stream.next().await.unwrap().body() }, async {
            proxy.create_session(&sess_opts).await
        })?;

    dbg!(&csr);
    println!("WE'RE OUT");

    let session =
        ObjectPath::try_from(std::io::stdin().lock().lines().next().unwrap().unwrap()).unwrap();

    dbg!(&session);

    println!("Activating selecting sources menu");
    let res_proxy = proxy
        .select_sources(
            &session,
            &SelectSourcesOptions {
                handle_token: UniqueToken::new(),
                types: Some(SourceType::Monitor.bits()),
                multiple: Some(true),
                cursor_mode: Some(CursorMode::Embedded.bits()),
                restore_token: None,
                persist_mode: Some(PersistMode::ExplicitlyRevoked as u32),
            },
        )
        .await?;

    println!("Getting stream");
    let mut res_stream = res_proxy.receive_response().await.unwrap();
    dbg!(&res_stream);

    println!("Getting response");
    let res = res_stream.next().await.unwrap();
    dbg!(res);

    println!("Streaming");
    proxy
        .start(
            &session,
            "RDESKTOPD",
            &StartCastOptions {
                handle_token: UniqueToken::new(),
            },
        )
        .await?;

    loop {
        std::thread::yield_now();
    }
}
