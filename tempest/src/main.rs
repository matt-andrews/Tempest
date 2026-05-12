mod models;
pub mod discovery;
pub mod pipeline;
mod utils;

use std::path::PathBuf;
use clap::{Parser, Subcommand};
use crate::models::options_model::OptionsModel;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}
#[derive(Subcommand)]
enum Commands {
    Test{
        #[arg(long, default_value = "/etc/tests")]
        path: PathBuf,
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let args = Cli::parse();
    match args.command{
        Commands::Test{path} => {
            let options = OptionsModel::default();
            let discovery = discovery::discover(path, None)?;
            pipeline::execute(discovery, options).await?;
        }
    }

    Ok(())
}

