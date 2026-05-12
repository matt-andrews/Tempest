pub mod runner;
pub mod expr_parser;

use colored::Colorize;
use crate::engine::runner::capabilities::{create_capabilities, RunnerCapability};
use crate::models::directory_model::DirectoryModel;
use crate::models::options_model::OptionsModel;
use crate::models::run_result::RunResult;

pub async fn execute(discovered: DirectoryModel, default_options: OptionsModel) -> anyhow::Result<()>{
    let capabilities = &create_capabilities().await;
    println!("\n\n{}", "Starting tests...".green());

    for directory in discovered.walk() {
        let base_options = default_options
            .clone()
            .merge(directory.options.iter()
                .cloned()
                .reduce(|acc, next| acc.merge(next))
                .unwrap_or_default()
            );

        for descriptor in directory.files.iter().flat_map(|m| m.descendants()) {
            let mut context = RunResult::default();

            let mut options = descriptor.options.clone().unwrap_or_default();
            options = base_options.clone().merge(options);

            for capability in capabilities {
                let result = capability.run(descriptor, &context, &options).await;
                if result.stop { break; }
                context = result;
            }
        }
    }

    Ok(())
}