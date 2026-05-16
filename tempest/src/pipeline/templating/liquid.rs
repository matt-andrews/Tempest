use std::sync::LazyLock;
use crate::pipeline::templating::TemplateEngine;

static PARSER: LazyLock<liquid::Parser> = LazyLock::new(|| {
    use crate::pipeline::templating::liquid_filters::*;
    liquid::ParserBuilder::with_stdlib()
        .filter(RedFilter)
        .filter(GreenFilter)
        .filter(YellowFilter)
        .filter(BrightRedFilter)
        .filter(BrightGreenFilter)
        .filter(BrightBlueFilter)
        .filter(BrightPurpleFilter)
        .filter(OnRedFilter)
        .filter(OnGreenFilter)
        .filter(OnYellowFilter)
        .filter(OnBrightRedFilter)
        .filter(OnBrightGreenFilter)
        .filter(OnBrightBlueFilter)
        .filter(OnBrightPurpleFilter)
        .filter(ColorStatusFilter)
        .filter(ColorDurationFilter)
        .filter(JsonFilter)
        .build()
        .expect("failed to build Liquid parser")
});

pub struct LiquidEngine;

impl TemplateEngine for LiquidEngine {
    fn render(&self, source: &str, context: &liquid::Object) -> anyhow::Result<String> {
        let parsed = PARSER.parse(source)?;
        Ok(parsed.render(context)?)
    }
}