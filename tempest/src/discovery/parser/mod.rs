pub mod yaml_parser;

use std::path::PathBuf;
use enum_dispatch::enum_dispatch;
use include_dir::{Dir, File};
use crate::models::descriptor_model::DescriptorModel;
use crate::discovery::parser::yaml_parser::YamlFileParser;
use crate::models::options_model::OptionsModel;
use crate::models::report_template_model::ReportTemplateModel;

#[enum_dispatch]
pub trait FileParser{
    fn parse_descriptor(&self, path: PathBuf) -> anyhow::Result<DescriptorModel>;
    fn parse_config(&self, path: PathBuf) -> anyhow::Result<OptionsModel>;
    fn parse_report_template(&self, path: PathBuf) -> anyhow::Result<ReportTemplateModel>;
    fn parse_embedded_report_template(&self, file: &File, dir: &Dir) -> anyhow::Result<ReportTemplateModel>;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn create_parser_returns_some_for_yml() {
        assert!(create_parser(&PathBuf::from("anything.yml")).is_some());
    }

    #[test]
    fn create_parser_returns_some_for_yaml() {
        assert!(create_parser(&PathBuf::from("anything.yaml")).is_some());
    }

    #[test]
    fn create_parser_returns_none_for_json() {
        assert!(create_parser(&PathBuf::from("anything.json")).is_none());
    }

    #[test]
    fn create_parser_returns_none_for_toml() {
        assert!(create_parser(&PathBuf::from("config.toml")).is_none());
    }

    #[test]
    fn create_parser_returns_none_for_no_extension() {
        assert!(create_parser(&PathBuf::from("noextension")).is_none());
    }

    #[test]
    fn create_parser_returns_none_for_txt() {
        assert!(create_parser(&PathBuf::from("readme.txt")).is_none());
    }

    #[test]
    fn create_parser_works_with_full_path() {
        assert!(create_parser(&PathBuf::from("/some/nested/path/test.yml")).is_some());
        assert!(create_parser(&PathBuf::from("/some/nested/path/test.json")).is_none());
    }

    #[test]
    fn create_parser_yml_and_yaml_produce_same_parser_type() {
        let yml = create_parser(&PathBuf::from("test.yml")).unwrap();
        let yaml = create_parser(&PathBuf::from("test.yaml")).unwrap();
        // Both should successfully parse the same YAML content - verified by discriminant
        assert!(matches!(yml, FileParserSelector::YamlFileParser(_)));
        assert!(matches!(yaml, FileParserSelector::YamlFileParser(_)));
    }
}