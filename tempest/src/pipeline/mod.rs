pub mod assertions;
mod directory_runner;
mod file_runner;
mod file_scheduler;
mod pipeline_runner;
mod report_coordinator;
mod reporting;
pub mod runners;
pub mod variables;
pub mod warnings;

use crate::discovery::DiscoveryResult;
use crate::models::run_options::RunOptions;
use crate::models::summary_result::SummaryResult;
use crate::pipeline::pipeline_runner::PipelineRunner;
use std::num::NonZeroUsize;
use std::time::Instant;

pub async fn execute(
    discovery_result: &DiscoveryResult,
    default_options: &RunOptions,
    workers: Option<NonZeroUsize>,
    start_instant: &Instant,
) -> anyhow::Result<SummaryResult> {
    let mut run = PipelineRunner::new(
        discovery_result,
        default_options.clone(),
        discovery_result.directory.dir.as_path(),
        workers,
    );

    run.title(start_instant.elapsed().as_millis())?;
    run.walk().await?;
    let result = run.summary()?;

    Ok(result)
}
