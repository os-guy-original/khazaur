use crate::error::{KhazaurError, Result};
use serde::{Deserialize, Serialize};

/// AUR package information from RPC API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AurPackage {
    #[serde(rename = "ID")]
    pub id: u64,
    pub name: String,
    pub package_base: String,
    pub version: String,
    pub description: Option<String>,
    #[serde(rename = "URL")]
    pub url: Option<String>,
    pub maintainer: Option<String>,
    pub first_submitted: u64,
    pub last_modified: u64,
    pub num_votes: u32,
    pub popularity: f64,
    pub out_of_date: Option<u64>,
    
    #[serde(default)]
    pub depends: Vec<String>,
    #[serde(default)]
    pub make_depends: Vec<String>,
    #[serde(default)]
    pub opt_depends: Vec<String>,
    #[serde(default)]
    pub conflicts: Vec<String>,
    #[serde(default)]
    pub provides: Vec<String>,
    #[serde(default)]
    pub replaces: Vec<String>,
    
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub license: Vec<String>,
}

impl AurPackage {
    /// Get all dependencies (depends + makedepends)
    pub fn all_depends(&self) -> Vec<String> {
        let mut deps = self.depends.clone();
        deps.extend(self.make_depends.clone());
        deps
    }



}

/// AUR RPC API response
#[derive(Debug, Deserialize)]
pub struct AurResponse {
    #[allow(dead_code)]
    pub version: u32,
    #[serde(rename = "type")]
    pub response_type: String,
    pub resultcount: u32,
    pub results: Vec<AurPackage>,
    pub error: Option<String>,  // Error message from API if type is "error"
}

impl AurResponse {
    /// Check if the response indicates an error
    pub fn is_error(&self) -> bool {
        self.response_type == "error"
    }

    /// Get the first result, if any
    pub fn first(&self) -> Result<&AurPackage> {
        self.results.first()
            .ok_or_else(|| KhazaurError::PackageNotFound("No results".to_string()))
    }
}
