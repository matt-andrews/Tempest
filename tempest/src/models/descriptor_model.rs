use std::collections::VecDeque;
use serde::{Deserialize, Serialize};
use crate::models::options_model::{OptionsModel};
use crate::models::test_model::TestModel;

#[derive(Debug, Deserialize, Serialize)]
pub struct DescriptorModel {
    pub name: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,

    pub test: Option<TestModel>,
    pub describe: Option<Vec<DescriptorModel>>,

    pub options: Option<OptionsModel>
}

pub struct DescriptorModelIter<'a> {
    queue: VecDeque<&'a DescriptorModel>,
}

impl<'a> Iterator for DescriptorModelIter<'a> {
    type Item = &'a DescriptorModel;

    fn next(&mut self) -> Option<Self::Item> {
        let node = self.queue.pop_front()?;
        self.queue.extend(node.describe.as_deref().unwrap_or_default().iter());
        Some(node)
    }
}

impl DescriptorModel {
    pub(crate) fn descendants(&self) -> DescriptorModelIter<'_> {
        DescriptorModelIter { queue: VecDeque::from([self]) }
    }
}