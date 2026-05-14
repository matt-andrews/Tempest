pub mod discovery;
mod models;
pub mod pipeline;
mod utils;

use crate::models::options_model::OptionsModel;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}
#[derive(Subcommand)]
enum Commands {
    Test {
        #[arg(long, default_value = "/etc/tests")]
        path: PathBuf,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let args = Cli::parse();
    match args.command {
        Commands::Test { path } => {
            let mut options = OptionsModel::default_debug_false();
            options.reports = Some(vec!["console".to_string()]);

            let discovery = &discovery::discover(&path, None)?;
            pipeline::execute(discovery, &options).await?;
        }
    }

    Ok(())
}
