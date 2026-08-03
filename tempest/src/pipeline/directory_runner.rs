use crate::models::directory_node::DirectoryNode;
use crate::models::run_options::RunOptions;
use crate::pipeline::file_scheduler::FileScheduler;
use crate::templating::liquid::LiquidEngine;
use std::path::Path;

pub struct DirectoryRunner<'a> {
    directory: &'a DirectoryNode,
    template_engine: &'a LiquidEngine,
    base_options: RunOptions,
    top_path: &'a Path,
}

impl<'a> DirectoryRunner<'a> {
    pub fn new(
        base_options: RunOptions,
        directory: &'a DirectoryNode,
        template_engine: &'a LiquidEngine,
        top_path: &'a Path,
    ) -> Self {
        Self {
            base_options,
            directory,
            template_engine,
            top_path,
        }
    }

    pub fn schedule(&self, scheduler: &mut FileScheduler<'a>) {
        for root in &self.directory.files {
            scheduler.schedule(
                root,
                &self.directory.envs,
                self.base_options.clone(),
                self.template_engine,
                self.top_path,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::descriptor::Descriptor;
    use crate::models::directory_node::DirectoryNode;
    use crate::models::report_template::{ReportFile, ReportTemplate};
    use crate::models::run_options::RunOptions;
    use crate::models::summary_result::SummaryResult;
    use crate::models::test_spec::TestSpec;
    use crate::pipeline::report_coordinator::ReportCoordinator;
    use crate::templating::liquid::LiquidEngine;
    use mockito::Server;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn dir_node(files: Vec<Descriptor>) -> DirectoryNode {
        DirectoryNode {
            files,
            options: vec![],
            children: vec![],
            dir: PathBuf::from("test"),
            envs: HashMap::new(),
        }
    }

    fn dir_node_with_envs(files: Vec<Descriptor>, envs: HashMap<String, String>) -> DirectoryNode {
        DirectoryNode {
            files,
            options: vec![],
            children: vec![],
            dir: PathBuf::from("test"),
            envs,
        }
    }

    fn make_runner<'a>(
        directory: &'a DirectoryNode,
        engine: &'a LiquidEngine,
        base_options: RunOptions,
    ) -> DirectoryRunner<'a> {
        DirectoryRunner::new(base_options, directory, engine, Path::new("./"))
    }

    async fn execute_runner(
        runner: DirectoryRunner<'_>,
        templates: &HashMap<String, ReportTemplate>,
    ) -> Vec<SummaryResult> {
        let mut reports = ReportCoordinator::new(templates);
        let mut scheduler = FileScheduler::new();
        runner.schedule(&mut scheduler);
        scheduler.execute(&mut reports).await;
        reports.results().to_vec()
    }

    fn get_descriptor(route: &str, assertions: Option<Vec<&str>>) -> Descriptor {
        Descriptor {
            name: Some("test".to_string()),
            description: None,
            tags: None,
            test: Some(TestSpec {
                route: route.to_string(),
                verb: Some("GET".to_string()),
                assert: assertions.map(|a| a.iter().map(|s| s.to_string()).collect()),
                ..Default::default()
            }),
            describe: None,
            options: None,
            file: None,
        }
    }

    #[tokio::test]
    async fn descriptor_without_test_produces_no_summary_result() {
        let engine = LiquidEngine;
        let templates = HashMap::new();
        let dir = dir_node(vec![Descriptor {
            name: Some("no-test".to_string()),
            description: None,
            tags: None,
            test: None,
            describe: None,
            options: None,
            file: None,
        }]);

        let summary = execute_runner(
            make_runner(&dir, &engine, RunOptions::default()),
            &templates,
        )
        .await;

        assert!(summary.is_empty());
    }

    #[tokio::test]
    async fn passing_assertion_yields_passed_summary() {
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/ok")
            .with_status(200)
            .create_async()
            .await;

        let engine = LiquidEngine;
        let templates = HashMap::new();
        let dir = dir_node(vec![get_descriptor(
            &format!("{}/ok", server.url()),
            Some(vec!["status == 200"]),
        )]);

        let summary = execute_runner(
            make_runner(&dir, &engine, RunOptions::default()),
            &templates,
        )
        .await;

        assert_eq!(summary.len(), 1);
        assert!(matches!(summary[0], SummaryResult::Passed));
    }

    #[tokio::test]
    async fn failing_assertion_yields_failed_summary() {
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/ok")
            .with_status(200)
            .create_async()
            .await;

        let engine = LiquidEngine;
        let templates = HashMap::new();
        let dir = dir_node(vec![get_descriptor(
            &format!("{}/ok", server.url()),
            Some(vec!["status == 404"]),
        )]);

        let summary = execute_runner(
            make_runner(&dir, &engine, RunOptions::default()),
            &templates,
        )
        .await;

        assert_eq!(summary.len(), 1);
        assert!(matches!(summary[0], SummaryResult::Failed));
    }

    #[tokio::test]
    async fn test_without_assertions_yields_passed_summary() {
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/ok")
            .with_status(200)
            .create_async()
            .await;

        let engine = LiquidEngine;
        let templates = HashMap::new();
        let dir = dir_node(vec![get_descriptor(&format!("{}/ok", server.url()), None)]);

        let summary = execute_runner(
            make_runner(&dir, &engine, RunOptions::default()),
            &templates,
        )
        .await;

        assert_eq!(summary.len(), 1);
        assert!(matches!(summary[0], SummaryResult::Passed));
    }

    #[tokio::test]
    async fn multiple_descriptors_produce_multiple_summary_entries() {
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/a")
            .with_status(200)
            .create_async()
            .await;
        server
            .mock("GET", "/b")
            .with_status(404)
            .create_async()
            .await;

        let engine = LiquidEngine;
        let templates = HashMap::new();
        let dir = dir_node(vec![
            get_descriptor(&format!("{}/a", server.url()), Some(vec!["status == 200"])),
            get_descriptor(&format!("{}/b", server.url()), Some(vec!["status == 200"])),
        ]);

        let summary = execute_runner(
            make_runner(&dir, &engine, RunOptions::default()),
            &templates,
        )
        .await;

        assert_eq!(summary.len(), 2);
        assert!(matches!(summary[0], SummaryResult::Passed));
        assert!(matches!(summary[1], SummaryResult::Failed));
    }

    #[tokio::test]
    async fn base_uri_is_prepended_to_relative_route() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/items")
            .with_status(200)
            .create_async()
            .await;

        let engine = LiquidEngine;
        let templates = HashMap::new();
        let dir = dir_node(vec![get_descriptor("/items", Some(vec!["status == 200"]))]);

        let summary = execute_runner(
            make_runner(
                &dir,
                &engine,
                RunOptions {
                    base_uri: Some(server.url()),
                    ..Default::default()
                },
            ),
            &templates,
        )
        .await;

        assert_eq!(summary.len(), 1);
        assert!(matches!(summary[0], SummaryResult::Passed));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn descriptor_options_override_base_options() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/path")
            .with_status(200)
            .create_async()
            .await;

        let engine = LiquidEngine;
        let templates = HashMap::new();
        let dir = dir_node(vec![Descriptor {
            name: Some("override".to_string()),
            description: None,
            tags: None,
            test: Some(TestSpec {
                route: "/path".to_string(),
                verb: Some("GET".to_string()),
                assert: Some(vec!["status == 200".to_string()]),
                ..Default::default()
            }),
            describe: None,
            options: Some(RunOptions {
                base_uri: Some(server.url()),
                ..Default::default()
            }),
            file: None,
        }]);

        // base_options has a bogus URI; descriptor.options has the real one
        let summary = execute_runner(
            make_runner(
                &dir,
                &engine,
                RunOptions {
                    base_uri: Some("http://127.0.0.1:1".to_string()),
                    ..Default::default()
                },
            ),
            &templates,
        )
        .await;

        assert_eq!(summary.len(), 1);
        assert!(matches!(summary[0], SummaryResult::Passed));
        mock.assert_async().await;
    }

