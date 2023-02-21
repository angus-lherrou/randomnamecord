use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::json;

pub(crate) struct Data {} // User data, which is stored and accessible in all command invocations
pub(crate) type Error = Box<dyn std::error::Error + Send + Sync>;
pub(crate) type Context<'a> = poise::Context<'a, Data, Error>;

#[derive(Clone, Copy, Deserialize, Serialize, Debug)]
#[serde(rename_all = "lowercase")]
pub(crate) enum GenMode {
    Coherent,
    Chaotic,
}

impl FromStr for GenMode {
    type Err = serde_json::error::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_value::<GenMode>(json!(s))
    }
}
