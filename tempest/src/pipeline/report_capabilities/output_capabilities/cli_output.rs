use crate::pipeline::report_capabilities::output_capabilities::OutputCapability;

pub struct CliOutput;
impl OutputCapability for CliOutput{
    fn println(&self, msg: &str) {
        println!("{}", msg);
    }

    fn print(&self, msg: &str) {
        print!("{}", msg);
    }
}