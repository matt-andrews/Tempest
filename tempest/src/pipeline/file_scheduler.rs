use crate::models::descriptor::Descriptor;
use crate::models::run_options::RunOptions;
use crate::pipeline::file_runner::FileRunner;
use crate::pipeline::report_coordinator::ReportCoordinator;
use crate::templating::liquid::LiquidEngine;
use std::collections::HashMap;
use std::path::Path;

pub struct FileJob<'a> {
    ordinal: usize,
    root: &'a Descriptor,
    envs: &'a HashMap<String, String>,
    base_options: RunOptions,
    template_engine: &'a LiquidEngine,
    suite_path: &'a Path,
}

pub struct FileScheduler<'a> {
    jobs: Vec<FileJob<'a>>,
}

impl<'a> FileScheduler<'a> {
    pub fn new() -> Self {
        Self { jobs: Vec::new() }
    }

    pub fn schedule(
        &mut self,
        root: &'a Descriptor,
        envs: &'a HashMap<String, String>,
        base_options: RunOptions,
        template_engine: &'a LiquidEngine,
        suite_path: &'a Path,
    ) {
        let ordinal = self.jobs.len();
        self.jobs.push(FileJob {
            ordinal,
            root,
            envs,
            base_options,
            template_engine,
            suite_path,
        });
    }

    pub async fn execute(self, reports: &mut ReportCoordinator<'_>) {
        for job in self.jobs {
            let outcome = FileRunner::new(
                job.ordinal,
                job.root,
                job.envs,
                job.base_options,
                job.template_engine,
                job.suite_path,
            )
            .execute()
            .await;

            reports.consume(outcome);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::test_spec::TestSpec;
    use std::path::PathBuf;

    fn root(path: &str) -> Descriptor {
        Descriptor {
            name: Some(path.to_string()),
            description: None,
            tags: None,
            test: Some(TestSpec::default()),
            describe: None,
            options: None,
            file: Some(PathBuf::from(path)),
        }
    }

    #[test]
    fn scheduling_assigns_contiguous_discovery_ordinals() {
        let roots = [root("b.spec.yml"), root("a.spec.yml")];
        let envs = HashMap::new();
        let engine = LiquidEngine;
        let suite_path = Path::new(".");
        let mut scheduler = FileScheduler::new();

        for root in &roots {
            scheduler.schedule(root, &envs, RunOptions::default(), &engine, suite_path);
        }

        assert_eq!(
            scheduler
                .jobs
                .iter()
                .map(|job| (job.ordinal, job.root.file.as_deref()))
                .collect::<Vec<_>>(),
            vec![
                (0, Some(Path::new("b.spec.yml"))),
                (1, Some(Path::new("a.spec.yml"))),
            ]
        );
    }
}