    fn spec_file(children: Vec<Descriptor>) -> Descriptor {
        Descriptor {
            name: Some("test spec".to_string()),
            description: None,
            tags: None,
            test: None,
            describe: Some(children),
            options: None,
            file: Some(PathBuf::from("test.spec.yml")),
        }
    }

    #[tokio::test]
    async fn variable_set_in_one_test_is_available_in_next() {
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/seed")
            .with_status(200)
            .with_body("42")
            .create_async()
            .await;
        let mock_consumer = server
            .mock("GET", "/items/42")
            .with_status(200)
            .create_async()
            .await;

        let engine = LiquidEngine;
        let templates = HashMap::new();
        let seed = Descriptor {
            name: Some("seed".to_string()),
            description: None,
            tags: None,
            test: Some(TestSpec {
                route: format!("{}/seed", server.url()),
                verb: Some("GET".to_string()),
                vars: Some(HashMap::from([("id".to_string(), "body".to_string())])),
                ..Default::default()
            }),
            describe: None,
            options: None,
            file: None,
        };
        let consumer = Descriptor {
            name: Some("consumer".to_string()),
            description: None,
            tags: None,
            test: Some(TestSpec {
                route: format!("{}/items/{{{{ file.id }}}}", server.url()),
                verb: Some("GET".to_string()),
                assert: Some(vec!["status == 200".to_string()]),
                ..Default::default()
            }),
            describe: None,
            options: None,
            file: None,
        };

        let dir = dir_node(vec![spec_file(vec![seed, consumer])]);
        let summary = execute_runner(
            make_runner(&dir, &engine, RunOptions::default()),
            &templates,
        )
        .await;

        assert_eq!(summary.len(), 2);
        assert!(matches!(summary[1], SummaryResult::Passed));
        mock_consumer.assert_async().await;
    }

