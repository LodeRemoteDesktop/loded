pub(crate) mod api;
pub(crate) mod capture;
pub(crate) mod input;
pub(crate) mod screencast;
pub(crate) mod session_request;
pub(crate) mod unique_token;

pub use api::ApiManager;
pub use capture::CaptureManager;
pub use input::{InputManager, KeyDirection};

pub const DESTINATION: &str = "org.freedesktop.portal.Desktop";
pub const PATH: &str = "/org/freedesktop/portal/desktop";

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
