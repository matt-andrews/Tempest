use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct OptionsModel {
    #[serde(rename = "base_uri")]
    pub base_uri: Option<String>,
    pub debug: Option<bool>,
    pub reports: Option<Vec<String>>,
}

impl OptionsModel {
    pub fn default() -> Self {
        Self {
            base_uri: None,
            debug: Some(false),
            reports: None,
        }
    }
    pub fn merge(self, other: OptionsModel) -> OptionsModel {
        OptionsModel {
            base_uri: other.base_uri.or(self.base_uri),
            debug: other.debug.or(self.debug),
            reports: other.reports.or(self.reports),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_debug_false_and_no_other_fields() {
        let d = OptionsModel::default();
        assert_eq!(d.base_uri, None);
        assert_eq!(d.debug, Some(false));
        assert_eq!(d.reports, None);
    }

    #[test]
    fn merge_other_wins_when_both_set() {
        let base = OptionsModel {
            base_uri: Some("http://base".to_string()),
            debug: Some(false),
            reports: Some(vec!["base.html".to_string()]),
        };
        let other = OptionsModel {
            base_uri: Some("http://other".to_string()),
            debug: Some(true),
            reports: Some(vec!["other.html".to_string()]),
        };
        let merged = base.merge(other);
        assert_eq!(merged.base_uri, Some("http://other".to_string()));
        assert_eq!(merged.debug, Some(true));
        assert_eq!(merged.reports, Some(vec!["other.html".to_string()]));
    }

    #[test]
    fn merge_falls_back_to_self_when_other_fields_are_none() {
        let base = OptionsModel {
            base_uri: Some("http://base".to_string()),
            debug: Some(true),
            reports: Some(vec!["r.html".to_string()]),
        };
        let other = OptionsModel {
            base_uri: None,
            debug: None,
            reports: None,
        };
        let merged = base.merge(other);
        assert_eq!(merged.base_uri, Some("http://base".to_string()));
        assert_eq!(merged.debug, Some(true));
        assert_eq!(merged.reports, Some(vec!["r.html".to_string()]));
    }

    #[test]
    fn merge_both_none_yields_none() {
        let base = OptionsModel {
            base_uri: None,
            debug: None,
            reports: None,
        };
        let other = OptionsModel {
            base_uri: None,
            debug: None,
            reports: None,
        };
        let merged = base.merge(other);
        assert_eq!(merged.base_uri, None);
        assert_eq!(merged.debug, None);
        assert_eq!(merged.reports, None);
    }
}
