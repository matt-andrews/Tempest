use std::fs::File;
use crate::models::report_template_model::ReportTemplateModel;
use crate::pipeline::report_capabilities::output_capabilities::OutputCapability;

pub struct FileOutput{
    file: File
}
impl FileOutput{
    pub fn new(template: &ReportTemplateModel) -> Self{
        Self{
            
        }
    }
}
impl OutputCapability for FileOutput {
    fn println(&self, msg: &str) {
        todo!()
    }

    fn print(&self, msg: &str) {
        todo!()
    }
}