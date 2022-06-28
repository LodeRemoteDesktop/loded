use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use zbus::{dbus_proxy, fdo::Result};
use zvariant::{DeserializeDict, ObjectPath, OwnedFd, OwnedValue, SerializeDict, Type};

use crate::handle_token::UniqueToken;
use crate::session_request::*;

use bitflags::bitflags;

bitflags! {
    /// The source types that should be presented to be chose from
    #[derive(Type, Serialize, Deserialize)]
    pub struct SourceType: u32 {
        /// Whole Monitors
        const MONITOR = 1 << 0;
        /// Specific Windows
        const WINDOW = 1 << 1;
        /// Virtual Desktops
        const VIRTUAL = 1 << 2;
    }

    /// The cursor mode to be used
    #[derive(Type, Serialize, Deserialize)]
    pub struct CursorMode: u32 {
        /// The cursor isn't shown
        const HIDDEN = 1 << 0;
        /// The cursor is embedded in the stream
        const EMBEDDED = 1 << 1;
        /// The cursor's position is sent alongside pipewire stream data
        const METADATA = 1 << 2;
    }
}

/// Permission persistence options
#[repr(u32)]
#[derive(Type, Serialize, Deserialize, Debug)]
pub enum PersistMode {
    /// Do not persist permissions
    DoNot = 0,
    /// Persist permissions so long as the application is running
    Application = 1,
    /// Persist permissions until they are explicitly revoked
    ExplicitlyRevoked = 2,
}

/// Options used while creating a session
#[derive(DeserializeDict, SerializeDict, Type, Default, Debug)]
#[zvariant(signature = "dict")]
pub struct CreateSessionOptions {
    pub handle_token: UniqueToken,
    pub session_handle_token: UniqueToken,
}

#[derive(DeserializeDict, SerializeDict, Type, Debug)]
#[zvariant(signature = "dict")]
pub struct CreateSessionResponse {
    pub response: ResponseCode,
    pub session_handle: String,
}

/// Options for the SelectSource method
#[derive(DeserializeDict, SerializeDict, Type, Debug)]
#[zvariant(signature = "dict")]
pub struct SelectSourcesOptions {
    /// String to use as last element of handle
    pub handle_token: UniqueToken,
    /// Types of input to record (Use [SourceType])
    pub types: Option<u32>,
    /// Allow multiple sources to be recorded
    pub multiple: Option<bool>,
    /// The cursor mode (Use [CursorMode])
    pub cursor_mode: Option<u32>,
    /// The restore token
    pub restore_token: Option<String>,
    /// Permission persistence mode (Use [PersistMode])
    pub persist_mode: Option<u32>,
}

#[derive(SerializeDict, DeserializeDict, Type, Debug)]
#[zvariant(signature = "dict")]
pub struct StartCastOptions {
    /// String to use as last element of handle
    pub handle_token: UniqueToken,
}

/// It's really just a string, anyways. Lifetimes are dumb and break stuff
// pub type ObjectPath = String;

#[dbus_proxy(interface = "org.freedesktop.portal.ScreenCast")]
pub trait Screencast {
    #[dbus_proxy(object = "Request")]
    fn create_session(&self, options: &CreateSessionOptions);

    #[dbus_proxy(object = "Request")]
    fn select_sources(&self, session_handle: &ObjectPath<'_>, options: &SelectSourcesOptions);

    fn open_pipe_wire_remote(
        &self,
        session_handle: &ObjectPath<'_>,
        options: std::collections::HashMap<&str, zbus::zvariant::Value<'_>>,
    ) -> Result<OwnedFd>;

    #[dbus_proxy(object = "Session")]
    fn start(
        &self,
        session_handle: &ObjectPath<'_>,
        parent_window: &str,
        options: &StartCastOptions,
    );

    /// This returns [CursorMode] as a u32
    #[dbus_proxy(property)]
    fn available_cursor_modes(&self) -> Result<u32>;

    /// This returns [SourceType] as a u32
    #[dbus_proxy(property)]
    fn available_source_types(&self) -> Result<u32>;

    #[dbus_proxy(property)]
    fn version(&self) -> zbus::Result<u32>;
}
