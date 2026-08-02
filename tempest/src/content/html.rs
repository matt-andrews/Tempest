use anyhow::anyhow;
use scraper::{Html, Selector};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize)]
pub struct HtmlMatch {
    pub tag: String,
    pub text: String,
    pub attrs: HashMap<String, String>,
}

pub fn select(input: &str, selector_text: &str) -> anyhow::Result<Vec<HtmlMatch>> {
    let selector = Selector::parse(selector_text)
        .map_err(|error| anyhow!("invalid CSS selector `{selector_text}`: {error}"))?;

    let document = Html::parse_document(input);

    Ok(document
        .select(&selector)
        .map(|element| {
            let raw_text = element.text().collect::<String>();
            let text = raw_text.split_whitespace().collect::<Vec<_>>().join(" ");

            let attrs = element
                .value()
                .attrs()
                .map(|(name, value)| (name.to_owned(), value.to_owned()))
                .collect();

            HtmlMatch {
                tag: element.value().name().to_owned(),
                text,
                attrs,
            }
        })
        .collect())
}
