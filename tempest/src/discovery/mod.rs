use std::collections::HashMap;
use std::path::PathBuf;
use include_dir::{include_dir, Dir};
use crate::discovery::parser::FileParser;
use crate::models::directory_model::DirectoryModel;
use crate::models::descriptor_model::DescriptorModel;
use crate::models::options_model::OptionsModel;
use crate::models::report_template_model::ReportTemplateModel;

mod parser;

static BUILTIN_REPORTERS: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/builtin_reporters");

pub struct DiscoveryResult {
    pub directory: DirectoryModel,
    pub templates: HashMap<String, ReportTemplateModel>,
}

fn discover_internal_templates() -> anyhow::Result<HashMap<String, ReportTemplateModel>> {
    let mut templates = HashMap::new();
    collect_embedded_templates(&BUILTIN_REPORTERS, &mut templates)?;
    Ok(templates)
}

fn collect_embedded_templates(dir: &Dir, templates: &mut HashMap<String, ReportTemplateModel>) -> anyhow::Result<()> {
    for file in dir.files() {
        let path = file.path();
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else { continue };
        if !stem.ends_with(".template") { continue }


        let Some(parser) = parser::create_parser(&path.to_path_buf()) else { continue };
        let template = parser.parse_report_template(path.to_path_buf())?;
        let key = stem.trim_end_matches(".template").to_lowercase();
        templates.insert(key, template);
    }

    for sub_dir in dir.dirs() {
        collect_embedded_templates(sub_dir, templates)?;
    }

    Ok(())
}

pub fn discover(dir: PathBuf, inherited_configs: Option<Vec<OptionsModel>>) -> anyhow::Result<DiscoveryResult> {
    let (dirs, files): (Vec<_>, Vec<_>) = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .partition(|e| e.path().is_dir());

    let mut options: Vec<OptionsModel> = inherited_configs.unwrap_or_default();
    let mut tests: Vec<DescriptorModel> = Vec::new();
    let mut templates: HashMap<String, ReportTemplateModel> = discover_internal_templates()
        .unwrap_or_else(|e| {
            eprintln!("{e}");
            HashMap::new()
        });

    for entry in files {
        let path = entry.path();
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else { continue };
        let Some(parser) = parser::create_parser(&path) else { continue };

        if stem.ends_with(".config") {
            options.push(parser.parse_config(path)?);
        } else if stem.ends_with(".spec") {
            tests.push(parser.parse_descriptor(path)?);
        } else if stem.ends_with(".template") {
            let template = parser.parse_report_template(path.clone())?;
            let key = stem.trim_end_matches(".template").to_lowercase();
            templates.insert(key, template);
        }
    }

    let mut children = Vec::new();
    for entry in dirs {
        let sub = discover(entry.path(), Some(options.clone()))?;
        children.push(sub.directory);
        templates.extend(sub.templates);
    }

    Ok(DiscoveryResult {
        directory: DirectoryModel { files: tests, options, children, dir },
        templates,
    })
}