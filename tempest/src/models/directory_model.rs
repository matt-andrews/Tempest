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