use std::fmt::Display;

use crate::error::ApplicationError;

pub fn validate_candidate_config<T, SerializeErr, ParseErr, BuildErr>(
    candidate: &T,
    serialize: impl Fn(&T) -> Result<String, SerializeErr>,
    parse: impl Fn(&str) -> Result<T, ParseErr>,
    build_runtime: impl Fn(T) -> Result<(), BuildErr>,
) -> Result<(), ApplicationError>
where
    SerializeErr: Display,
    ParseErr: Display,
    BuildErr: Display,
{
    let raw =
        serialize(candidate).map_err(|error| ApplicationError::ConfigAdmin(error.to_string()))?;
    let normalized =
        parse(&raw).map_err(|error| ApplicationError::ConfigAdmin(error.to_string()))?;
    build_runtime(normalized).map_err(|error| ApplicationError::ConfigAdmin(error.to_string()))
}

pub fn replace_config<T, RuntimeState, BuildErr, SaveErr>(
    candidate: T,
    normalize: impl Fn(T) -> T,
    build_runtime: impl Fn(&T) -> Result<RuntimeState, BuildErr>,
    save_config: impl Fn(&T) -> Result<(), SaveErr>,
) -> Result<RuntimeState, ApplicationError>
where
    BuildErr: Display,
    SaveErr: Display,
{
    let normalized = normalize(candidate);
    let runtime_state = build_runtime(&normalized)
        .map_err(|error| ApplicationError::ConfigAdmin(error.to_string()))?;
    save_config(&normalized).map_err(|error| ApplicationError::ConfigAdmin(error.to_string()))?;
    Ok(runtime_state)
}

pub fn reload_runtime_state<RuntimeState, LoadErr>(
    load_runtime: impl Fn() -> Result<RuntimeState, LoadErr>,
) -> Result<RuntimeState, ApplicationError>
where
    LoadErr: Display,
{
    load_runtime().map_err(|error| ApplicationError::ConfigAdmin(error.to_string()))
}

#[cfg(test)]
mod tests {
    use crate::{
        ApplicationError, reload_runtime_state, replace_config, validate_candidate_config,
    };

    #[test]
    fn validates_candidate_through_ports() {
        let candidate = String::from("ok");
        validate_candidate_config(
            &candidate,
            |value| Ok::<_, &'static str>(value.clone()),
            |raw| Ok::<_, &'static str>(raw.to_string()),
            |_normalized| Ok::<_, &'static str>(()),
        )
        .expect("candidate should validate");
    }

    #[test]
    fn replaces_config_through_ports() {
        let runtime_state = replace_config(
            String::from("a"),
            |value| value.to_uppercase(),
            |normalized| Ok::<_, &'static str>(normalized.clone()),
            |_normalized| Ok::<_, &'static str>(()),
        )
        .expect("replace config should succeed");
        assert_eq!(runtime_state, "A");
    }

    #[test]
    fn reload_runtime_maps_errors() {
        let error =
            reload_runtime_state::<(), _>(|| Err::<(), _>("boom")).expect_err("reload should fail");
        assert!(matches!(error, ApplicationError::ConfigAdmin(_)));
    }
}
