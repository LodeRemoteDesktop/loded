use std::fmt::Display;

use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::{Deserialize, Serialize};
use zbus::names::OwnedMemberName;
use zvariant::Type;

#[derive(Serialize, Deserialize, Type, Debug, Clone)]
pub struct UniqueToken(OwnedMemberName);

impl UniqueToken {
    pub fn new() -> Self {
        let rng = thread_rng();
        let token: String = rng
            .sample_iter(Alphanumeric)
            .take(15)
            .map(char::from)
            .collect();
        UniqueToken(
            OwnedMemberName::try_from(format!("rdesktopd_{token}")).expect("Invalid Handle Token"),
        )
    }
}

impl Display for UniqueToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl Default for UniqueToken {
    fn default() -> Self {
        Self::new()
    }
}

impl TryFrom<&str> for UniqueToken {
    type Error = zbus::names::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(UniqueToken(OwnedMemberName::try_from(value)?))
    }
}

impl TryFrom<String> for UniqueToken {
    type Error = zbus::names::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(UniqueToken(OwnedMemberName::try_from(value)?))
    }
}
