use std::collections::HashMap;
use std::sync::Arc;

use crate::{
    domain::{Resource, Query, QuerySource, DomainError},
    ports::ResourceProvider,
};

pub struct ResourceService {
    providers: HashMap<String, Arc<dyn ResourceProvider>>,
}

impl ResourceService {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    pub fn add_provider(&mut self, provider: Arc<dyn ResourceProvider>) {
        let name = provider.provider_name().to_lowercase();
        self.providers.insert(name, provider);
    }

    pub async fn fetch_resources(&self, query: &Query) -> Result<Vec<Resource>, DomainError> {
        match &query.source {
            QuerySource::Notion => {
                let provider = self.providers.get("notion")
                    .ok_or_else(|| DomainError::ProviderError("Notion provider not configured".to_string()))?;
                provider.fetch_resources(query).await
            }
            QuerySource::Linear => {
                let provider = self.providers.get("linear")
                    .ok_or_else(|| DomainError::ProviderError("Linear provider not configured".to_string()))?;
                provider.fetch_resources(query).await
            }
            QuerySource::All => {
                let mut all_resources = Vec::new();
                
                for provider in self.providers.values() {
                    match provider.fetch_resources(query).await {
                        Ok(mut resources) => all_resources.append(&mut resources),
                        Err(e) => tracing::warn!("Provider {} failed: {}", provider.provider_name(), e),
                    }
                }
                
                Ok(all_resources)
            }
        }
    }

    pub async fn fetch_resource_by_id(&self, id: &str) -> Result<Resource, DomainError> {
        // Determine provider from ID prefix
        if id.starts_with("notion_") {
            let provider = self.providers.get("notion")
                .ok_or_else(|| DomainError::ProviderError("Notion provider not configured".to_string()))?;
            provider.fetch_resource_by_id(id).await
        } else if id.starts_with("linear_") {
            let provider = self.providers.get("linear")
                .ok_or_else(|| DomainError::ProviderError("Linear provider not configured".to_string()))?;
            provider.fetch_resource_by_id(id).await
        } else {
            // Try all providers
            for provider in self.providers.values() {
                match provider.fetch_resource_by_id(id).await {
                    Ok(resource) => return Ok(resource),
                    Err(DomainError::ResourceNotFound(_)) => continue,
                    Err(e) => return Err(e),
                }
            }
            Err(DomainError::ResourceNotFound(format!("Resource not found: {}", id)))
        }
    }

    pub async fn search(&self, query: &str, sources: Option<Vec<QuerySource>>) -> Result<Vec<Resource>, DomainError> {
        let mut all_resources = Vec::new();
        
        let search_sources = sources.unwrap_or_else(|| vec![QuerySource::All]);
        
        for source in search_sources {
            match source {
                QuerySource::Notion => {
                    if let Some(provider) = self.providers.get("notion") {
                        match provider.search(query).await {
                            Ok(mut resources) => all_resources.append(&mut resources),
                            Err(e) => tracing::warn!("Notion search failed: {}", e),
                        }
                    }
                }
                QuerySource::Linear => {
                    if let Some(provider) = self.providers.get("linear") {
                        match provider.search(query).await {
                            Ok(mut resources) => all_resources.append(&mut resources),
                            Err(e) => tracing::warn!("Linear search failed: {}", e),
                        }
                    }
                }
                QuerySource::All => {
                    for provider in self.providers.values() {
                        match provider.search(query).await {
                            Ok(mut resources) => all_resources.append(&mut resources),
                            Err(e) => tracing::warn!("Provider {} search failed: {}", provider.provider_name(), e),
                        }
                    }
                }
            }
        }
        
        Ok(all_resources)
    }

    pub fn list_providers(&self) -> Vec<&str> {
        self.providers.values().map(|p| p.provider_name()).collect()
    }
}