#[derive(Clone, Debug, PartialEq)]
pub enum XPathValue {
    Boolean(bool),
    Number(f64),
    String(String),
    Nodes(Vec<String>),
}

pub fn evaluate(input: &[u8], expression: &str) -> simdxml::error::Result<XPathValue> {
    let mut document = simdxml::parse(input)?;

    Ok(match document.eval(expression)? {
        simdxml::XPathResult::Boolean(value) => XPathValue::Boolean(value),
        simdxml::XPathResult::Number(value) => XPathValue::Number(value),
        simdxml::XPathResult::String(value) => XPathValue::String(value),
        simdxml::XPathResult::NodeSet(_) => XPathValue::Nodes(document.xpath_string(expression)?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const XML: &[u8] = br#"
        <slideshow title="Sample Slide Show">
            <slide type="all"><title>First</title></slide>
            <slide type="all"><title>Second</title></slide>
        </slideshow>
    "#;

    #[test]
    fn evaluates_xpath_scalar_values() {
        assert_eq!(
            evaluate(XML, "string(/slideshow/@title)").unwrap(),
            XPathValue::String("Sample Slide Show".to_string())
        );
        assert_eq!(
            evaluate(XML, "count(/slideshow/slide)").unwrap(),
            XPathValue::Number(2.0)
        );
        assert_eq!(
            evaluate(XML, "boolean(/slideshow/slide[@type='all'])").unwrap(),
            XPathValue::Boolean(true)
        );
    }

    #[test]
    fn evaluates_node_sets_as_xpath_string_values() {
        assert_eq!(
            evaluate(XML, "/slideshow/slide/title").unwrap(),
            XPathValue::Nodes(vec!["First".to_string(), "Second".to_string()])
        );
    }
}
