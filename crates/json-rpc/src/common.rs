use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum Id {
    Number(u64),
    String(String),
    None,
}

impl Id {
    pub fn is_none(&self) -> bool {
        matches!(self, Id::None)
    }
}
