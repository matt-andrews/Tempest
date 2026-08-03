use crate::models::descriptor::Descriptor;
use crate::models::run_options::RunOptions;
use crate::pipeline::file_runner::FileRunner;
use crate::pipeline::report_coordinator::ReportCoordinator;
use crate::templating::liquid::LiquidEngine;
use futures_util::{StreamExt, stream};
use std::collections::HashMap;
use std::future::Future;
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

    pub async fn execute(
        self,
        reports: &mut ReportCoordinator<'_>,
        workers: usize,
    ) -> anyhow::Result<()> {
        run_bounded(
            self.jobs,
            workers,
            |job| async move {
                FileRunner::new(
                    job.ordinal,
                    job.root,
                    job.envs,
                    job.base_options,
                    job.template_engine,
                    job.suite_path,
                )
                .execute()
                .await
            },
            |outcome| reports.consume(outcome),
        )
        .await
    }
}

async fn run_bounded<I, Run, RunFuture, Output, Consume>(
    items: I,
    workers: usize,
    run: Run,
    mut consume: Consume,
) -> anyhow::Result<()>
where
    I: IntoIterator,
    Run: FnMut(I::Item) -> RunFuture,
    RunFuture: Future<Output = Output>,
    Consume: FnMut(Output) -> anyhow::Result<()>,
{
    assert!(workers > 0, "file worker count must be greater than zero");

    let outcomes = stream::iter(items).map(run).buffer_unordered(workers);
    futures_util::pin_mut!(outcomes);

    while let Some(outcome) = outcomes.next().await {
        consume(outcome)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::test_spec::TestSpec;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

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

    async fn observed_max_concurrency(workers: usize) -> (usize, usize) {
        let started = Arc::new(AtomicUsize::new(0));
        let active = Arc::new(AtomicUsize::new(0));
        let max_active = Arc::new(AtomicUsize::new(0));
        let completed = Arc::new(AtomicUsize::new(0));

        run_bounded(
            0..4,
            workers,
            {
                let started = Arc::clone(&started);
                let active = Arc::clone(&active);
                let max_active = Arc::clone(&max_active);

                move |_| {
                    let started = Arc::clone(&started);
                    let active = Arc::clone(&active);
                    let max_active = Arc::clone(&max_active);

                    async move {
                        let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                        max_active.fetch_max(current, Ordering::SeqCst);
                        started.fetch_add(1, Ordering::SeqCst);

                        std::future::poll_fn(|cx| {
                            if started.load(Ordering::SeqCst) >= workers {
                                std::task::Poll::Ready(())
                            } else {
                                cx.waker().wake_by_ref();
                                std::task::Poll::Pending
                            }
                        })
                        .await;

                        active.fetch_sub(1, Ordering::SeqCst);
                    }
                }
            },
            {
                let completed = Arc::clone(&completed);
                move |_| {
                    completed.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
            },
        )
        .await
        .unwrap();

        (
            completed.load(Ordering::SeqCst),
            max_active.load(Ordering::SeqCst),
        )
    }

    #[tokio::test]
    async fn bounded_runner_respects_the_worker_limit() {
        assert_eq!(observed_max_concurrency(2).await, (4, 2));
    }

    #[tokio::test]
    async fn one_worker_keeps_file_execution_serial() {
        assert_eq!(observed_max_concurrency(1).await, (4, 1));
    }
}
