use crate::pipeline::reporting::output_capabilities::OutputSink;

pub struct ConsoleSink;
impl OutputSink for ConsoleSink {
    fn println(&self, msg: &str) {
        println!("{}", msg);
    }

    fn print(&self, msg: &str) {
        print!("{}", msg);
    }
}