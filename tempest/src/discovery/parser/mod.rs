pub mod yaml_parser;

use crate::discovery::parser::yaml_parser::YamlFileParser;
use crate::models::descriptor_model::DescriptorModel;
use crate::models::options_model::OptionsModel;
use crate::models::report_template_model::ReportTemplateModel;
use enum_dispatch::enum_dispatch;
use include_dir::{Dir, File};
use std::path::Path;

#[enum_dispatch]
pub trait FileParser {
    fn parse_descriptor(&self, path: &Path) -> anyhow::Result<DescriptorModel>;
    fn parse_config(&self, path: &Path) -> anyhow::Result<OptionsModel>;
    fn parse_report_template(&self, path: &Path) -> anyhow::Result<ReportTemplateModel>;
    fn parse_embedded_report_template(
        &self,
        file: &File,
        dir: &Dir,
    ) -> anyhow::Result<ReportTemplateModel>;
}

#[enum_dispatch(FileParser)]
pub enum AnyFileParser {
    YamlFileParser,
}

pub fn parser_for(path: &Path) -> Option<AnyFileParser> {
    let ext = path.extension()?.to_str()?;

    match ext {
        "yml" | "yaml" => Some(YamlFileParser.into()),
        _ => None,
    }
}
