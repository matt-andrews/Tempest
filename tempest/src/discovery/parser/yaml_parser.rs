use std::fs;
use std::path::PathBuf;
use crate::models::config_model::ConfigModel;
use crate::models::descriptor_model::DescriptorModel;
use crate::discovery::parser::FileParser;

pub struct YamlFileParser;
impl FileParser for YamlFileParser{
    fn parse_descriptor(&self, path: PathBuf) -> anyhow::Result<DescriptorModel> {
        let contents = fs::read_to_string(path)?;
        let config: DescriptorModel = serde_yml::from_str(&contents)?;
        Ok(config)
    }

    fn parse_config(&self, path: PathBuf) -> anyhow::Result<ConfigModel> {
        let contents = fs::read_to_string(path)?;
        let config: ConfigModel = serde_yml::from_str(&contents)?;
        Ok(config)
    }
}