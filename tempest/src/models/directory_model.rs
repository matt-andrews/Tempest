use std::collections::VecDeque;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::models::descriptor_model::DescriptorModel;
use crate::models::options_model::OptionsModel;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DirectoryModel{
    pub files: Vec<DescriptorModel>,
    pub options: Vec<OptionsModel>,
    pub children: Vec<DirectoryModel>,
    pub dir: PathBuf
}

pub struct DirectoryModelIter<'a> {
    queue: VecDeque<&'a DirectoryModel>,
}

impl<'a> Iterator for DirectoryModelIter<'a> {
    type Item = &'a DirectoryModel;

    fn next(&mut self) -> Option<Self::Item> {
        let dir = self.queue.pop_front()?;
        self.queue.extend(dir.children.iter());
        Some(dir)
    }
}

impl DirectoryModel {
    pub fn walk(&self) -> DirectoryModelIter<'_> {
        DirectoryModelIter { queue: VecDeque::from([self]) }
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
    use crate::models::descriptor_model::DescriptorModel;
    use crate::models::test_model::TestModel;

    fn empty_dir(path: &str) -> DirectoryModel {
        DirectoryModel { files: vec![], options: vec![], children: vec![], dir: PathBuf::from(path) }
    }

    fn descriptor_with_test() -> DescriptorModel {
        DescriptorModel {
            name: None,
            description: None,
            tags: None,
            test: Some(TestModel::default()),
            describe: None,
            options: None,
        }
    }

    #[test]
    fn walk_single_dir_yields_itself() {
        let root = empty_dir("root");
        let dirs: Vec<&DirectoryModel> = root.walk().collect();
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].dir, PathBuf::from("root"));
    }

    #[test]
    fn walk_is_breadth_first() {
        // root -> [child1, child2], child1 -> [grandchild]
        // BFS order: root, child1, child2, grandchild
        let grandchild = empty_dir("grandchild");
        let child1 = DirectoryModel { files: vec![], options: vec![], children: vec![grandchild], dir: PathBuf::from("child1") };
        let child2 = empty_dir("child2");
        let root = DirectoryModel { files: vec![], options: vec![], children: vec![child1, child2], dir: PathBuf::from("root") };

        let paths: Vec<&PathBuf> = root.walk().map(|d| &d.dir).collect();
        assert_eq!(paths, vec![
            &PathBuf::from("root"),
            &PathBuf::from("child1"),
            &PathBuf::from("child2"),
            &PathBuf::from("grandchild"),
        ]);
    }

    #[test]
    fn test_count_empty_dir_is_zero() {
        assert_eq!(empty_dir("root").test_count(), 0);
    }

    #[test]
    fn test_count_sums_files_and_children_recursively() {
        let child = DirectoryModel {
            files: vec![descriptor_with_test()],
            options: vec![],
            children: vec![],
            dir: PathBuf::from("child"),
        };
        let root = DirectoryModel {
            files: vec![descriptor_with_test(), descriptor_with_test()],
            options: vec![],
            children: vec![child],
            dir: PathBuf::from("root"),
        };
        assert_eq!(root.test_count(), 3);
    }
}