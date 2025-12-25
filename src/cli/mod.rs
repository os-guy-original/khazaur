pub mod args;
pub mod install;
pub mod search;
pub mod interactive;
pub mod package_source;
pub mod source_finder;
pub mod optional_deps;
pub mod selector;

use crate::error::Result;
pub use args::Args;

pub use package_source::*;
pub use source_finder::*;

/// Execute CLI command
pub async fn execute(args: Args) -> Result<()> {
    args.execute().await
}
