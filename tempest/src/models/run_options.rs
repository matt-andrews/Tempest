use crate::models::templated::Templated;
use crate::templating::TemplateEngine;
use crate::templating::liquid::LiquidEngine;
use liquid_core::model::DateTime;
use serde::{Deserialize, Serialize};
use std::num::NonZeroUsize;

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct RunOptions {
    pub base_uri: Option<String>,
    pub debug: Option<bool>,
    pub reports: Option<Vec<String>>,
    pub retries: Option<u8>,
    pub concurrent: Option<bool>,
    pub skip: Option<Templated<bool>>,
    #[serde(rename = "loop")]
    pub loop_count: Option<NonZeroUsize>,

    pub quiet_retry: Option<bool>,
    pub quiet_run: Option<bool>,
    pub quiet_fail: Option<bool>,

    #[serde(skip)]
    pub start_time: Option<DateTime>,
}

impl RunOptions {
    pub fn default_from_args(debug: bool, retries: u8) -> Self {
        Self {
            debug: Some(debug),
            reports: Some(vec!["console".to_string()]),
            start_time: Some(DateTime::now()),
            retries: Some(retries),
            ..RunOptions::default()
        }
    }
    pub fn merge(self, other: RunOptions) -> RunOptions {
        RunOptions {
            base_uri: other.base_uri.or(self.base_uri),
            debug: other.debug.or(self.debug),
            reports: other.reports.or(self.reports),
            start_time: other.start_time.or(self.start_time),
            retries: other.retries.or(self.retries),
            concurrent: other.concurrent.or(self.concurrent),
            skip: other.skip.or(self.skip),
            // Looping is a descriptor-local execution directive. Taking only the
            // more-local value prevents a parent's loop from being applied again
            // by every descendant.
            loop_count: other.loop_count,

            quiet_fail: other.quiet_fail.or(self.quiet_fail),
            quiet_retry: other.quiet_retry.or(self.quiet_retry),
            quiet_run: other.quiet_run.or(self.quiet_run),
        }
    }
    pub fn render_template(&mut self, engine: &LiquidEngine, obj: &liquid_core::Object) {
        self.base_uri = engine.render_option_string_or_self(&self.base_uri, obj);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::num::NonZeroUsize;

    #[test]
    fn default_debug_false_has_specific_fields_and_no_other_fields() {
        let d = RunOptions::default_from_args(false, 33);
        assert_eq!(d.base_uri, None);
        assert_eq!(d.debug, Some(false));
        assert_eq!(d.reports, Some(vec!["console".to_string()]));
        assert_eq!(d.retries, Some(33));
        assert_eq!(d.concurrent, None);
    }

    #[test]
    fn default_debug_true_has_specific_fields_and_no_other_fields() {
        let d = RunOptions::default_from_args(true, 12);
        assert_eq!(d.base_uri, None);
        assert_eq!(d.debug, Some(true));
        assert_eq!(d.reports, Some(vec!["console".to_string()]));
        assert_eq!(d.retries, Some(12));
        assert_eq!(d.concurrent, None);
    }

    #[test]
    fn merge_other_wins_when_both_set() {
        let base = RunOptions {
            base_uri: Some("http://base".to_string()),
            reports: Some(vec!["base.html".to_string()]),
            retries: Some(0),
            ..RunOptions::default()
        };
        let other = RunOptions {
            base_uri: Some("http://other".to_string()),
            debug: Some(true),
            reports: Some(vec!["other.html".to_string()]),
            retries: Some(0),
            ..RunOptions::default()
        };
        let merged = base.merge(other);
        assert_eq!(merged.base_uri, Some("http://other".to_string()));
        assert_eq!(merged.debug, Some(true));
        assert_eq!(merged.reports, Some(vec!["other.html".to_string()]));
        assert_eq!(merged.retries, Some(0));
    }

    #[test]
    fn merge_falls_back_to_self_when_other_fields_are_none() {
        let base = RunOptions {
            base_uri: Some("http://base".to_string()),
            debug: Some(true),
            reports: Some(vec!["r.html".to_string()]),
            retries: Some(0),
            ..RunOptions::default()
        };
        let other = RunOptions::default();
        let merged = base.merge(other);
        assert_eq!(merged.base_uri, Some("http://base".to_string()));
        assert_eq!(merged.debug, Some(true));
        assert_eq!(merged.reports, Some(vec!["r.html".to_string()]));
        assert_eq!(merged.retries, Some(0));
    }

    #[test]
    fn merge_both_none_yields_none() {
        let base = RunOptions {
            retries: Some(0),
            ..RunOptions::default()
        };
        let other = RunOptions {
            retries: Some(0),
            ..RunOptions::default()
        };
        let merged = base.merge(other);
        assert_eq!(merged.base_uri, None);
        assert_eq!(merged.debug, None);
        assert_eq!(merged.reports, None);
        assert_eq!(merged.retries, Some(0));
    }

    #[test]
    fn merge_preserves_retries() {
        let base = RunOptions {
            retries: Some(1),
            ..Default::default()
        };
        let other = RunOptions {
            retries: Some(3),
            ..Default::default()
        };

        let merged = base.merge(other);

        assert_eq!(merged.retries, Some(3));
    }

    #[test]
    fn merge_falls_back_to_self_retries_when_other_is_none() {
        let base = RunOptions {
            retries: Some(2),
            ..Default::default()
        };
        let other = RunOptions {
            retries: None,
            ..Default::default()
        };

        let merged = base.merge(other);

        assert_eq!(merged.retries, Some(2));
    }

    #[test]
    fn merge_uses_more_local_concurrency_setting() {
        let base = RunOptions {
            concurrent: Some(false),
            ..Default::default()
        };
        let other = RunOptions {
            concurrent: Some(true),
            ..Default::default()
        };

        assert_eq!(base.merge(other).concurrent, Some(true));
    }

    #[test]
    fn merge_does_not_inherit_descriptor_loop_count() {
        let parent = RunOptions {
            loop_count: NonZeroUsize::new(2),
            ..Default::default()
        };

        assert_eq!(parent.merge(RunOptions::default()).loop_count, None);
    }

    #[test]
    fn merge_uses_loop_count_declared_on_more_local_descriptor() {
        let local = RunOptions {
            loop_count: NonZeroUsize::new(3),
            ..Default::default()
        };

        assert_eq!(
            RunOptions::default().merge(local).loop_count.unwrap().get(),
            3
        );
    }
}
