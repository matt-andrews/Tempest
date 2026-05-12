use std::collections::VecDeque;
use serde::{Deserialize, Serialize};
use crate::models::options_model::{OptionsModel};
use crate::models::test_model::TestModel;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DescriptorModel {
    pub name: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,

    pub test: Option<TestModel>,
    pub describe: Option<Vec<DescriptorModel>>,

    pub options: Option<OptionsModel>
}

pub struct DescriptorModelIter<'a> {
    // Each stack entry carries the node and its parent's accumulated options,
    // so the pipeline can merge the full ancestor chain without re-walking the tree.
    // DFS (push_front + rev children) preserves source order: a section header is
    // always yielded immediately before its own tests, not after all sibling sections.
    stack: VecDeque<(&'a DescriptorModel, OptionsModel)>,
}

impl<'a> Iterator for DescriptorModelIter<'a> {
    type Item = (&'a DescriptorModel, OptionsModel);

    fn next(&mut self) -> Option<Self::Item> {
        let (node, parent_options) = self.stack.pop_front()?;
        let node_options = parent_options.clone().merge(node.options.clone().unwrap_or_default());
        for child in node.describe.as_deref().unwrap_or_default().iter().rev() {
            self.stack.push_front((child, node_options.clone()));
        }
        Some((node, parent_options))
    }
}

impl DescriptorModel {
    pub(crate) fn descendants(&self) -> DescriptorModelIter<'_> {
        DescriptorModelIter { stack: VecDeque::from([(self, OptionsModel::default())]) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::options_model::OptionsModel;

    fn options(base_uri: &str) -> OptionsModel {
        OptionsModel { base_uri: Some(base_uri.to_string()), debug: None }
    }

    fn group(name: &str, opts: Option<OptionsModel>, children: Vec<DescriptorModel>) -> DescriptorModel {
        DescriptorModel { name: Some(name.to_string()), description: None, tags: None, test: None, describe: Some(children), options: opts }
    }

    fn leaf(name: &str, opts: Option<OptionsModel>) -> DescriptorModel {
        DescriptorModel { name: Some(name.to_string()), description: None, tags: None, test: None, describe: None, options: opts }
    }

    fn collected(root: &DescriptorModel) -> Vec<(String, Option<String>)> {
        root.descendants()
            .map(|(d, parent_opts)| (d.name.clone().unwrap_or_default(), parent_opts.base_uri))
            .collect()
    }

    #[test]
    fn options_on_root_flow_down_to_direct_child() {
        let root = group("root", Some(options("http://root")), vec![
            leaf("child", None),
        ]);

        let results = collected(&root);
        // root itself gets no parent options
        assert_eq!(results[0], ("root".to_string(), None));
        // child receives root's options as its parent context
        assert_eq!(results[1], ("child".to_string(), Some("http://root".to_string())));
    }

    #[test]
    fn child_options_override_root_for_grandchildren() {
        let root = group("root", Some(options("http://root")), vec![
            group("child", Some(options("http://child")), vec![
                leaf("grandchild", None),
            ]),
        ]);

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
        let root = group("root", None, vec![
            group("child", None, vec![
                group("grandchild-a", Some(options("http://a")), vec![
                    leaf("ggc-b", Some(options("http://b"))),
                    leaf("ggc-sibling", None),
                ]),
                leaf("grandchild-b", None),
            ]),
        ]);

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
        let root = group("root", Some(OptionsModel { base_uri: None, debug: Some(true) }), vec![
            group("a", Some(options("http://a")), vec![
                group("b", None, vec![
                    leaf("deep", None),
                ]),
            ]),
        ]);

        let results: Vec<(String, Option<String>, Option<bool>)> = root.descendants()
            .map(|(d, opts)| (d.name.clone().unwrap_or_default(), opts.base_uri, opts.debug))
            .collect();

        let deep = results.iter().find(|(n, _, _)| n == "deep").unwrap();
        assert_eq!(deep.1, Some("http://a".to_string()));
        assert_eq!(deep.2, Some(true));
    }

    #[test]
    fn traversal_is_depth_first_not_breadth_first() {
        // BFS would yield: root, section1, section2, test1, test2, test3
        // DFS must yield:  root, section1, test1, test2, section2, test3
        let root = group("root", None, vec![
            group("section1", None, vec![
                leaf("test1", None),
                leaf("test2", None),
            ]),
            group("section2", None, vec![
                leaf("test3", None),
            ]),
        ]);

        let names: Vec<String> = root.descendants()
            .map(|(d, _)| d.name.clone().unwrap_or_default())
            .collect();

        assert_eq!(names, vec!["root", "section1", "test1", "test2", "section2", "test3"]);
    }

    #[test]
    fn node_own_options_are_not_included_in_yielded_parent_options() {
        // The pipeline merges ancestor_options THEN descriptor.options — so the iterator
        // must yield the parent's accumulated state, not include the current node's own options.
        let root = group("root", Some(options("http://root")), vec![
            leaf("child", Some(options("http://child"))),
        ]);

        let results = collected(&root);
        let child = results.iter().find(|(n, _)| n == "child").unwrap();
        // child's yielded parent_options should be root's ("http://root"), not its own ("http://child")
        assert_eq!(child.1, Some("http://root".to_string()));
    }
}