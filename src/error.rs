use thiserror::Error;

#[derive(Error, Debug)]
pub enum KhazaurError {
    #[error("AUR API error: {0}")]
    AurApi(String),

    #[error("Package not found: {0}")]
    PackageNotFound(String),

    #[error("Failed to download package: {0}")]
    DownloadFailed(String),

    #[error("Build failed: {0}")]
    BuildFailed(String),

    #[error("Pacman command failed: {0}")]
    PacmanFailed(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),




    #[error("Dialog error: {0}")]
    Dialog(String),
}

impl From<dialoguer::Error> for KhazaurError {
    fn from(err: dialoguer::Error) -> Self {
        KhazaurError::Dialog(err.to_string())
    }
}

pub type Result<T> = std::result::Result<T, KhazaurError>;
