mod test_model;
mod test_engine;

use std::path::PathBuf;
use std::sync::Arc;
use clap::{Parser, Subcommand};
use tracing::{info};
use walkdir::WalkDir;
use crate::test_engine::entry::Entry;
use crate::test_model::DescribeModel;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}
#[derive(Subcommand)]
enum Commands {
    Test{
        #[arg(long, default_value = "/data/tests")]
        path: PathBuf,
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_init();

    let args = Cli::parse();
    match args.command{
        Commands::Test{path} => {
            let test_files = walk_dir(path);
            Entry::new().run_tests(test_files);
        }
    }

    Ok(())
}

fn walk_dir(dir: PathBuf) -> Vec<DescribeModel>{
    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            println!("{}", entry.path().display());
        }
    }

    Vec::new()
}

fn tracing_init(){
    use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    let format = std::env::var("RUST_LOG_FORMAT")
        .unwrap_or_default()
        .to_ascii_lowercase();
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let ansi = std::env::var("NO_COLOR").is_err();
    let fmt_layer: Box<dyn tracing_subscriber::Layer<_> + Send + Sync> = match format.as_str() {
        "json" => Box::new(fmt::layer().json()),
        "pretty" => Box::new(fmt::layer().pretty().with_ansi(ansi)),
        _ => Box::new(fmt::layer().compact().with_ansi(ansi)),
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .init();
}
