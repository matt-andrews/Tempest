pub mod discovery;
mod models;
pub mod pipeline;
mod utils;

use crate::models::run_options::RunOptions;
use clap::{Parser, Subcommand};
use std::collections::HashMap;
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
        #[arg(long, default_value = "/")]
        run: PathBuf,
        #[arg(short, long, default_value = "false")]
        debug: bool,
        #[arg(long, default_value = "0")]
        retries: u8,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let args = Cli::parse();
    match args.command {
        Commands::Test {
            path,
            run,
            debug,
            retries,
        } => {
            let options = RunOptions::default_from_args(debug, retries);

            let discovery =
                &discovery::discover(&path, None, &mut HashMap::new(), &path.join(&run))?;
            pipeline::execute(discovery, &options).await?;
        }
    }

    Ok(())
}
