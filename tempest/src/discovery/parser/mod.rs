pub mod yaml_parser;

use std::path::PathBuf;
use enum_dispatch::enum_dispatch;
use crate::models::config_model::ConfigModel;
use crate::models::descriptor_model::DescriptorModel;
use crate::discovery::parser::yaml_parser::YamlFileParser;

#[enum_dispatch]
pub trait FileParser{
    fn parse_descriptor(&self, path: PathBuf) -> anyhow::Result<DescriptorModel>;
    fn parse_config(&self, path: PathBuf) -> anyhow::Result<ConfigModel>;
}

#[enum_dispatch(FileParser)]
pub enum FileParserSelector{
    YamlFileParser
}

pub fn create_parser(path: &PathBuf) -> Option<FileParserSelector> {
    let ext = path.extension()?.to_str()?;

    match ext {
        "yml" | "yaml" => Some(YamlFileParser.into()),
        _ => None,
    }
}