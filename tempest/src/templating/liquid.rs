use crate::templating::TemplateEngine;
use std::collections::HashMap;
use std::sync::LazyLock;
use indexmap::IndexMap;

static PARSER: LazyLock<liquid::Parser> = LazyLock::new(|| {
    use crate::templating::liquid_filters::*;
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
        .filter(OnBrightYellowFilter)
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

    fn render_string_or_self(&self, source: &str, context: &liquid::Object) -> String {
        self.render(source, context).unwrap_or(source.to_owned())
    }

    fn render_option_string_or_self(
        &self,
        source: &Option<String>,
        context: &liquid::Object,
    ) -> Option<String> {
        source
            .as_ref()
            .map(|value| self.render(value, context).unwrap_or(value.to_owned()))
    }

    fn render_vec_string_or_self(
        &self,
        source: &Option<Vec<String>>,
        context: &liquid::Object,
    ) -> Option<Vec<String>> {
        source.as_ref().map(|values| {
            values
                .iter()
                .map(|m| self.render(m, context).unwrap_or(m.to_owned()))
                .collect()
        })
    }

    fn render_hashmap_string_or_self(
        &self,
        source: &Option<HashMap<String, String>>,
        context: &liquid::Object,
    ) -> Option<HashMap<String, String>> {
        source.as_ref().map(|values| {
            values
                .iter()
                .map(|(key, value)| {
                    (
                        self.render(key, context).unwrap_or(key.to_owned()),
                        self.render(value, context).unwrap_or(value.to_owned()),
                    )
                })
                .collect()
        })
    }

    fn render_indexmap_string_or_self(
        &self,
        source: &Option<IndexMap<String, String>>,
        context: &liquid::Object,
    ) -> Option<IndexMap<String, String>> {
        source.as_ref().map(|values| {
            values
                .iter()
                .map(|(key, value)| {
                    (
                        self.render(key, context).unwrap_or(key.to_owned()),
                        self.render(value, context).unwrap_or(value.to_owned()),
                    )
                })
                .collect()
        })
    }
}
