mod models;
pub mod discovery;
pub mod engine;

use std::path::PathBuf;
use clap::{Parser, Subcommand};

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
            let discover = discovery::discover(path, None)?;
            engine::execute(discover).await?;
        }
    }

    Ok(())
}

