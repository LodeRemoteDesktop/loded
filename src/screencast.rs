use std::ops::BitOr;

use serde::{Deserialize, Serialize};
use zbus::{dbus_proxy, fdo::Result};
use zvariant::{DeserializeDict, ObjectPath, SerializeDict, Type};

use crate::handle_token::UniqueToken;
use crate::session_request::*;

/// The source types that should be presented to be chose from
#[derive(Type, Serialize, Deserialize, Debug, Clone)]
#[zvariant(signature = "u")]
#[repr(transparent)]
pub struct SourceType(pub u32);

impl SourceType {
    /// Whole Monitors
    pub const MONITOR: Self = Self(1 << 0);
    /// Specific Windows
    pub const WINDOW: Self = Self(1 << 1);
    /// Virtual Desktops
    pub const VIRTUAL: Self = Self(1 << 2);
}

impl BitOr for SourceType {
    type Output = SourceType;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

/// The cursor mode to be used
#[derive(Type, Serialize, Deserialize, Debug, Clone)]
#[zvariant(signature = "u")]
#[repr(transparent)]
pub struct CursorMode(u32);

impl CursorMode {
    /// The cursor isn't shown
    pub const HIDDEN: Self = Self(1 << 0);
    /// The cursor is embedded in the stream
    pub const EMBEDDED: Self = Self(1 << 1);
    /// The cursor's position is sent alongside pipewire stream data
    pub const METADATA: Self = Self(1 << 2);
}

impl BitOr for CursorMode {
    type Output = CursorMode;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

/// Permission persistence options
#[repr(u32)]
#[derive(Type, Serialize, Deserialize, Debug)]
#[zvariant(signature = "u")]
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
    pub session_handle: Option<String>,
}

/// Options for the SelectSource method
#[derive(DeserializeDict, SerializeDict, Type, Debug)]
#[zvariant(signature = "dict")]
pub struct SelectSourcesOptions {
    /// String to use as last element of handle
    pub handle_token: UniqueToken,
    /// Types of input to record (Use [SourceType])
    pub types: Option<SourceType>,
    /// Allow multiple sources to be recorded
    pub multiple: Option<bool>,
    /// The cursor mode (Use [CursorMode])
    pub cursor_mode: Option<CursorMode>,
    /// The restore token
    pub restore_token: Option<String>,
    /// Permission persistence mode (Use [PersistMode])
    pub persist_mode: Option<PersistMode>,
}

#[derive(SerializeDict, DeserializeDict, Type, Debug)]
#[zvariant(signature = "dict")]
pub struct StartCastOptions {
    /// String to use as last element of handle
    pub handle_token: UniqueToken,
}

impl StartCastOptions {
    pub fn new_from(token: &UniqueToken) -> Self {
        Self {
            handle_token: token.clone(),
        }
    }
}

#[derive(SerializeDict, DeserializeDict, Type, Debug)]
#[zvariant(signature = "dict")]
pub struct StartCastResponse {
    pub streams: Vec<Stream>,
    pub restore_token: Option<String>,
}

/// Properties describing a stream
#[derive(SerializeDict, DeserializeDict, Type, Debug, Clone)]
#[zvariant(signature = "dict")]
pub struct StreamProperties {
    id: Option<String>,
    position: Option<(i32, i32)>,
    size: Option<(i32, i32)>,
    /// This is a [SourceType]
    source_type: Option<SourceType>,
}

impl StreamProperties {
    /// Gets the ID
    pub fn id(&self) -> Option<String> {
        self.id.clone()
    }

    /// Get the position in the format `(x, y)`
    pub fn position(&self) -> Option<(i32, i32)> {
        self.position
    }

    /// Get the size of the desktop in the format `(width, height)`
    pub fn size(&self) -> Option<(i32, i32)> {
        self.size
    }

    /// Get the source type
    pub fn source_type(&self) -> Option<SourceType> {
        self.source_type.clone()
    }
}

/// Struct representing a stream with its associated pipewire path
#[derive(Serialize, Deserialize, Type, Clone, Debug)]
pub struct Stream(u32, StreamProperties);

impl Stream {
    /// Get the pipewire path of the contained stream
    pub fn pipewire_path(&self) -> u32 {
        self.0
    }

    /// Get the stream properties
    pub fn properties(&self) -> &StreamProperties {
        &self.1
    }
}

#[dbus_proxy(interface = "org.freedesktop.portal.ScreenCast")]
pub trait Screencast {
    #[dbus_proxy(object = "Request")]
    fn create_session(&self, options: &CreateSessionOptions);

    #[dbus_proxy(object = "Request")]
    fn select_sources(&self, session_handle: &ObjectPath<'_>, options: &SelectSourcesOptions);

    #[dbus_proxy(object = "Request")]
    fn open_pipe_wire_remote(
        &self,
        session_handle: &ObjectPath<'_>,
        options: std::collections::HashMap<&str, zbus::zvariant::Value<'_>>,
    );

    #[dbus_proxy(object = "Request")]
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
