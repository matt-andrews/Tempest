pub mod cel;
pub mod content;
pub mod discovery;
pub mod environment;
mod models;
pub mod pipeline;
pub mod templating;
mod utils;

use crate::models::run_options::RunOptions;
use crate::models::summary_result::SummaryResult;
use crate::pipeline::warnings;
use anyhow::ensure;
use clap::{Parser, Subcommand};
use colored::Colorize;
use futures_util::FutureExt;
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::num::NonZeroUsize;
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::{env, process};

const ERROR_EXIT_CODE: u8 = 1;
const INTERNAL_ERROR_EXIT_CODE: u8 = 70;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}
#[derive(Subcommand)]
enum Commands {
    Version,
    Test {
        #[arg(long, default_value = "/etc/tests")]
        path: PathBuf,
        #[arg(short, long)]
        run: Option<Vec<PathBuf>>,
        #[arg(short, long, default_value = "false")]
        debug: bool,
        #[arg(long, default_value = "0")]
        retries: u8,
        #[arg(short, long, default_value = "false")]
        strict: bool,
        #[arg(short, long, default_value = "false")]
        warn_as_err: bool,
        /// Maximum number of spec files to execute at once.
        #[arg(long)]
        workers: Option<NonZeroUsize>,
        #[arg(short, long)]
        env: Option<Vec<String>>,
    },
}

#[tokio::main]
async fn main() -> process::ExitCode {
    install_panic_hook();

    match AssertUnwindSafe(run()).catch_unwind().await {
        Ok(Ok(exit)) => process::ExitCode::from(exit as u8),
        Ok(Err(error)) => {
            write_stderr(&format_error(&error));
            process::ExitCode::from(ERROR_EXIT_CODE)
        }
        Err(_) => process::ExitCode::from(INTERNAL_ERROR_EXIT_CODE),
    }
}

async fn run() -> anyhow::Result<ExitCode> {
    let args = Cli::parse();
    match args.command {
        Commands::Test {
            path,
            run,
            debug,
            retries,
            strict,
            warn_as_err,
            workers,
            env,
        } => {
            let project_dir = dunce::canonicalize(&path)?;
            load_project_dotenv(&project_dir)?;
            let options = RunOptions::default_from_args(debug, retries);
            let run_paths = &resolve_run_paths(&project_dir, run);

            let mut envs = parse_cli_envs(env)?;

            let discovery = &discovery::discover(&project_dir, None, &mut envs, run_paths)?;

            let result = pipeline::execute(discovery, &options, workers).await?;

            print_warnings();

            let exit = determine_exit_code(result, strict, warn_as_err);
            Ok(exit)
        }
        Commands::Version => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            Ok(ExitCode::Success)
        }
    }
}

fn load_project_dotenv(project_dir: &Path) -> anyhow::Result<()> {
    let path = project_dir.join(".env");

    match discovery::parse_env(&path) {
        Ok(envs) => unsafe {
            for e in envs {
                env::set_var(e.0, e.1);
            }
        },
        Err(err) => eprintln!("{}", err),
    };
    Ok(())
}

fn parse_cli_envs(env: Option<Vec<String>>) -> anyhow::Result<HashMap<String, String>> {
    env.unwrap_or_default()
        .into_iter()
        .map(|entry| -> anyhow::Result<(String, String)> {
            let (key, value) = entry.split_once('=').ok_or_else(|| {
                anyhow::anyhow!("invalid environment variable `{entry}`; expected KEY=VALUE")
            })?;

            let key = key.trim();
            let value = value.trim();

            ensure!(
                !key.is_empty(),
                "invalid environment variable `{entry}`; key cannot be empty"
            );

            Ok((key.to_owned(), value.to_owned()))
        })
        .collect()
}

fn install_panic_hook() {
    let default_hook = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |panic_info| {
        if backtrace_requested(std::env::var("RUST_BACKTRACE").ok().as_deref()) {
            default_hook(panic_info);
        } else {
            write_stderr(
                "fatal: Tempest encountered an unexpected internal error.\n\
                 Re-run with RUST_BACKTRACE=1 for diagnostic details.",
            );
        }
    }));
}

