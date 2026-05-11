pub mod runner;
pub mod expr_parser;

use async_recursion::async_recursion;
use colored::Colorize;
use crate::engine::runner::{RunnerCapability};
use crate::models::directory_model::DirectoryModel;
use crate::models::run_result::RunResult;

pub async fn execute(discovered: DirectoryModel) -> anyhow::Result<()>{
    let capabilities = runner::create_capabilities().await;
    println!("\n\n{}", "Starting tests...".green());
    execute_recurse(&discovered, &capabilities).await
}

#[async_recursion]
async fn execute_recurse(
    discovered: &DirectoryModel,
    capabilities: &Vec<Box<dyn RunnerCapability>>
) -> anyhow::Result<()> {

    //run through every descriptor + child descriptors all the way down
    for descriptor in discovered.files.iter().flat_map(|m| m.descendants()){
        let mut context: Option<RunResult> = None;
        for rule in capabilities.iter() {
            let result = rule.run(descriptor, context).await;
            if !result.success {
                break;
            }
            context = Some(result);
        }
    }

    //run through all child directories
    for child in &discovered.children{
        _ = execute_recurse(&child, capabilities).await;
    }

    Ok(())
}