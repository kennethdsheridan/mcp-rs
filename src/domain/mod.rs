use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub id: String,
    pub source: ResourceSource,
    pub title: String,
    pub content: String,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceSource {
    Notion {
        page_id: String,
        database_id: Option<String>,
    },
    Linear {
        issue_id: String,
        project_id: Option<String>,
    },
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Query {
    pub source: QuerySource,
    pub filters: HashMap<String, String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuerySource {
    Notion,
    Linear,
    All,
}

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    #[error("Invalid query: {0}")]
    InvalidQuery(String),

    #[error("Provider error: {0}")]
    ProviderError(String),
}