fn backtrace_requested(value: Option<&str>) -> bool {
    value.is_some_and(|value| !value.is_empty() && value != "0")
}

fn format_error(error: &anyhow::Error) -> String {
    format!("error: {error:#}")
}

fn write_stderr(message: &str) {
    let _ = writeln!(std::io::stderr().lock(), "{message}");
}

fn resolve_run_paths(project_dir: &Path, runs: Option<Vec<PathBuf>>) -> Vec<PathBuf> {
    runs.map(|runs| {
        runs.iter()
            .map(|run| {
                let sanitized: PathBuf = run
                    .components()
                    .filter(|component| {
                        matches!(
                            component,
                            std::path::Component::Normal(_) | std::path::Component::CurDir
                        )
                    })
                    .collect();

                project_dir.join(sanitized)
            })
            .collect()
    })
    .unwrap_or_else(|| vec![project_dir.to_path_buf()])
}

fn print_warnings() {
    let warning_count = warnings::get_warning_count();
    if warning_count > 0 {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed_runs(args: &[&str]) -> Result<Option<Vec<PathBuf>>, clap::Error> {
        let cli = Cli::try_parse_from(args)?;
        let Commands::Test { run, .. } = cli.command else {
            panic!("expected the test command");
        };
        Ok(run)
    }

    fn parsed_workers(args: &[&str]) -> Result<Option<NonZeroUsize>, clap::Error> {
        let cli = Cli::try_parse_from(args)?;
        let Commands::Test { workers, .. } = cli.command else {
            panic!("expected the test command");
        };
        Ok(workers)
    }

    #[test]
    fn run_paths_are_optional() {
        assert_eq!(parsed_runs(&["tempest", "test"]).unwrap(), None);
    }

    #[test]
    fn run_accepts_multiple_occurrences() {
        assert_eq!(
            parsed_runs(&[
                "tempest",
                "test",
                "--run",
                "api",
                "--run",
                "smoke/test.spec.yml",
            ])
            .unwrap(),
            Some(vec![
                PathBuf::from("api"),
                PathBuf::from("smoke/test.spec.yml"),
            ])
        );
    }

    #[test]
    fn omitted_run_resolves_to_project_root() {
        let project_dir = Path::new("project");

        assert_eq!(
            resolve_run_paths(project_dir, None),
            vec![project_dir.to_path_buf()]
        );
    }

    #[test]
    fn run_paths_resolve_relative_to_project_root() {
        let project_dir = Path::new("project");

        assert_eq!(
            resolve_run_paths(
                project_dir,
                Some(vec![
                    PathBuf::from("api"),
                    PathBuf::from("smoke/test.spec.yml"),
                ]),
            ),
            vec![
                project_dir.join("api"),
                project_dir.join("smoke/test.spec.yml"),
            ]
        );
    }

    #[test]
    fn workers_are_optional() {
        assert_eq!(parsed_workers(&["tempest", "test"]).unwrap(), None);
    }

    #[test]
    fn workers_accepts_a_positive_limit() {
        assert_eq!(
            parsed_workers(&["tempest", "test", "--workers", "4"])
                .unwrap()
                .map(NonZeroUsize::get),
            Some(4)
        );
    }

    #[test]
    fn workers_rejects_zero() {
        assert!(parsed_workers(&["tempest", "test", "--workers", "0"]).is_err());
    }

    #[test]
    fn backtraces_are_only_shown_when_explicitly_requested() {
        assert!(!backtrace_requested(None));
        assert!(!backtrace_requested(Some("")));
        assert!(!backtrace_requested(Some("0")));
        assert!(backtrace_requested(Some("1")));
        assert!(backtrace_requested(Some("full")));
    }

    #[test]
    fn expected_errors_are_formatted_without_debug_backtraces() {
        let error = anyhow::anyhow!("line 3, column 7").context("invalid YAML in test.yml");
        let message = format_error(&error);

        assert_eq!(message, "error: invalid YAML in test.yml: line 3, column 7");
        assert!(!message.contains("Stack backtrace"));
    }
}
