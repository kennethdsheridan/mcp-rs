use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

use crate::{
    domain::{Resource, ResourceSource, Query, DomainError},
    ports::ResourceProvider,
};

#[derive(Debug, Serialize, Deserialize)]
struct NotionDatabaseQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    filter: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sorts: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    page_size: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct NotionQueryResponse {
    results: Vec<serde_json::Value>,
    has_more: bool,
    next_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct NotionBlock {
    id: String,
    #[serde(rename = "type")]
    block_type: String,
    #[serde(flatten)]
    content: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct NotionBlocksResponse {
    results: Vec<NotionBlock>,
    has_more: bool,
    next_cursor: Option<String>,
}

pub struct NotionAdapter {
    client: reqwest::Client,
    api_key: String,
}

impl NotionAdapter {
    pub fn new(api_key: String) -> Result<Self, DomainError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", api_key))
                .map_err(|e| DomainError::ProviderError(e.to_string()))?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert("Notion-Version", HeaderValue::from_static("2022-06-28"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| DomainError::ProviderError(e.to_string()))?;

        Ok(Self { client, api_key })
    }

    async fn get_page_blocks(&self, page_id: &str) -> Result<Vec<NotionBlock>, DomainError> {
        let url = format!("https://api.notion.com/v1/blocks/{}/children", page_id);
        let mut all_blocks = Vec::new();
        let mut start_cursor: Option<String> = None;

        loop {
            let mut request = self.client.get(&url);
            
            if let Some(cursor) = &start_cursor {
                request = request.query(&[("start_cursor", cursor)]);
            }

            let response = request.send().await
                .map_err(|e| DomainError::ProviderError(e.to_string()))?;

            if !response.status().is_success() {
                let error_text = response.text().await
                    .map_err(|e| DomainError::ProviderError(e.to_string()))?;
                return Err(DomainError::ProviderError(format!("Notion API error: {}", error_text)));
            }

            let blocks_response: NotionBlocksResponse = response.json().await
                .map_err(|e| DomainError::ProviderError(e.to_string()))?;
            
            all_blocks.extend(blocks_response.results);

            if !blocks_response.has_more {
                break;
            }

            start_cursor = blocks_response.next_cursor;
        }

        Ok(all_blocks)
    }

    fn extract_text_from_blocks(&self, blocks: &[NotionBlock]) -> String {
        let mut text = String::new();
        
        for block in blocks {
            match block.block_type.as_str() {
                "paragraph" | "heading_1" | "heading_2" | "heading_3" => {
                    if let Some(content) = block.content.get(&block.block_type) {
                        if let Some(rich_text_array) = content.get("rich_text").and_then(|rt| rt.as_array()) {
                            for rich_text in rich_text_array {
                                if let Some(plain_text) = rich_text.get("plain_text").and_then(|pt| pt.as_str()) {
                                    text.push_str(plain_text);
                                    text.push('\n');
                                }
                            }
                        }
                    }
                }
                "bulleted_list_item" | "numbered_list_item" => {
                    if let Some(content) = block.content.get(&block.block_type) {
                        if let Some(rich_text_array) = content.get("rich_text").and_then(|rt| rt.as_array()) {
                            text.push_str("• ");
                            for rich_text in rich_text_array {
                                if let Some(plain_text) = rich_text.get("plain_text").and_then(|pt| pt.as_str()) {
                                    text.push_str(plain_text);
                                }
                            }
                            text.push('\n');
                        }
                    }
                }
                _ => {}
            }
        }
        
        text
    }

    async fn page_to_resource(&self, page_data: &serde_json::Value) -> Result<Resource, DomainError> {
        let page_id = page_data.get("id")
            .and_then(|id| id.as_str())
            .ok_or_else(|| DomainError::ProviderError("Missing page ID".to_string()))?;

        let title = self.extract_title_from_page(page_data);
        
        let blocks = self.get_page_blocks(page_id).await?;
        let content = self.extract_text_from_blocks(&blocks);

        let created_at = page_data.get("created_time")
            .and_then(|ct| ct.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        let updated_at = page_data.get("last_edited_time")
            .and_then(|edited_time| edited_time.as_str())
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(Utc::now);

        let mut metadata = HashMap::new();
        if let Some(props) = page_data.get("properties") {
            metadata.insert("properties".to_string(), props.clone());
        }

        Ok(Resource {
            id: format!("notion_{}", page_id),
            source: ResourceSource::Notion { 
                page_id: page_id.to_string(),
                database_id: None,
            },
            title,
            content,
            metadata,
            created_at,
            updated_at,
        })
    }

    fn extract_title_from_page(&self, page_data: &serde_json::Value) -> String {
        if let Some(properties) = page_data.get("properties") {
            // Try to find a title property
            for (key, value) in properties.as_object().unwrap_or(&serde_json::Map::new()) {
                if let Some(title_array) = value.get("title").and_then(|t| t.as_array()) {
                    if let Some(first_title) = title_array.first() {
                        if let Some(plain_text) = first_title.get("plain_text").and_then(|pt| pt.as_str()) {
                            return plain_text.to_string();
                        }
                    }
                }
            }
        }
        "Untitled".to_string()
    }
}

#[async_trait]
impl ResourceProvider for NotionAdapter {
    async fn fetch_resources(&self, query: &Query) -> Result<Vec<Resource>, DomainError> {
        // For now, we'll need a database_id from the query filters
        let database_id = query.filters.get("database_id")
            .ok_or_else(|| DomainError::InvalidQuery("database_id required for Notion queries".to_string()))?;

        let url = format!("https://api.notion.com/v1/databases/{}/query", database_id);
        
        let notion_query = NotionDatabaseQuery {
            filter: None,
            sorts: None,
            start_cursor: None,
            page_size: query.limit.map(|l| l as u32),
        };

        let response = self.client
            .post(&url)
            .json(&notion_query)
            .send()
            .await
            .map_err(|e| DomainError::ProviderError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await
                .map_err(|e| DomainError::ProviderError(e.to_string()))?;
            return Err(DomainError::ProviderError(format!("Notion API error: {}", error_text)));
        }

        let query_response: NotionQueryResponse = response.json().await
            .map_err(|e| DomainError::ProviderError(e.to_string()))?;

        let mut resources = Vec::new();
        for page_data in query_response.results {
            match self.page_to_resource(&page_data).await {
                Ok(resource) => resources.push(resource),
                Err(e) => tracing::warn!("Failed to convert page to resource: {}", e),
            }
        }

        Ok(resources)
    }

    async fn fetch_resource_by_id(&self, id: &str) -> Result<Resource, DomainError> {
        // Remove the "notion_" prefix if present
        let page_id = id.strip_prefix("notion_").unwrap_or(id);
        
        let url = format!("https://api.notion.com/v1/pages/{}", page_id);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| DomainError::ProviderError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await
                .map_err(|e| DomainError::ProviderError(e.to_string()))?;
            return Err(DomainError::ResourceNotFound(format!("Notion page not found: {}", error_text)));
        }

        let page_data: serde_json::Value = response.json().await
            .map_err(|e| DomainError::ProviderError(e.to_string()))?;

        self.page_to_resource(&page_data).await
    }

    async fn search(&self, query: &str) -> Result<Vec<Resource>, DomainError> {
        let url = "https://api.notion.com/v1/search";
        
        let search_body = serde_json::json!({
            "query": query,
            "filter": {
                "property": "object",
                "value": "page"
            }
        });

        let response = self.client
            .post(url)
            .json(&search_body)
            .send()
            .await
            .map_err(|e| DomainError::ProviderError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await
                .map_err(|e| DomainError::ProviderError(e.to_string()))?;
            return Err(DomainError::ProviderError(format!("Notion search error: {}", error_text)));
        }

        let search_response: NotionQueryResponse = response.json().await
            .map_err(|e| DomainError::ProviderError(e.to_string()))?;

        let mut resources = Vec::new();
        for page_data in search_response.results {
            match self.page_to_resource(&page_data).await {
                Ok(resource) => resources.push(resource),
                Err(e) => tracing::warn!("Failed to convert search result to resource: {}", e),
            }
        }

        Ok(resources)
    }

    fn provider_name(&self) -> &'static str {
        "Notion"
    }
}