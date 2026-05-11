use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::models::config_model::ConfigModel;
use crate::models::descriptor_model::DescriptorModel;

#[derive(Debug, Deserialize, Serialize)]
pub struct DirectoryModel{
    pub files: Vec<DescriptorModel>,
    pub configs: Vec<ConfigModel>,
    pub children: Vec<DirectoryModel>,
    pub dir: PathBuf
}