use crate::discovery::DiscoveryResult;
use crate::models::run_options::RunOptions;
use crate::models::summary_result::SummaryResult;
use crate::pipeline::directory_runner::DirectoryRunner;
use crate::pipeline::file_scheduler::FileScheduler;
use crate::pipeline::report_coordinator::ReportCoordinator;
use crate::templating::liquid::LiquidEngine;
use std::path::Path;

pub struct PipelineRunner<'a> {
    options: RunOptions,
    discovered: &'a DiscoveryResult,
    template_engine: LiquidEngine,
    top_path: &'a Path,
    report_coordinator: ReportCoordinator<'a>,
}

impl<'a> PipelineRunner<'a> {
    pub fn new(
        discovery_result: &'a DiscoveryResult,
        default_options: RunOptions,
        top_path: &'a Path,
    ) -> Self {
        let options = merge_option_chain(&default_options, &discovery_result.directory.options);
        Self {
            options,
            discovered: discovery_result,
            template_engine: LiquidEngine,
            top_path,
            report_coordinator: ReportCoordinator::new(&discovery_result.templates),
        }
    }
    pub fn title(&self) {
        self.report_coordinator
            .title(&self.options, self.discovered.directory.test_count());
    }

    pub fn summary(&self) -> SummaryResult {
        self.report_coordinator.summary(&self.options)
    }

    pub async fn walk(&mut self) {
        let mut scheduler = FileScheduler::new();
        for directory in self.discovered.directory.walk() {
            let base_options = merge_option_chain(&self.options, &directory.options);

            let run = DirectoryRunner::new(
                base_options,
                directory,
                &self.template_engine,
                self.top_path,
            );

            run.schedule(&mut scheduler);
        }

        scheduler.execute(&mut self.report_coordinator).await;
    }
}

fn merge_option_chain(default_options: &RunOptions, options: &[RunOptions]) -> RunOptions {
    default_options.clone().merge(
        options
            .iter()
            .cloned()
            .reduce(|acc, next| acc.merge(next))
            .unwrap_or_default(),
    )
}
