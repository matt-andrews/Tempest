use crate::models::run_options::RunOptions;
use crate::models::test_spec::TestSpec;
use crate::templating::TemplateEngine;
use crate::templating::liquid::LiquidEngine;
use serde::{Deserialize, Serialize};
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::collections::VecDeque;
use std::path::PathBuf;

pub type Profile = JsonMap<String, JsonValue>;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Descriptor {
    pub name: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,

    pub test: Option<TestSpec>,
    pub describe: Option<Vec<Descriptor>>,
    pub profiles: Option<Vec<Profile>>,

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
        let mut node_options = parent_options
            .clone()
            .merge(node.options.clone().unwrap_or_default());
        node_options.loop_count = None;

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
        self.execution_count() * (own + nested)
    }
    pub fn execution_count(&self) -> usize {
        let profile_count = self.profiles.as_ref().map_or(1, Vec::len);
        let loop_count = self
            .options
            .as_ref()
            .and_then(|options| options.loop_count)
            .map_or(1, |count| count.get());
        profile_count * loop_count
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
    use serde_json::json;
    use std::num::NonZeroUsize;

    fn options(base_uri: &str) -> RunOptions {
        RunOptions {
            base_uri: Some(base_uri.to_string()),
            ..RunOptions::default()
        }
    }

    fn group(name: &str, opts: Option<RunOptions>, children: Vec<Descriptor>) -> Descriptor {
        Descriptor {
            name: Some(name.to_string()),
            description: None,
            tags: None,
            test: None,
            describe: Some(children),
            profiles: None,
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
            profiles: None,
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
                debug: Some(true),
                ..RunOptions::default()
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

    #[test]
    fn descriptor_loop_is_not_propagated_as_a_child_run_option() {
        let root = group(
            "root",
            Some(RunOptions {
                loop_count: NonZeroUsize::new(2),
                ..Default::default()
            }),
            vec![leaf("child", None)],
        );

        let child_options = root
            .descendants()
            .find(|(descriptor, _, _)| descriptor.name.as_deref() == Some("child"))
            .map(|(_, options, _)| options)
            .unwrap();

        assert_eq!(child_options.loop_count, None);
    }

    #[test]
    fn test_count_includes_nested_profile_and_loop_cartesian_expansion() {
        let mut child = leaf("child", None);
        child.test = Some(TestSpec::default());
        child.profiles = Some(vec![
            Profile::from_iter([("role".to_string(), json!("admin"))]),
            Profile::from_iter([("role".to_string(), json!("reader"))]),
            Profile::from_iter([("role".to_string(), json!("author"))]),
        ]);

        let mut root = group("root", None, vec![child]);
        root.profiles = Some(vec![
            Profile::from_iter([("region".to_string(), json!("us"))]),
            Profile::from_iter([("region".to_string(), json!("eu"))]),
        ]);
        root.options = Some(RunOptions {
            loop_count: NonZeroUsize::new(2),
            ..Default::default()
        });

        assert_eq!(root.test_count(), 12);
    }
}
