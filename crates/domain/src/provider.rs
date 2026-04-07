use std::collections::BTreeMap;

use crate::error::DomainError;
use crate::ids::ProviderId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provider {
    id: ProviderId,
    display_name: String,
}

impl Provider {
    pub fn new(id: ProviderId, display_name: impl Into<String>) -> Result<Self, DomainError> {
        let display_name = display_name.into().trim().to_string();
        if display_name.is_empty() {
            return Err(DomainError::BlankProviderName);
        }

        Ok(Self { id, display_name })
    }

    pub fn id(&self) -> &ProviderId {
        &self.id
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProviderCatalog {
    providers: BTreeMap<ProviderId, Provider>,
}

impl ProviderCatalog {
    pub fn new(providers: Vec<Provider>) -> Result<Self, DomainError> {
        let mut indexed = BTreeMap::new();

        for provider in providers {
            let provider_id = provider.id().clone();
            if indexed.insert(provider_id.clone(), provider).is_some() {
                return Err(DomainError::DuplicateProviderId { provider_id });
            }
        }

        Ok(Self { providers: indexed })
    }

    pub fn get(&self, provider_id: &ProviderId) -> Option<&Provider> {
        self.providers.get(provider_id)
    }

    pub fn contains(&self, provider_id: &ProviderId) -> bool {
        self.providers.contains_key(provider_id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Provider> {
        self.providers.values()
    }

    pub fn len(&self) -> usize {
        self.providers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::{Provider, ProviderCatalog};
    use crate::ProviderId;

    #[test]
    fn provider_catalog_tracks_uniqueness() {
        let providers = vec![
            Provider::new(ProviderId::new("openai").unwrap(), "OpenAI").unwrap(),
            Provider::new(ProviderId::new("anthropic").unwrap(), "Anthropic").unwrap(),
        ];

        let catalog = ProviderCatalog::new(providers).unwrap();

        assert!(catalog.contains(&ProviderId::new("openai").unwrap()));
        assert_eq!(catalog.len(), 2);
    }
}
