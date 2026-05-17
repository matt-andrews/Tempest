use crate::discovery::DiscoveryResult;
use crate::models::run_options::RunOptions;
use crate::models::summary_result::SummaryResult;
use crate::pipeline::directory_runner::DirectoryRunner;
use crate::pipeline::reporting::{AnyReporter, Reporter, reporter_for};
use crate::pipeline::templating::liquid::LiquidEngine;
use std::path::Path;

pub struct PipelineRunner<'a> {
    reporter: AnyReporter,
    options: RunOptions,
    discovered: &'a DiscoveryResult,
    summary: Vec<SummaryResult>,
    template_engine: LiquidEngine,
    top_path: &'a Path,
}

impl<'a> PipelineRunner<'a> {
    pub fn new(
        discovery_result: &'a DiscoveryResult,
        default_options: RunOptions,
        top_path: &'a Path,
    ) -> Self {
        let options = merge_option_chain(&default_options, &discovery_result.directory.options);
        Self {
            reporter: reporter_for(),
            options,
            discovered: discovery_result,
            summary: Vec::new(),
            template_engine: LiquidEngine,
            top_path,
        }
    }
    pub fn title(&self) {
        self.reporter.title(
            &self.options,
            &self.discovered.templates,
            self.discovered.directory.test_count(),
        );
    }

    pub fn summary(&self) {
        self.reporter
            .summary(&self.options, &self.discovered.templates, &self.summary);
    }

    pub async fn walk(&mut self) {
        for directory in self.discovered.directory.walk() {
            let base_options = merge_option_chain(&self.options, &directory.options);

            let run = DirectoryRunner::new(
                base_options,
                directory,
                &self.template_engine,
                &self.discovered.templates,
                &self.reporter,
                self.top_path,
            );

            run.execute_dir(&mut self.summary).await;
        }
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
