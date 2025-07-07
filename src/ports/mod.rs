use crate::domain::{DomainError, Query, Resource};
use async_trait::async_trait;

#[async_trait]
pub trait ResourceProvider: Send + Sync {
    async fn fetch_resources(&self, query: &Query) -> Result<Vec<Resource>, DomainError>;
    async fn fetch_resource_by_id(&self, id: &str) -> Result<Resource, DomainError>;
    async fn search(&self, query: &str) -> Result<Vec<Resource>, DomainError>;
    fn provider_name(&self) -> &'static str;
}

#[async_trait]
pub trait ResourceRepository: Send + Sync {
    async fn save(&self, resource: &Resource) -> Result<(), DomainError>;
    async fn find_by_id(&self, id: &str) -> Result<Option<Resource>, DomainError>;
    async fn find_all(&self) -> Result<Vec<Resource>, DomainError>;
    async fn delete(&self, id: &str) -> Result<(), DomainError>;
}
