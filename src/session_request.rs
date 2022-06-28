use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use zvariant::{OwnedValue, Type};

use zbus::{dbus_proxy, fdo::Result, Connection};

use crate::{handle_token::UniqueToken, DESTINATION};

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
#[derive(Serialize, Deserialize, Type, Debug)]
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
            .path(get_path_by_unique_id(conn, handle).await)
            .unwrap()
            .destination(DESTINATION)
            .unwrap()
            .build()
            .await
            .unwrap()
    }
}

pub async fn get_path_by_unique_id(conn: &Connection, handle: &UniqueToken) -> String {
    format!(
        "/org/freedesktop/portal/desktop/session/{}/{}",
        conn.unique_name()
            .unwrap()
            .trim_start_matches(':')
            .replace('.', "_"),
        handle
    )
}
