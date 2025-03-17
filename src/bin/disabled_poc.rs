use lattice::framework::prelude::*;
use serde::{Deserialize, Deserializer};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let config: Result<Config, _> = serde_yml::from_str(
        r#"
disabled: bar is not c and not qux
        "#,
    );

    let config = config.unwrap();

    let controls = UiControlBuilder::new()
        .slider("foo", 0.0, (0.0, 1.0), 0.001, config.disabled)
        .select("bar", "a", &["a", "b", "c"], None)
        .checkbox("qux", false, None)
        .slider("corge", 50.0, (0.0, 100.0), 1.0, None)
        .build();

    println!("foo is disabled: {}", controls.disabled("foo").unwrap());

    Ok(())
}

#[derive(Deserialize)]
struct Config {
    #[serde(default, deserialize_with = "to_disabled_fn")]
    pub disabled: DisabledFn,
}

fn to_disabled_fn<'de, D>(deserializer: D) -> Result<DisabledFn, D::Error>
where
    D: Deserializer<'de>,
{
    let expression = match String::deserialize(deserializer) {
        Ok(expr) => expr,
        Err(_) => return Ok(None),
    };

    if expression.trim().is_empty() {
        return Ok(None);
    }

    match parse_disabled_expression(&expression) {
        Ok(disabled_fn) => Ok(disabled_fn),
        Err(e) => Err(serde::de::Error::custom(e)),
    }
}

fn parse_disabled_expression(expr: &str) -> Result<DisabledFn, Box<dyn Error>> {
    if expr.trim().is_empty() {
        return Ok(None);
    }

    let or_conditions: Vec<&str> =
        expr.split(" or ").map(|s| s.trim()).collect();
    let mut condition_closures = Vec::new();

    for or_condition in or_conditions {
        let and_conditions: Vec<&str> =
            or_condition.split(" and ").map(|s| s.trim()).collect();

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

    if let Some(inner_condition) = condition.strip_prefix("not ") {
        let inner_closure = parse_condition(inner_condition)?;

        if let Some(f) = inner_closure {
            let negated = Box::new(move |controls: &UiControls| !f(controls));
            return Ok(Some(negated));
        }

        return Ok(None);
    }

    if condition.contains(" is not ") {
        let parts: Vec<&str> = condition.split(" is not ").collect();
        if parts.len() != 2 {
            return Err(
                format!("Invalid condition format: {}", condition).into()
            );
        }

        let field_name = parts[0].trim().to_string();
        let value = parts[1].trim().to_string();

        let closure = Box::new(move |controls: &UiControls| {
            controls.string(&field_name) != value
        });

        return Ok(Some(closure));
    }

    if condition.contains(" is ") {
        let parts: Vec<&str> = condition.split(" is ").collect();
        if parts.len() != 2 {
            return Err(
                format!("Invalid condition format: {}", condition).into()
            );
        }

        let field_name = parts[0].trim().to_string();
        let value = parts[1].trim().to_string();

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
