use std::path::PathBuf;
use crate::discovery::parser::{FileParser};
use crate::models::config_model::ConfigModel;
use crate::models::directory_model::DirectoryModel;
use crate::models::descriptor_model::DescriptorModel;

mod parser;

pub fn discover(dir: PathBuf, inherited_configs: Option<Vec<ConfigModel>>) -> anyhow::Result<DirectoryModel> {
    let mut tests: Vec<DescriptorModel> = Vec::new();
    let mut configs: Vec<ConfigModel> = inherited_configs.unwrap_or_default();
    let mut subdirectories: Vec<DirectoryModel> = Vec::new();

    let entries = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok());

    for entry in entries {
        let path = entry.path();
        let file_name = path.file_stem();
        let file_name = file_name.unwrap().to_str().unwrap();

        if path.is_dir() {
            let sub_dir = discover(path, Some(configs.clone()))?;
            subdirectories.push(sub_dir);
        } else if path.is_file() {
            if let Some(parser) = parser::create_parser(&path){
                if file_name.ends_with(".spec") {
                    let test = parser.parse_descriptor(path)?;
                    tests.push(test);
                } else if file_name.ends_with(".config") {
                    let config = parser.parse_config(path)?;
                    configs.push(config);
                }
            }
        }
    }

    Ok(DirectoryModel {
        files: tests,
        configs,
        children: subdirectories,
        dir,
    })
}