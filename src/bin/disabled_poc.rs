use lattice::framework::prelude::*;
use serde::{Deserialize, Deserializer};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let config: Result<Config, _> = serde_yml::from_str(
        r#"
disabled:
  cond: >
    bar != c &&
    !qux
  then: corge
        "#,
    );

    let config = config.unwrap();

    let controls = UiControlBuilder::new()
        .slider("foo", 0.0, (0.0, 1.0), 0.001, config.disabled.disabled)
        .select("bar", "a", &["a", "b", "c"], None)
        .checkbox("qux", false, None)
        .slider("corge", 50.0, (0.0, 100.0), 1.0, None)
        .build();

    println!("foo is disabled: {}", controls.disabled("foo").unwrap());
    println!("then action: {}", config.disabled.then);
    println!(
        "then action result: {}",
        controls.get(&config.disabled.then)
    );

    Ok(())
}

#[derive(Deserialize)]
struct Config {
    #[serde(default, deserialize_with = "to_disabled_fn")]
    pub disabled: DisabledConfig,
}

struct DisabledConfig {
    pub disabled: DisabledFn,
    pub then: String,
}

impl Default for DisabledConfig {
    fn default() -> Self {
        DisabledConfig {
            disabled: None,
            then: "".to_string(),
        }
    }
}

fn to_disabled_fn<'de, D>(deserializer: D) -> Result<DisabledConfig, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct CondThen {
        cond: String,
        then: String,
    }

    if let Ok(structured) = CondThen::deserialize(deserializer) {
        println!("Raw structured expression: {}", structured.cond.trim());
        match parse_disabled_expression(&structured.cond) {
            Ok(disabled_fn) => {
                return Ok(DisabledConfig {
                    disabled: disabled_fn,
                    then: structured.then,
                });
            }
            Err(e) => return Err(serde::de::Error::custom(e)),
        }
    }

    Ok(DisabledConfig::default())
}

fn parse_disabled_expression(expr: &str) -> Result<DisabledFn, Box<dyn Error>> {
    if expr.trim().is_empty() {
        return Ok(None);
    }

    let or_conditions: Vec<&str> = expr.split("||").map(|s| s.trim()).collect();
    let mut condition_closures = Vec::new();

    for or_condition in or_conditions {
        let and_conditions: Vec<&str> =
            or_condition.split("&&").map(|s| s.trim()).collect();

        if and_conditions.len() == 1 {
            let closure = parse_condition(and_conditions[0])?;
            if let Some(f) = closure {
                condition_closures.push(f);
            }
        } else {
            let mut and_closures = Vec::new();
            for and_condition in and_conditions {
                let closure = parse_condition(and_condition)?;
                if let Some(f) = closure {
                    and_closures.push(f);
                }
            }

            if !and_closures.is_empty() {
                let combined_and = Box::new(move |controls: &UiControls| {
                    and_closures.iter().all(|closure| closure(controls))
                });
                condition_closures.push(combined_and);
            }
        }
    }

    if condition_closures.is_empty() {
        return Ok(None);
    }

    let combined_fn = Box::new(move |controls: &UiControls| {
        condition_closures.iter().any(|closure| closure(controls))
    });

    Ok(Some(combined_fn))
}

type ParseResult =
    Result<Option<Box<dyn Fn(&UiControls) -> bool + 'static>>, Box<dyn Error>>;

fn parse_condition(condition: &str) -> ParseResult {
    let condition = condition.trim();

    if let Some(inner_condition) = condition.strip_prefix('!') {
        let inner_closure = parse_condition(inner_condition)?;

        if let Some(f) = inner_closure {
            let negated = Box::new(move |controls: &UiControls| !f(controls));
            return Ok(Some(negated));
        }

        return Ok(None);
    }

    if condition.contains("!=") {
        let parts: Vec<&str> =
            condition.split("!=").map(|s| s.trim()).collect();

        if parts.len() != 2 {
            return Err(
                format!("Invalid condition format: {}", condition).into()
            );
        }

        let field_name = parts[0].to_string();
        let value = parts[1].to_string();

        let closure = Box::new(move |controls: &UiControls| {
            controls.string(&field_name) != value
        });

        return Ok(Some(closure));
    }

    if condition.contains("==") {
        let parts: Vec<&str> =
            condition.split("==").map(|s| s.trim()).collect();

        if parts.len() != 2 {
            return Err(
                format!("Invalid condition format: {}", condition).into()
            );
        }

        let field_name = parts[0].to_string();
        let value = parts[1].to_string();

        let closure = Box::new(move |controls: &UiControls| {
            controls.string(&field_name) == value
        });

        return Ok(Some(closure));
    }

    let field_name = condition.to_string();
    let closure =
        Box::new(move |controls: &UiControls| controls.bool(&field_name));

    Ok(Some(closure))
}
