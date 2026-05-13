use std::fs;
use std::path::PathBuf;
use include_dir::{include_dir, Dir, File};
use crate::models::descriptor_model::DescriptorModel;
use crate::discovery::parser::FileParser;
use crate::discovery::{BUILTIN_REPORTERS};
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
        let contents = fs::read_to_string(path.clone())?;
        let mut template: ReportTemplateModel = serde_yml::from_str(&contents)?;

        let parent = path.parent().map(PathBuf::from).unwrap_or_default();

        template.test_template    = Self::resolve_liquid_ref(template.test_template, &parent);
        template.section_template = Self::resolve_liquid_ref(template.section_template, &parent);
        template.error_template = Self::resolve_liquid_ref(template.error_template, &parent);
        template.title_template = Self::resolve_liquid_ref(template.title_template, &parent);
        template.summary_template = Self::resolve_liquid_ref(template.summary_template, &parent);

        Ok(template)
    }

    fn parse_embedded_report_template(&self, file: &File, dir: &Dir) -> anyhow::Result<ReportTemplateModel>{
        let contents = file.contents_utf8().unwrap_or_default();
        let mut template = serde_yml::from_str::<ReportTemplateModel>(contents)?;

        template.test_template    = Self::resolve_embedded_liquid(template.test_template,    dir);
        template.section_template = Self::resolve_embedded_liquid(template.section_template, dir);
        template.error_template   = Self::resolve_embedded_liquid(template.error_template,   dir);
        template.title_template   = Self::resolve_embedded_liquid(template.title_template,   dir);
        template.summary_template = Self::resolve_embedded_liquid(template.summary_template, dir);

        Ok(template)
    }
}

impl YamlFileParser {

    fn resolve_liquid_ref(value: Option<String>, base_dir: &PathBuf) -> Option<String> {
        value.map(|v| {
            let trimmed = v.trim();
            if trimmed.ends_with(".liquid") {
                let file_path = base_dir.join(trimmed);
                std::fs::read_to_string(&file_path)
                    .unwrap_or_else(|e| format!("<!-- could not load {trimmed}: {e} -->"))
            } else {
                v
            }
        })
    }
    fn resolve_embedded_liquid(value: Option<String>, dir: &Dir) -> Option<String> {
        value.map(|v| {
            let trimmed = v.trim();
            if !trimmed.ends_with(".liquid") {
                return v;
            }
            let liquid_path = dir.path().join(trimmed);
            BUILTIN_REPORTERS
                .get_file(&liquid_path)
                .and_then(|f| f.contents_utf8())
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("<!-- could not load embedded {trimmed} -->"))
        })
    }
}