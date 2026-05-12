pub mod yaml_parser;

use std::path::PathBuf;
use enum_dispatch::enum_dispatch;
use crate::models::descriptor_model::DescriptorModel;
use crate::discovery::parser::yaml_parser::YamlFileParser;
use crate::models::options_model::OptionsModel;
use crate::models::report_template_model::ReportTemplateModel;

#[enum_dispatch]
pub trait FileParser{
    fn parse_descriptor(&self, path: PathBuf) -> anyhow::Result<DescriptorModel>;
    fn parse_config(&self, path: PathBuf) -> anyhow::Result<OptionsModel>;
    fn parse_report_template(&self, path: PathBuf) -> anyhow::Result<ReportTemplateModel>;
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