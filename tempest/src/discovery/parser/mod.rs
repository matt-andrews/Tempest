pub mod yaml_parser;

use crate::discovery::parser::yaml_parser::YamlFileParser;
use crate::models::descriptor::Descriptor;
use crate::models::run_options::RunOptions;
use crate::models::report_template::ReportTemplate;
use enum_dispatch::enum_dispatch;
use include_dir::{Dir, File};
use std::path::Path;

#[enum_dispatch]
pub trait FileParser {
    fn parse_descriptor(&self, path: &Path) -> anyhow::Result<Descriptor>;
    fn parse_config(&self, path: &Path) -> anyhow::Result<RunOptions>;
    fn parse_report_template(&self, path: &Path) -> anyhow::Result<ReportTemplate>;
    fn parse_embedded_report_template(
        &self,
        file: &File,
        dir: &Dir,
    ) -> anyhow::Result<ReportTemplate>;
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
