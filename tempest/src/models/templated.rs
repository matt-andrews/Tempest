use crate::templating::TemplateEngine;
use crate::templating::liquid::LiquidEngine;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Templated<T> {
    Literal(T),
    Liquid(String),
}

impl Templated<bool> {
    pub fn resolve(&self, engine: &LiquidEngine, context: &liquid::Object) -> anyhow::Result<bool> {
        match self {
            Self::Literal(value) => Ok(*value),
            Self::Liquid(template) => {
                let rendered = engine.render(template, context)?;

                match rendered.trim() {
                    "true" => Ok(true),
                    "false" => Ok(false),
                    value => anyhow::bail!(
                        "Liquid template must render to `true` or `false`, got `{value}`"
                    ),
                }
            }
        }
    }
}
