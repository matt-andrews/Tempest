pub mod assertions;
mod directory_runner;
mod pipeline_runner;
mod reporting;
pub mod runners;
pub mod templating;
pub mod variables;
pub mod warnings;

use crate::discovery::DiscoveryResult;
use crate::models::run_options::RunOptions;
use crate::models::summary_result::SummaryResult;
use crate::pipeline::pipeline_runner::PipelineRunner;

pub async fn execute(
    discovery_result: &DiscoveryResult,
    default_options: &RunOptions,
) -> anyhow::Result<SummaryResult> {
    let mut run = PipelineRunner::new(
        discovery_result,
        default_options.clone(),
        discovery_result.directory.dir.as_path(),
    );

    run.title();
    run.walk().await;
    let result = run.summary();

    Ok(result)
}
