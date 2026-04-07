use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SettingsSchema {
    pub global: SettingsSchemaSection,
    pub providers: SettingsSchemaSection,
    pub models: SettingsSchemaSection,
}

#[derive(Debug, Clone, Serialize)]
pub struct SettingsSchemaSection {
    pub key: String,
    pub title: String,
    pub description: String,
    pub list_label: Option<String>,
    pub add_label: Option<String>,
    pub empty_text: Option<String>,
    pub fields: Vec<SettingsSchemaField>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SettingsSchemaField {
    pub key: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub option_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<SettingsSchemaField>>,
}

pub fn gateway_settings_schema() -> SettingsSchema {
    SettingsSchema {
        global: SettingsSchemaSection {
            key: "global".to_string(),
            title: "Global config".to_string(),
            description:
                "These values still use the live gateway config state and stay host-controlled."
                    .to_string(),
            list_label: None,
            add_label: None,
            empty_text: None,
            fields: vec![
                text_field("listen", "Listen"),
                text_field("admin_listen", "Admin Listen"),
                SettingsSchemaField {
                    key: "default_secret_env".to_string(),
                    label: "Default Secret Env".to_string(),
                    field_type: "text".to_string(),
                    required: None,
                    placeholder: Some("PROXY_SECRET".to_string()),
                    help_text: None,
                    option_source: None,
                    fields: None,
                },
            ],
        },
        providers: SettingsSchemaSection {
            key: "providers".to_string(),
            title: "Providers".to_string(),
            description: "Manage upstream providers and their default headers.".to_string(),
            list_label: Some("Providers".to_string()),
            add_label: Some("Add Provider".to_string()),
            empty_text: Some("No providers configured.".to_string()),
            fields: vec![
                text_field("id", "ID"),
                text_field("name", "Name"),
                SettingsSchemaField {
                    key: "base_url".to_string(),
                    label: "Base URL".to_string(),
                    field_type: "text".to_string(),
                    required: Some(true),
                    placeholder: Some("https://example.com".to_string()),
                    help_text: None,
                    option_source: None,
                    fields: None,
                },
                SettingsSchemaField {
                    key: "default_headers".to_string(),
                    label: "Default Headers".to_string(),
                    field_type: "object_list".to_string(),
                    required: None,
                    placeholder: None,
                    help_text: Some("Headers sent with every upstream request.".to_string()),
                    option_source: None,
                    fields: Some(vec![
                        text_field("name", "Header"),
                        text_field("value", "Value"),
                        text_field("secret_env", "Secret Env"),
                        SettingsSchemaField {
                            key: "encrypted".to_string(),
                            label: "Encrypted".to_string(),
                            field_type: "boolean".to_string(),
                            required: None,
                            placeholder: None,
                            help_text: None,
                            option_source: None,
                            fields: None,
                        },
                    ]),
                },
            ],
        },
        models: SettingsSchemaSection {
            key: "models".to_string(),
            title: "Models".to_string(),
            description: "Attach models to providers through the shared config state.".to_string(),
            list_label: Some("Models".to_string()),
            add_label: Some("Add Model".to_string()),
            empty_text: Some("No models configured.".to_string()),
            fields: vec![
                text_field("id", "ID"),
                text_field("name", "Name"),
                SettingsSchemaField {
                    key: "provider_id".to_string(),
                    label: "Provider".to_string(),
                    field_type: "select".to_string(),
                    required: Some(true),
                    placeholder: None,
                    help_text: None,
                    option_source: Some("providers".to_string()),
                    fields: None,
                },
                SettingsSchemaField {
                    key: "description".to_string(),
                    label: "Description".to_string(),
                    field_type: "textarea".to_string(),
                    required: None,
                    placeholder: Some("Optional model description.".to_string()),
                    help_text: None,
                    option_source: None,
                    fields: None,
                },
            ],
        },
    }
}

fn text_field(key: &str, label: &str) -> SettingsSchemaField {
    SettingsSchemaField {
        key: key.to_string(),
        label: label.to_string(),
        field_type: "text".to_string(),
        required: Some(true),
        placeholder: None,
        help_text: None,
        option_source: None,
        fields: None,
    }
}

#[cfg(test)]
mod tests {
    use super::gateway_settings_schema;

    #[test]
    fn builds_gateway_settings_schema() {
        let schema = gateway_settings_schema();
        assert_eq!(schema.global.key, "global");
        assert_eq!(schema.providers.key, "providers");
        assert_eq!(schema.models.key, "models");
    }
}
