use crate::discovery::DiscoveryResult;
use crate::models::run_options::RunOptions;
use crate::models::summary_result::SummaryResult;
use crate::pipeline::directory_runner::DirectoryRunner;
use crate::pipeline::file_scheduler::FileScheduler;
use crate::pipeline::report_coordinator::ReportCoordinator;
use crate::templating::liquid::LiquidEngine;
use std::num::NonZeroUsize;
use std::path::Path;

pub struct PipelineRunner<'a> {
    options: RunOptions,
    discovered: &'a DiscoveryResult,
    template_engine: LiquidEngine,
    top_path: &'a Path,
    workers: usize,
    report_coordinator: ReportCoordinator<'a>,
}

impl<'a> PipelineRunner<'a> {
    pub fn new(
        discovery_result: &'a DiscoveryResult,
        default_options: RunOptions,
        top_path: &'a Path,
        workers: Option<NonZeroUsize>,
    ) -> Self {
        let options = merge_option_chain(&default_options, &discovery_result.directory.options);
        let workers = resolve_worker_count(options.concurrent, workers);
        Self {
            options,
            discovered: discovery_result,
            template_engine: LiquidEngine,
            top_path,
            workers,
            report_coordinator: ReportCoordinator::new(&discovery_result.templates),
        }
    }
    pub fn title(&self, start_time_ms: u128) -> anyhow::Result<()> {
        self.report_coordinator.title(
            &self.options,
            self.discovered.directory.test_count(),
            start_time_ms,
        )
    }

    pub fn summary(&self) -> anyhow::Result<SummaryResult> {
        self.report_coordinator.summary(&self.options)
    }

    pub async fn walk(&mut self) -> anyhow::Result<()> {
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

        scheduler
            .execute(&mut self.report_coordinator, self.workers)
            .await
    }
}

fn resolve_worker_count(concurrent: Option<bool>, workers: Option<NonZeroUsize>) -> usize {
    if let Some(workers) = workers {
        return workers.get();
    }

    if concurrent.unwrap_or(false) {
        std::thread::available_parallelism()
            .map(NonZeroUsize::get)
            .unwrap_or(1)
    } else {
        1
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn files_are_serial_by_default() {
        assert_eq!(resolve_worker_count(None, None), 1);
        assert_eq!(resolve_worker_count(Some(false), None), 1);
    }

    #[test]
    fn configured_concurrency_uses_available_parallelism() {
        let expected = std::thread::available_parallelism()
            .map(NonZeroUsize::get)
            .unwrap_or(1);

        assert_eq!(resolve_worker_count(Some(true), None), expected);
    }

    #[test]
    fn cli_worker_limit_overrides_configuration() {
        let workers = NonZeroUsize::new(3).unwrap();

        assert_eq!(resolve_worker_count(Some(false), Some(workers)), 3);
        assert_eq!(resolve_worker_count(Some(true), Some(workers)), 3);
    }
}