    #[tokio::test]
    async fn env_vars_are_available_in_route_templates() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/api")
            .with_status(200)
            .create_async()
            .await;

        let engine = LiquidEngine;
        let templates = HashMap::new();
        let mut envs = HashMap::new();
        envs.insert("HOST".to_string(), server.url());

        let dir = dir_node_with_envs(
            vec![Descriptor {
                name: Some("env-test".to_string()),
                description: None,
                tags: None,
                test: Some(TestSpec {
                    route: "{{ env.HOST }}/api".to_string(),
                    verb: Some("GET".to_string()),
                    assert: Some(vec!["status == 200".to_string()]),
                    ..Default::default()
                }),
                describe: None,
                options: None,
                file: None,
            }],
            envs,
        );

        let summary = execute_runner(
            make_runner(&dir, &engine, RunOptions::default()),
            &templates,
        )
        .await;

        assert_eq!(summary.len(), 1);
        assert!(matches!(summary[0], SummaryResult::Passed));
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn retry_pass_after_failure_yields_single_flaky_summary() {
        let mut server = Server::new_async().await;
        let fail = server
            .mock("GET", "/unstable")
            .with_status(500)
            .expect(1)
            .create_async()
            .await;
        let pass = server
            .mock("GET", "/unstable")
            .with_status(200)
            .expect(1)
            .create_async()
            .await;

        let engine = LiquidEngine;
        let templates = HashMap::new();
        let dir = dir_node(vec![get_descriptor(
            &format!("{}/unstable", server.url()),
            Some(vec!["status == 200"]),
        )]);

        let summary = execute_runner(
            make_runner(
                &dir,
                &engine,
                RunOptions {
                    retries: Some(1),
                    ..Default::default()
                },
            ),
            &templates,
        )
        .await;

        assert_eq!(summary.len(), 1);
        assert!(matches!(summary[0], SummaryResult::Flaky));
        fail.assert_async().await;
        pass.assert_async().await;
    }

    #[tokio::test]
    async fn retry_exhaustion_yields_single_failed_summary() {
        let mut server = Server::new_async().await;
        let fail = server
            .mock("GET", "/down")
            .with_status(500)
            .expect(3)
            .create_async()
            .await;

        let engine = LiquidEngine;
        let templates = HashMap::new();
        let dir = dir_node(vec![get_descriptor(
            &format!("{}/down", server.url()),
            Some(vec!["status == 200"]),
        )]);

        let summary = execute_runner(
            make_runner(
                &dir,
                &engine,
                RunOptions {
                    retries: Some(2),
                    ..Default::default()
                },
            ),
            &templates,
        )
        .await;

        assert_eq!(summary.len(), 1);
        assert!(matches!(summary[0], SummaryResult::Failed));
        fail.assert_async().await;
    }

    #[tokio::test]
    async fn retry_attempts_are_reported_but_summary_counts_descriptor_once() {
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/reported")
            .with_status(500)
            .expect(1)
            .create_async()
            .await;
        server
            .mock("GET", "/reported")
            .with_status(200)
            .expect(1)
            .create_async()
            .await;

        let report_dir = tempfile::tempdir().unwrap();
        let report_path = report_dir.path().join("report.txt");
        let mut templates = HashMap::new();
        templates.insert(
            "file".to_string(),
            ReportTemplate {
                test_template: Some("{{ status }}\n".to_string()),
                section_template: None,
                error_template: None,
                debug_template: None,
                title_template: None,
                summary_template: None,
                file: Some(ReportFile {
                    dir: Some(report_dir.path().to_path_buf()),
                    file_name: Some("report.txt".to_string()),
                }),
            },
        );

        let engine = LiquidEngine;
        let dir = dir_node(vec![get_descriptor(
            &format!("{}/reported", server.url()),
            Some(vec!["status == 200"]),
        )]);

        let summary = execute_runner(
            make_runner(
                &dir,
                &engine,
                RunOptions {
                    reports: Some(vec!["file".to_string()]),
                    retries: Some(1),
                    ..Default::default()
                },
            ),
            &templates,
        )
        .await;

        let report = std::fs::read_to_string(report_path).unwrap();

        assert_eq!(summary.len(), 1);
        assert!(matches!(summary[0], SummaryResult::Flaky));
        assert_eq!(report.lines().collect::<Vec<_>>(), vec!["500", "200"]);
    }

