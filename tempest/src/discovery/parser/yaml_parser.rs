use std::fs;
use std::path::PathBuf;
use crate::models::descriptor_model::DescriptorModel;
use crate::discovery::parser::FileParser;
use crate::models::options_model::OptionsModel;
use crate::models::report_template_model::ReportTemplateModel;

pub struct YamlFileParser;
impl FileParser for YamlFileParser{
    fn parse_descriptor(&self, path: PathBuf) -> anyhow::Result<DescriptorModel> {
        let contents = fs::read_to_string(path)?;
        let config: DescriptorModel = serde_yml::from_str(&contents)?;
        Ok(config)
    }

    fn parse_config(&self, path: PathBuf) -> anyhow::Result<OptionsModel> {
        let contents = fs::read_to_string(path)?;
        let config: OptionsModel = serde_yml::from_str(&contents)?;
        Ok(config)
    }

    fn parse_report_template(&self, path: PathBuf) -> anyhow::Result<ReportTemplateModel> {
        let contents = fs::read_to_string(path)?;
        let config: ReportTemplateModel = serde_yml::from_str(&contents)?;
        Ok(config)
    }
}