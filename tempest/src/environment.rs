use std::env;
use std::ops::RangeInclusive;
use std::sync::LazyLock;
use colored::{ColoredString, Colorize};

struct Environment {
    status_success: RangeInclusive<i64>,
    status_redirect: RangeInclusive<i64>,
    duration_small: RangeInclusive<f64>,
    duration_medium: RangeInclusive<f64>,
}

impl Environment {
    pub fn new() -> Self{
        Self{
            status_success: env_i64_range("STATUS_SUCCESS", 200..=299),
            status_redirect: env_i64_range("STATUS_REDIRECT", 300..=399),
            duration_small: env_f64_range("DURATION_SMALL", 0.0..=50.0),
            duration_medium: env_f64_range("DURATION_MEDIUM", 51.0..=200.0),
        }
    }
}

static ENV: LazyLock<Environment> = LazyLock::new(Environment::new);

pub fn get_status_color(status: i64, str: &str) -> ColoredString{

    if ENV.status_success.contains(&status){
        return str.green();
    } else if ENV.status_redirect.contains(&status){
        return str.yellow();
    }
    str.red()
}

pub fn get_duration_color(ms: f64, str: &str) -> ColoredString{
    if ENV.duration_small.contains(&ms){
        return str.green();
    } else if ENV.duration_medium.contains(&ms){
        return str.yellow();
    }
    str.red()
}

fn env_i64_range(name: &str, default: RangeInclusive<i64>) -> RangeInclusive<i64> {
    match env::var(name){
        Ok(value) => {
          let (start, end) = value
              .split_once(':')
              .unwrap_or(("0", "0"));
            let start = start.parse::<i64>().unwrap_or(0);
            let end = end.parse::<i64>().unwrap_or(0);
            if start >= end{
                return default;
            }
            return start..=end;
        },
        Err(_) => default,
    }
}
fn env_f64_range(name: &str, default: RangeInclusive<f64>) -> RangeInclusive<f64> {
    match env::var(name){
        Ok(value) => {
            let (start, end) = value
                .split_once(':')
                .unwrap_or(("0", "0"));
            let start = start.parse::<f64>().unwrap_or(0.0);
            let end = end.parse::<f64>().unwrap_or(0.0);
            if start >= end{
                return default;
            }
            return start..=end;
        },
        Err(_) => default,
    }
}
