use super::{HeaderPolicyRequest, PolicyError};

pub fn render_template(
    template: &str,
    request: &HeaderPolicyRequest<'_>,
) -> Result<String, PolicyError> {
    let mut output = template.to_string();

    replace_optional_token(&mut output, "${provider.id}", request.provider_id)?;
    replace_optional_token(&mut output, "${provider.name}", request.provider_name)?;
    replace_optional_token(&mut output, "${route.id}", request.route_id)?;
    replace_optional_token(&mut output, "${model.id}", request.model_id)?;

    while let Some(start) = output.find("${request.header.") {
        let Some(end) = output[start..].find('}') else {
            return Err(PolicyError::InvalidTemplate(
                "unterminated request.header template".to_string(),
            ));
        };
        let full = &output[start..start + end + 1];
        let key = full
            .strip_prefix("${request.header.")
            .and_then(|value| value.strip_suffix('}'))
            .ok_or_else(|| {
                PolicyError::InvalidTemplate(format!("invalid template variable '{full}'"))
            })?;
        let value = request
            .headers
            .get(&key.to_ascii_lowercase())
            .ok_or_else(|| PolicyError::MissingRequestHeader(key.to_string()))?;
        output = output.replacen(full, value, 1);
    }

    while let Some(start) = output.find("${ctx.") {
        let Some(end) = output[start..].find('}') else {
            return Err(PolicyError::InvalidTemplate(
                "unterminated ctx template".to_string(),
            ));
        };
        let full = &output[start..start + end + 1];
        let key = full
            .strip_prefix("${ctx.")
            .and_then(|value| value.strip_suffix('}'))
            .ok_or_else(|| {
                PolicyError::InvalidTemplate(format!("invalid template variable '{full}'"))
            })?;
        let value = request
            .context
            .get(key)
            .ok_or_else(|| PolicyError::MissingContextValue(key.to_string()))?;
        output = output.replacen(full, value, 1);
    }

    while let Some(start) = output.find("${context.") {
        let Some(end) = output[start..].find('}') else {
            return Err(PolicyError::InvalidTemplate(
                "unterminated context template".to_string(),
            ));
        };
        let full = &output[start..start + end + 1];
        let key = full
            .strip_prefix("${context.")
            .and_then(|value| value.strip_suffix('}'))
            .ok_or_else(|| {
                PolicyError::InvalidTemplate(format!("invalid template variable '{full}'"))
            })?;
        let value = request
            .context
            .get(key)
            .ok_or_else(|| PolicyError::MissingContextValue(key.to_string()))?;
        output = output.replacen(full, value, 1);
    }

    while let Some(start) = output.find("${env.") {
        let Some(end) = output[start..].find('}') else {
            return Err(PolicyError::InvalidTemplate(
                "unterminated env template".to_string(),
            ));
        };
        let full = &output[start..start + end + 1];
        let key = full
            .strip_prefix("${env.")
            .and_then(|value| value.strip_suffix('}'))
            .ok_or_else(|| {
                PolicyError::InvalidTemplate(format!("invalid template variable '{full}'"))
            })?;
        let value = std::env::var(key)
            .map_err(|_| PolicyError::MissingEnvironmentValue(key.to_string()))?;
        output = output.replacen(full, &value, 1);
    }

    Ok(output)
}

fn replace_optional_token(
    output: &mut String,
    token: &str,
    value: Option<&str>,
) -> Result<(), PolicyError> {
    if output.contains(token) {
        let value = value.ok_or_else(|| {
            PolicyError::InvalidTemplate(format!("template variable '{}' is unavailable", token))
        })?;
        *output = output.replace(token, value);
    }
    Ok(())
}
