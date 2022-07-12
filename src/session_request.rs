use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use zvariant::{OwnedValue, Type};

use zbus::{dbus_proxy, fdo::Result, Connection};

use crate::{unique_token::UniqueToken, DESTINATION};

#[dbus_proxy(
    interface = "org.freedesktop.portal.Session",
    default_service = "org.freedesktop.portal.Session",
    default_path = "/org/freedesktop/portal/Session"
)]
pub trait Session {
    fn close(&self) -> Result<()>;

    #[dbus_proxy(signal)]
    fn closed(&self, options: HashMap<String, OwnedValue>) -> Result<()>;

    #[dbus_proxy(property)]
    fn get_version(&self) -> Result<u32>;
}

/// Indicates how the user interaction ended
#[derive(Serialize, Deserialize, Type, Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u32)]
pub enum ResponseCode {
    Success = 0,
    Cancelled = 1,
    UnknownEnded = 2,
}

#[dbus_proxy(interface = "org.freedesktop.portal.Request")]
pub trait Request {
    fn close(&self) -> Result<()>;

    #[dbus_proxy(signal)]
    fn response(&self, response: ResponseCode, results: HashMap<String, OwnedValue>) -> Result<()>;
}

impl<'a> RequestProxy<'a> {
    pub async fn from_unique(conn: &Connection, handle: &UniqueToken) -> RequestProxy<'a> {
        RequestProxy::builder(conn)
            .path(get_path_by_unique_id("request", conn, handle).await)
            .unwrap()
            .destination(DESTINATION)
            .unwrap()
            .build()
            .await
            .unwrap()
    }
}

pub async fn get_path_by_unique_id(ty: &str, conn: &Connection, handle: &UniqueToken) -> String {
    format!(
        "/org/freedesktop/portal/desktop/{ty}/{}/{handle}",
        conn.unique_name()
            .unwrap()
            .trim_start_matches(':')
            .replace('.', "_")
    )
}

#[macro_export]
/// Future, request, desired type
macro_rules! call_and_receive_response {
    ($future:expr, $req:ident, $ty:ty) => {{
        use ::futures::StreamExt;
        let mut stream = $req.receive_response().await?;
        let (res, rp): ($ty, $crate::session_request::RequestProxy) = futures::try_join!(
            async {
                let res_item: $crate::session_request::Response = stream
                    .next()
                    .await
                    .ok_or(::zbus::fdo::Error::ZBus(::zbus::Error::InvalidReply))?;

                let (res_code, res) = res_item
                    .body::<(u32, $ty)>()
                    .map_err(|e| ::zbus::fdo::Error::ZBus(e))?;

                if res_code == 0 {
                    Ok(res)
                } else {
                    Err(::zbus::fdo::Error::Failed(
                        "Interaction Cancelled: {res_code}".to_owned(),
                    ))
                }
            },
            async { $future.await.map_err(|e| ::zbus::fdo::Error::ZBus(e)) }
        )?;

        if $req.path() == rp.path() {
            Ok(res)
        } else {
            Err(::zbus::fdo::Error::ZBus(::zbus::Error::InvalidReply))
        }
    }};
}
