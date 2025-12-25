use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlatpakPackage {
    pub name: String,
    pub app_id: String,
    pub version: String,
    pub branch: String,
    pub origin: String,
    pub description: String,
}