    #[tokio::test]
    async fn failed_retry_attempts_do_not_mutate_context_for_next_attempt() {
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/seed")
            .with_status(200)
            .with_body("start")
            .expect(1)
            .create_async()
            .await;
        server
            .mock("GET", "/unstable/start")
            .with_status(500)
            .with_body("bad")
            .expect(1)
            .create_async()
            .await;
        server
            .mock("GET", "/unstable/start")
            .with_status(200)
            .with_body("good")
            .expect(1)
            .create_async()
            .await;
        let consumer = server
            .mock("GET", "/items/good")
            .with_status(200)
            .expect(1)
            .create_async()
            .await;

        let engine = LiquidEngine;
        let templates = HashMap::new();
        let seed = Descriptor {
            name: Some("seed".to_string()),
            description: None,
            tags: None,
            test: Some(TestSpec {
                route: format!("{}/seed", server.url()),
                verb: Some("GET".to_string()),
                assert: Some(vec!["status == 200".to_string()]),
                vars: Some(HashMap::from([("token".to_string(), "body".to_string())])),
                ..Default::default()
            }),
            describe: None,
            options: None,
            file: None,
        };
        let unstable = Descriptor {
            name: Some("unstable".to_string()),
            description: None,
            tags: None,
            test: Some(TestSpec {
                route: format!("{}/unstable/{{{{ file.token }}}}", server.url()),
                verb: Some("GET".to_string()),
                assert: Some(vec!["status == 200".to_string()]),
                vars: Some(HashMap::from([("token".to_string(), "body".to_string())])),
                ..Default::default()
            }),
            describe: None,
            options: None,
            file: None,
        };
        let consumer_descriptor = Descriptor {
            name: Some("consumer".to_string()),
            description: None,
            tags: None,
            test: Some(TestSpec {
                route: format!("{}/items/{{{{ file.token }}}}", server.url()),
                verb: Some("GET".to_string()),
                assert: Some(vec!["status == 200".to_string()]),
                ..Default::default()
            }),
            describe: None,
            options: None,
            file: None,
        };

        let dir = dir_node(vec![spec_file(vec![seed, unstable, consumer_descriptor])]);
        let summary = execute_runner(
            make_runner(
                &dir,
                &engine,
                RunOptions {
                    retries: Some(1),
                    ..Default::default()
                },
            ),
            &templates,
        )
        .await;

        assert_eq!(summary.len(), 3);
        assert!(matches!(summary[0], SummaryResult::Passed));
        assert!(matches!(summary[1], SummaryResult::Flaky));
        assert!(matches!(summary[2], SummaryResult::Passed));
        consumer.assert_async().await;
    }

    #[tokio::test]
    async fn file_context_resets_when_file_changes() {
        // file-a sets file.token via vars; file-b is in a different file so its
        // context is reset — it hits an unrelated route with no template variables.
        // The companion test `variable_set_in_one_test_is_available_in_next` proves
        // that within the same file the variable DOES flow; this test proves that
        // crossing a file boundary produces a clean context.
        let mut server = Server::new_async().await;
        server
            .mock("GET", "/file-a")
            .with_status(200)
            .with_body("secret")
            .create_async()
            .await;
        let mock_b = server
            .mock("GET", "/file-b")
            .with_status(200)
            .create_async()
            .await;

        let engine = LiquidEngine;
        let templates = HashMap::new();
        let file_a = Descriptor {
            name: Some("file-a".to_string()),
            description: None,
            tags: None,
            test: Some(TestSpec {
                route: format!("{}/file-a", server.url()),
                verb: Some("GET".to_string()),
                vars: Some(HashMap::from([("token".to_string(), "body".to_string())])),
                ..Default::default()
            }),
            describe: None,
            options: None,
            file: Some(PathBuf::from("file-a.yaml")),
        };
        let file_b = Descriptor {
            name: Some("file-b".to_string()),
            description: None,
            tags: None,
            test: Some(TestSpec {
                route: format!("{}/file-b", server.url()),
                verb: Some("GET".to_string()),
                assert: Some(vec!["status == 200".to_string()]),
                ..Default::default()
            }),
            describe: None,
            options: None,
            file: Some(PathBuf::from("file-b.yaml")),
        };

        let dir = dir_node(vec![file_a, file_b]);
        let summary = execute_runner(
            make_runner(&dir, &engine, RunOptions::default()),
            &templates,
        )
        .await;

        assert_eq!(summary.len(), 2);
        assert!(matches!(summary[0], SummaryResult::Passed));
        assert!(matches!(summary[1], SummaryResult::Passed));
        mock_b.assert_async().await;
    }
}
