use crate::error::DomainError;
use crate::model::ModelCatalog;
use crate::provider::ProviderCatalog;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayCatalog {
    providers: ProviderCatalog,
    models: ModelCatalog,
}

impl GatewayCatalog {
    pub fn new(providers: ProviderCatalog, models: ModelCatalog) -> Result<Self, DomainError> {
        for model in models.iter() {
            if !providers.contains(model.provider_id()) {
                return Err(DomainError::UnknownProviderReference {
                    model_id: model.id().clone(),
                    provider_id: model.provider_id().clone(),
                });
            }
        }

        Ok(Self { providers, models })
    }

    pub fn providers(&self) -> &ProviderCatalog {
        &self.providers
    }

    pub fn models(&self) -> &ModelCatalog {
        &self.models
    }
}

#[cfg(test)]
mod tests {
    use super::GatewayCatalog;
    use crate::{Model, ModelCatalog, ModelId, Provider, ProviderCatalog, ProviderId};

    #[test]
    fn gateway_catalog_accepts_consistent_catalogs() {
        let providers = ProviderCatalog::new(vec![
            Provider::new(ProviderId::new("openai").unwrap(), "OpenAI").unwrap(),
        ])
        .unwrap();
        let models = ModelCatalog::new(vec![
            Model::new(
                ModelId::new("gpt-4o").unwrap(),
                ProviderId::new("openai").unwrap(),
                "GPT-4o",
            )
            .unwrap(),
        ])
        .unwrap();

        let catalog = GatewayCatalog::new(providers, models).unwrap();

        assert_eq!(catalog.models().iter().count(), 1);
    }
}
