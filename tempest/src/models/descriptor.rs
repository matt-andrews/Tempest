use crate::models::run_options::RunOptions;
use crate::models::test_spec::TestSpec;
use crate::templating::TemplateEngine;
use crate::templating::liquid::LiquidEngine;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Descriptor {
    pub name: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,

    pub test: Option<TestSpec>,
    pub describe: Option<Vec<Descriptor>>,

    pub options: Option<RunOptions>,

    #[serde(skip)]
    pub file: Option<PathBuf>,
}

pub struct DescriptorIter<'a> {
    // Each stack entry carries the node, its parent's accumulated options, and
    // its ancestor titles so execution and reporting can share one traversal.
    // DFS (push_front + rev children) preserves source order: a section header is
    // always yielded immediately before its own tests, not after all sibling sections.
    stack: VecDeque<(&'a Descriptor, RunOptions, Vec<String>)>,
}

impl<'a> Iterator for DescriptorIter<'a> {
    type Item = (&'a Descriptor, RunOptions, Vec<String>);

    fn next(&mut self) -> Option<Self::Item> {
        let (node, parent_options, ancestor_titles) = self.stack.pop_front()?;
        let node_options = parent_options
            .clone()
            .merge(node.options.clone().unwrap_or_default());

        let mut child_titles = ancestor_titles.clone();
        if let Some(name) = node.name.as_deref().filter(|name| !name.trim().is_empty()) {
            child_titles.push(name.to_owned());
        }

        for child in node.describe.as_deref().unwrap_or_default().iter().rev() {
            self.stack
                .push_front((child, node_options.clone(), child_titles.clone()));
        }
        Some((node, parent_options, ancestor_titles))
    }
}

impl Descriptor {
    pub fn descendants(&self) -> DescriptorIter<'_> {
        DescriptorIter {
            stack: VecDeque::from([(self, RunOptions::default(), Vec::new())]),
        }
    }
    pub fn test_count(&self) -> usize {
        let own = if self.test.is_some() { 1 } else { 0 };
        let nested: usize = self.describe.iter().flatten().map(|d| d.test_count()).sum();
        own + nested
    }
    pub fn render_template(&mut self, engine: &LiquidEngine, obj: &liquid_core::Object) {
        self.name = engine.render_option_string_or_self(&self.name, obj);
        self.description = engine.render_option_string_or_self(&self.description, obj);
        self.tags = engine.render_vec_string_or_self(&self.tags, obj);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::run_options::RunOptions;

    fn options(base_uri: &str) -> RunOptions {
        RunOptions {
            base_uri: Some(base_uri.to_string()),
            debug: None,
            reports: None,
            start_time: None,
            retries: Some(0),
            concurrent: None,
        }
    }

    fn group(name: &str, opts: Option<RunOptions>, children: Vec<Descriptor>) -> Descriptor {
        Descriptor {
            name: Some(name.to_string()),
            description: None,
            tags: None,
            test: None,
            describe: Some(children),
            options: opts,
            file: None,
        }
    }

    fn leaf(name: &str, opts: Option<RunOptions>) -> Descriptor {
        Descriptor {
            name: Some(name.to_string()),
            description: None,
            tags: None,
            test: None,
            describe: None,
            options: opts,
            file: None,
        }
    }

    fn collected(root: &Descriptor) -> Vec<(String, Option<String>)> {
        root.descendants()
            .map(|(d, parent_opts, _)| (d.name.clone().unwrap_or_default(), parent_opts.base_uri))
            .collect()
    }

    #[test]
    fn options_on_root_flow_down_to_direct_child() {
        let root = group(
            "root",
            Some(options("http://root")),
            vec![leaf("child", None)],
        );

        let results = collected(&root);
        // root itself gets no parent options
        assert_eq!(results[0], ("root".to_string(), None));
        // child receives root's options as its parent context
        assert_eq!(
            results[1],
            ("child".to_string(), Some("http://root".to_string()))
        );
    }

    #[test]
    fn child_options_override_root_for_grandchildren() {
        let root = group(
            "root",
            Some(options("http://root")),
            vec![group(
                "child",
                Some(options("http://child")),
                vec![leaf("grandchild", None)],
            )],
        );

        let results = collected(&root);
        let grandchild = results.iter().find(|(n, _)| n == "grandchild").unwrap();
        // child's base_uri wins over root's for its own descendants
        assert_eq!(grandchild.1, Some("http://child".to_string()));
    }

    #[test]
    fn sibling_options_do_not_bleed_across_branches() {
        // The scenario from the question:
        //   root
        //     child
        //       grandchild-a (options: "http://a")
        //         great-grandchild (options: "http://b")  <- sibling of the node below
        //         great-grandchild-sibling               <- should see "http://a", NOT "http://b"
        //       grandchild-b                             <- should see nothing (no ancestor options)
        let root = group(
            "root",
            None,
            vec![group(
                "child",
                None,
                vec![
                    group(
                        "grandchild-a",
                        Some(options("http://a")),
                        vec![
                            leaf("ggc-b", Some(options("http://b"))),
                            leaf("ggc-sibling", None),
                        ],
                    ),
                    leaf("grandchild-b", None),
                ],
            )],
        );

        let results = collected(&root);

        // ggc-b: parent is grandchild-a, so parent_options = "http://a"
        let ggc_b = results.iter().find(|(n, _)| n == "ggc-b").unwrap();
        assert_eq!(ggc_b.1, Some("http://a".to_string()));

        // ggc-sibling: same parent (grandchild-a), same parent_options = "http://a", NOT "http://b"
        let ggc_sibling = results.iter().find(|(n, _)| n == "ggc-sibling").unwrap();
        assert_eq!(ggc_sibling.1, Some("http://a".to_string()));

        // grandchild-b: sibling of grandchild-a, parent is child (no options) — sees nothing
        let grandchild_b = results.iter().find(|(n, _)| n == "grandchild-b").unwrap();
        assert_eq!(grandchild_b.1, None);
    }

    #[test]
    fn deep_nesting_accumulates_all_ancestor_options() {
        // root (debug: true) -> a (base_uri: "http://a") -> b (no opts) -> leaf
        // leaf's parent_options should carry both debug AND base_uri from higher ancestors
        let root = group(
            "root",
            Some(RunOptions {
                base_uri: None,
                debug: Some(true),
                reports: None,
                start_time: None,
                retries: Some(0),
                concurrent: None,
            }),
            vec![group(
                "a",
                Some(options("http://a")),
                vec![group("b", None, vec![leaf("deep", None)])],
            )],
        );

        let results: Vec<(String, Option<String>, Option<bool>)> = root
            .descendants()
            .map(|(d, opts, _)| {
                (
                    d.name.clone().unwrap_or_default(),
                    opts.base_uri,
                    opts.debug,
                )
            })
            .collect();

        let deep = results.iter().find(|(n, _, _)| n == "deep").unwrap();
        assert_eq!(deep.1, Some("http://a".to_string()));
        assert_eq!(deep.2, Some(true));
    }

    #[test]
    fn traversal_is_depth_first_not_breadth_first() {
        let root = group(
            "root",
            None,
            vec![
                group(
                    "section1",
                    None,
                    vec![leaf("test1", None), leaf("test2", None)],
                ),
                group("section2", None, vec![leaf("test3", None)]),
            ],
        );

        let names: Vec<String> = root
            .descendants()
            .map(|(d, _, _)| d.name.clone().unwrap_or_default())
            .collect();

        assert_eq!(
            names,
            vec!["root", "section1", "test1", "test2", "section2", "test3"]
        );
    }

    #[test]
    fn traversal_carries_ancestor_title_paths() {
        let root = group(
            "root",
            None,
            vec![group("accounts", None, vec![leaf("creates a user", None)])],
        );

        let paths = root
            .descendants()
            .map(|(descriptor, _, titles)| (descriptor.name.clone().unwrap_or_default(), titles))
            .collect::<Vec<_>>();

        assert_eq!(paths[0], ("root".to_string(), vec![]));
        assert_eq!(paths[1], ("accounts".to_string(), vec!["root".to_string()]));
        assert_eq!(
            paths[2],
            (
                "creates a user".to_string(),
                vec!["root".to_string(), "accounts".to_string()]
            )
        );
    }

    #[test]
    fn node_own_options_are_not_included_in_yielded_parent_options() {
        // The pipeline merges ancestor_options THEN descriptor.options — so the iterator
        // must yield the parent's accumulated state, not include the current node's own options.
        let root = group(
            "root",
            Some(options("http://root")),
            vec![leaf("child", Some(options("http://child")))],
        );

        let results = collected(&root);
        let child = results.iter().find(|(n, _)| n == "child").unwrap();
        // child's yielded parent_options should be root's ("http://root"), not its own ("http://child")
        assert_eq!(child.1, Some("http://root".to_string()));
    }
}
