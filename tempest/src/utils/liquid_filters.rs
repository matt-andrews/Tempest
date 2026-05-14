use colored::Colorize;
use liquid_core::model::{ScalarCow, Value};
use liquid_core::parser::{FilterArguments, ParameterReflection};
use liquid_core::{Filter, FilterReflection, ParseFilter, Runtime, ValueView};

macro_rules! color_filter {
    ($parser:ident, $filter:ident, $name:literal, $method:ident) => {
        #[derive(Clone)]
        pub struct $parser;

        #[derive(Debug)]
        struct $filter;

        impl std::fmt::Display for $filter {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, $name)
            }
        }

        impl FilterReflection for $parser {
            fn name(&self) -> &'static str {
                $name
            }
            fn description(&self) -> &'static str {
                concat!("ANSI ", $name, " color")
            }
            fn positional_parameters(&self) -> &'static [ParameterReflection] {
                &[]
            }
            fn keyword_parameters(&self) -> &'static [ParameterReflection] {
                &[]
            }
        }
        impl ParseFilter for $parser {
            fn parse(&self, _: FilterArguments) -> liquid_core::Result<Box<dyn Filter>> {
                Ok(Box::new($filter))
            }
            fn reflection(&self) -> &dyn FilterReflection {
                self
            }
        }
        impl Filter for $filter {
            fn evaluate(
                &self,
                input: &dyn ValueView,
                _: &dyn Runtime,
            ) -> liquid_core::Result<Value> {
                let s = input.to_kstr().into_string();
                Ok(Value::Scalar(ScalarCow::from(s.$method().to_string())))
            }
        }
    };
}

color_filter!(RedFilter, RedFilterImpl, "red", red);
color_filter!(GreenFilter, GreenFilterImpl, "green", green);
color_filter!(YellowFilter, YellowFilterImpl, "yellow", yellow);
color_filter!(
    BrightRedFilter,
    BrightRedFilterImpl,
    "bright_red",
    bright_red
);
color_filter!(
    BrightGreenFilter,
    BrightGreenFilterImpl,
    "bright_green",
    bright_green
);
color_filter!(
    BrightBlueFilter,
    BrightBlueFilterImpl,
    "bright_blue",
    bright_blue
);
color_filter!(
    BrightPurpleFilter,
    BrightPurpleFilterImpl,
    "bright_purple",
    bright_purple
);

// Background color filters
color_filter!(OnRedFilter, OnRedFilterImpl, "on_red", on_red);
color_filter!(OnGreenFilter, OnGreenFilterImpl, "on_green", on_green);
color_filter!(OnYellowFilter, OnYellowFilterImpl, "on_yellow", on_yellow);
color_filter!(
    OnBrightRedFilter,
    OnBrightRedFilterImpl,
    "on_bright_red",
    on_bright_red
);
color_filter!(
    OnBrightGreenFilter,
    OnBrightGreenFilterImpl,
    "on_bright_green",
    on_bright_green
);
color_filter!(
    OnBrightBlueFilter,
    OnBrightBlueFilterImpl,
    "on_bright_blue",
    on_bright_blue
);
color_filter!(
    OnBrightPurpleFilter,
    OnBrightPurpleFilterImpl,
    "on_bright_purple",
    on_bright_purple
);

// -- Semantic filters ---------------------------------------------------------

/// Colors the input based on its value as an HTTP status code.
/// 2xx → green, 3xx → yellow, everything else → red.
#[derive(Clone)]
pub struct ColorStatusFilter;

#[derive(Debug)]
struct ColorStatusFilterImpl;

impl std::fmt::Display for ColorStatusFilterImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "color_status")
    }
}
impl FilterReflection for ColorStatusFilter {
    fn name(&self) -> &'static str {
        "color_status"
    }
    fn description(&self) -> &'static str {
        "Green (2xx), yellow (3xx), red (4xx+)"
    }
    fn positional_parameters(&self) -> &'static [ParameterReflection] {
        &[]
    }
    fn keyword_parameters(&self) -> &'static [ParameterReflection] {
        &[]
    }
}
impl ParseFilter for ColorStatusFilter {
    fn parse(&self, _: FilterArguments) -> liquid_core::Result<Box<dyn Filter>> {
        Ok(Box::new(ColorStatusFilterImpl))
    }
    fn reflection(&self) -> &dyn FilterReflection {
        self
    }
}
impl Filter for ColorStatusFilterImpl {
    fn evaluate(&self, input: &dyn ValueView, _: &dyn Runtime) -> liquid_core::Result<Value> {
        let code = input.as_scalar().and_then(|s| s.to_integer()).unwrap_or(0);
        let s = input.to_kstr().into_string();
        let colored = match code {
            200..=299 => s.green(),
            300..=399 => s.yellow(),
            _ => s.red(),
        };
        Ok(Value::Scalar(ScalarCow::from(colored.to_string())))
    }
}

/// Colors the input based on its value as a duration in milliseconds.
/// ≤50ms → green, 51–200ms → yellow, >200ms → red.
#[derive(Clone)]
pub struct ColorDurationFilter;

#[derive(Debug)]
struct ColorDurationFilterImpl;

impl std::fmt::Display for ColorDurationFilterImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "color_duration")
    }
}
impl FilterReflection for ColorDurationFilter {
    fn name(&self) -> &'static str {
        "color_duration"
    }
    fn description(&self) -> &'static str {
        "Green (≤50ms), yellow (51–200ms), red (>200ms)"
    }
    fn positional_parameters(&self) -> &'static [ParameterReflection] {
        &[]
    }
    fn keyword_parameters(&self) -> &'static [ParameterReflection] {
        &[]
    }
}
impl ParseFilter for ColorDurationFilter {
    fn parse(&self, _: FilterArguments) -> liquid_core::Result<Box<dyn Filter>> {
        Ok(Box::new(ColorDurationFilterImpl))
    }
    fn reflection(&self) -> &dyn FilterReflection {
        self
    }
}
impl Filter for ColorDurationFilterImpl {
    fn evaluate(&self, input: &dyn ValueView, _: &dyn Runtime) -> liquid_core::Result<Value> {
        let ms = input.as_scalar().and_then(|s| s.to_float()).unwrap_or(0.0);
        let s = input.to_kstr().into_string();
        let colored = match ms {
            0.0..=50.0 => s.green(),
            51.0..=200.0 => s.yellow(),
            _ => s.red(),
        };
        Ok(Value::Scalar(ScalarCow::from(colored.to_string())))
    }
}
