pub mod assertions;
mod reporting;
pub mod runners;
pub mod templating;
pub mod variables;
mod pipeline_runner;
mod directory_runner;

use crate::discovery::DiscoveryResult;
use crate::models::run_options::RunOptions;
use crate::pipeline::pipeline_runner::PipelineRunner;

pub async fn execute(
    discovery_result: &DiscoveryResult,
    default_options: &RunOptions,
) -> anyhow::Result<()>{
    let mut run = PipelineRunner::new(discovery_result, default_options.clone());

    run.title();
    run.walk().await;
    run.summary();

    Ok(())
}