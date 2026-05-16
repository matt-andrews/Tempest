use crate::models::descriptor::Descriptor;
use crate::models::run_options::RunOptions;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DirectoryNode {
    pub files: Vec<Descriptor>,
    pub options: Vec<RunOptions>,
    pub children: Vec<DirectoryNode>,
    pub dir: PathBuf,
    pub envs: HashMap<String, String>,
}

pub struct DirectoryNodeIter<'a> {
    queue: VecDeque<&'a DirectoryNode>,
}

impl<'a> Iterator for DirectoryNodeIter<'a> {
    type Item = &'a DirectoryNode;

    fn next(&mut self) -> Option<Self::Item> {
        let dir = self.queue.pop_front()?;
        self.queue.extend(dir.children.iter());
        Some(dir)
    }
}

impl DirectoryNode {
    pub fn walk(&self) -> DirectoryNodeIter<'_> {
        DirectoryNodeIter {
            queue: VecDeque::from([self]),
        }
    }
    pub fn test_count(&self) -> usize {
        let from_files: usize = self.files.iter().map(|f| f.test_count()).sum();
        let from_children: usize = self.children.iter().map(|c| c.test_count()).sum();
        from_files + from_children
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::descriptor::Descriptor;
    use crate::models::test_spec::TestSpec;

    fn empty_dir(path: &str) -> DirectoryNode {
        DirectoryNode {
            files: vec![],
            options: vec![],
            children: vec![],
            dir: PathBuf::from(path),
            envs: HashMap::new(),
        }
    }

    fn descriptor_with_test() -> Descriptor {
        Descriptor {
            name: None,
            description: None,
            tags: None,
            test: Some(TestSpec::default()),
            describe: None,
            options: None,
            file: None,
        }
    }

    #[test]
    fn walk_single_dir_yields_itself() {
        let root = empty_dir("root");
        let dirs: Vec<&DirectoryNode> = root.walk().collect();
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].dir, PathBuf::from("root"));
    }

    #[test]
    fn walk_is_breadth_first() {
        // root -> [child1, child2], child1 -> [grandchild]
        // BFS order: root, child1, child2, grandchild
        let grandchild = empty_dir("grandchild");
        let child1 = DirectoryNode {
            files: vec![],
            options: vec![],
            children: vec![grandchild],
            dir: PathBuf::from("child1"),
            envs: HashMap::new(),
        };
        let child2 = empty_dir("child2");
        let root = DirectoryNode {
            files: vec![],
            options: vec![],
            children: vec![child1, child2],
            dir: PathBuf::from("root"),
            envs: HashMap::new(),
        };

        let paths: Vec<&PathBuf> = root.walk().map(|d| &d.dir).collect();
        assert_eq!(
            paths,
            vec![
                &PathBuf::from("root"),
                &PathBuf::from("child1"),
                &PathBuf::from("child2"),
                &PathBuf::from("grandchild"),
            ]
        );
    }

    #[test]
    fn test_count_empty_dir_is_zero() {
        assert_eq!(empty_dir("root").test_count(), 0);
    }

    #[test]
    fn test_count_sums_files_and_children_recursively() {
        let child = DirectoryNode {
            files: vec![descriptor_with_test()],
            options: vec![],
            children: vec![],
            dir: PathBuf::from("child"),
            envs: HashMap::new(),
        };
        let root = DirectoryNode {
            files: vec![descriptor_with_test(), descriptor_with_test()],
            options: vec![],
            children: vec![child],
            dir: PathBuf::from("root"),
            envs: HashMap::new(),
        };
        assert_eq!(root.test_count(), 3);
    }
}
