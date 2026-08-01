pub mod discovery;
mod models;
pub mod pipeline;
mod utils;
pub mod content;

use crate::models::run_options::RunOptions;
use crate::models::summary_result::SummaryResult;
use crate::pipeline::warnings;
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process;

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
        #[arg(short, long, default_value = "false")]
        strict: bool,
        #[arg(short, long, default_value = "false")]
        warn_as_err: bool,
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
            strict,
            warn_as_err,
        } => {
            let options = RunOptions::default_from_args(debug, retries);
            let run_path = &resolve_run_path(&path, &run)?;

            let discovery = &discovery::discover(
                &dunce::canonicalize(&path)?,
                None,
                &mut HashMap::new(),
                run_path,
            )?;

            let result = pipeline::execute(discovery, &options).await?;

            print_warnings();

            let exit = determine_exit_code(result, strict, warn_as_err);
            process::exit(exit as i32);
        }
    }
}

fn resolve_run_path(project_dir: &PathBuf, run: &Path) -> anyhow::Result<PathBuf> {
    let root = dunce::canonicalize(project_dir)?;

    let sanitized: PathBuf = run
        .components()
        .filter(|c| {
            matches!(
                c,
                std::path::Component::Normal(_) | std::path::Component::CurDir
            )
        })
        .collect();

    Ok(root.join(sanitized))
}

fn print_warnings() {
    let warning_count = warnings::get_warning_count();
    if warning_count > 0  {
        println!(
            "{}{}",
            "Total Warnings: ".yellow(),
            warning_count.to_string().yellow()
        );
        let warnings: HashSet<String> = warnings::get_all_warnings();
        for warning in warnings {
            println!("{}: {}", "WARN".yellow(), warning.yellow());
        }
    }
}

#[repr(i32)]
enum ExitCode {
    Success = 0,
    TestsFailed = 1,
    FlakyTests = 2,
}

fn determine_exit_code(result: SummaryResult, strict: bool, warn_as_err: bool) -> ExitCode {
    if warn_as_err && warnings::get_warning_count() > 0 {
        return ExitCode::TestsFailed;
    }
    if result == SummaryResult::Failed {
        ExitCode::TestsFailed
    } else if result == SummaryResult::Flaky && strict {
        ExitCode::FlakyTests
    } else {
        ExitCode::Success
    }
}
