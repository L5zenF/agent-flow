use std::collections::BTreeMap;

use crate::error::DomainError;
use crate::ids::{ModelId, ProviderId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    id: ModelId,
    provider_id: ProviderId,
    display_name: String,
}

impl Model {
    pub fn new(
        id: ModelId,
        provider_id: ProviderId,
        display_name: impl Into<String>,
    ) -> Result<Self, DomainError> {
        let display_name = display_name.into().trim().to_string();
        if display_name.is_empty() {
            return Err(DomainError::BlankModelName);
        }

        Ok(Self {
            id,
            provider_id,
            display_name,
        })
    }

    pub fn id(&self) -> &ModelId {
        &self.id
    }

    pub fn provider_id(&self) -> &ProviderId {
        &self.provider_id
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn belongs_to(&self, provider_id: &ProviderId) -> bool {
        &self.provider_id == provider_id
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ModelCatalog {
    models: BTreeMap<ModelId, Model>,
}

impl ModelCatalog {
    pub fn new(models: Vec<Model>) -> Result<Self, DomainError> {
        let mut indexed = BTreeMap::new();

        for model in models {
            let model_id = model.id().clone();
            if indexed.insert(model_id.clone(), model).is_some() {
                return Err(DomainError::DuplicateModelId { model_id });
            }
        }

        Ok(Self { models: indexed })
    }

    pub fn get(&self, model_id: &ModelId) -> Option<&Model> {
        self.models.get(model_id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Model> {
        self.models.values()
    }

    pub fn models_for_provider(&self, provider_id: &ProviderId) -> Vec<&Model> {
        self.models
            .values()
            .filter(|model| model.belongs_to(provider_id))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{Model, ModelCatalog};
    use crate::{ModelId, ProviderId};

    #[test]
    fn model_catalog_groups_models_by_provider() {
        let catalog = ModelCatalog::new(vec![
            Model::new(
                ModelId::new("gpt-4o").unwrap(),
                ProviderId::new("openai").unwrap(),
                "GPT-4o",
            )
            .unwrap(),
            Model::new(
                ModelId::new("claude-sonnet").unwrap(),
                ProviderId::new("anthropic").unwrap(),
                "Claude Sonnet",
            )
            .unwrap(),
        ])
        .unwrap();

        assert_eq!(
            catalog
                .models_for_provider(&ProviderId::new("openai").unwrap())
                .len(),
            1
        );
    }
}
