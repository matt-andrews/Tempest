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
            let run_path = &resolve_run_path(&path, &run)?;
            
            let discovery =
                &discovery::discover(&dunce::canonicalize(&path)?, None, &mut HashMap::new(), run_path)?;
            pipeline::execute(discovery, &options).await?;
        }
    }

    Ok(())
}

fn resolve_run_path(project_dir: &PathBuf, run: &PathBuf) -> anyhow::Result<PathBuf> {
    let root = dunce::canonicalize(&project_dir)?;

    let sanitized: PathBuf = run
        .components()
        .filter(|c| matches!(
            c,
            std::path::Component::Normal(_) | std::path::Component::CurDir
        ))
        .collect();

    Ok(root.join(sanitized))
}
