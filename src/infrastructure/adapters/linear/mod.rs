use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

use crate::{
    domain::{Resource, ResourceSource, Query, DomainError},
    ports::ResourceProvider,
};

#[derive(Debug, Serialize)]
struct GraphQLRequest {
    query: String,
    variables: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
struct GraphQLResponse<T> {
    data: Option<T>,
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Debug, Deserialize)]
struct GraphQLError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct IssuesData {
    issues: IssuesConnection,
}

#[derive(Debug, Deserialize)]
struct IssuesConnection {
    nodes: Vec<Issue>,
    #[serde(rename = "pageInfo")]
    page_info: PageInfo,
}

#[derive(Debug, Deserialize)]
struct PageInfo {
    #[serde(rename = "hasNextPage")]
    has_next_page: bool,
    #[serde(rename = "endCursor")]
    end_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Issue {
    id: String,
    title: String,
    description: Option<String>,
    #[serde(rename = "createdAt")]
    created_at: DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    updated_at: DateTime<Utc>,
    state: IssueState,
    assignee: Option<User>,
    labels: Labels,
    project: Option<Project>,
}

#[derive(Debug, Deserialize)]
struct IssueState {
    name: String,
}

#[derive(Debug, Deserialize)]
struct User {
    name: String,
    email: String,
}

#[derive(Debug, Deserialize)]
struct Labels {
    nodes: Vec<Label>,
}

#[derive(Debug, Deserialize)]
struct Label {
    name: String,
}

#[derive(Debug, Deserialize)]
struct Project {
    id: String,
    name: String,
}

pub struct LinearAdapter {
    client: reqwest::Client,
    api_key: String,
}

impl LinearAdapter {
    pub fn new(api_key: String) -> Result<Self, DomainError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&api_key)
                .map_err(|e| DomainError::ProviderError(e.to_string()))?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| DomainError::ProviderError(e.to_string()))?;

        Ok(Self { client, api_key })
    }

    fn issue_to_resource(&self, issue: Issue) -> Resource {
        let mut metadata = HashMap::new();
        
        metadata.insert("state".to_string(), serde_json::json!(issue.state.name));
        
        if let Some(assignee) = &issue.assignee {
            metadata.insert("assignee".to_string(), serde_json::json!({
                "name": assignee.name,
                "email": assignee.email,
            }));
        }
        
        let labels: Vec<String> = issue.labels.nodes.into_iter().map(|l| l.name).collect();
        metadata.insert("labels".to_string(), serde_json::json!(labels));
        
        if let Some(project) = &issue.project {
            metadata.insert("project".to_string(), serde_json::json!({
                "id": project.id,
                "name": project.name,
            }));
        }

        Resource {
            id: format!("linear_{}", issue.id),
            source: ResourceSource::Linear {
                issue_id: issue.id.clone(),
                project_id: issue.project.map(|p| p.id),
            },
            title: issue.title,
            content: issue.description.unwrap_or_default(),
            metadata,
            created_at: issue.created_at,
            updated_at: issue.updated_at,
        }
    }

    async fn execute_graphql<T: for<'de> Deserialize<'de>>(
        &self,
        query: &str,
        variables: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<T, DomainError> {
        let request = GraphQLRequest {
            query: query.to_string(),
            variables,
        };

        let response = self.client
            .post("https://api.linear.app/graphql")
            .json(&request)
            .send()
            .await
            .map_err(|e| DomainError::ProviderError(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await
                .map_err(|e| DomainError::ProviderError(e.to_string()))?;
            return Err(DomainError::ProviderError(format!("Linear API error: {}", error_text)));
        }

        let graphql_response: GraphQLResponse<T> = response.json().await
            .map_err(|e| DomainError::ProviderError(e.to_string()))?;

        if let Some(errors) = graphql_response.errors {
            let error_messages: Vec<String> = errors.into_iter().map(|e| e.message).collect();
            return Err(DomainError::ProviderError(format!("GraphQL errors: {}", error_messages.join(", "))));
        }

        graphql_response.data
            .ok_or_else(|| DomainError::ProviderError("No data in response".to_string()))
    }
}

#[async_trait]
impl ResourceProvider for LinearAdapter {
    async fn fetch_resources(&self, query: &Query) -> Result<Vec<Resource>, DomainError> {
        let graphql_query = r#"
            query GetIssues($first: Int!, $after: String) {
                issues(first: $first, after: $after) {
                    nodes {
                        id
                        title
                        description
                        createdAt
                        updatedAt
                        state {
                            name
                        }
                        assignee {
                            name
                            email
                        }
                        labels {
                            nodes {
                                name
                            }
                        }
                        project {
                            id
                            name
                        }
                    }
                    pageInfo {
                        hasNextPage
                        endCursor
                    }
                }
            }
        "#;

        let limit = query.limit.unwrap_or(50).min(250) as i32;
        let mut variables = HashMap::new();
        variables.insert("first".to_string(), serde_json::json!(limit));

        let issues_data: IssuesData = self.execute_graphql(graphql_query, Some(variables)).await?;
        
        let resources: Vec<Resource> = issues_data.issues.nodes
            .into_iter()
            .map(|issue| self.issue_to_resource(issue))
            .collect();

        Ok(resources)
    }

    async fn fetch_resource_by_id(&self, id: &str) -> Result<Resource, DomainError> {
        let issue_id = id.strip_prefix("linear_").unwrap_or(id);
        
        let graphql_query = r#"
            query GetIssue($id: String!) {
                issue(id: $id) {
                    id
                    title
                    description
                    createdAt
                    updatedAt
                    state {
                        name
                    }
                    assignee {
                        name
                        email
                    }
                    labels {
                        nodes {
                            name
                        }
                    }
                    project {
                        id
                        name
                    }
                }
            }
        "#;

        let mut variables = HashMap::new();
        variables.insert("id".to_string(), serde_json::json!(issue_id));

        #[derive(Debug, Deserialize)]
        struct IssueData {
            issue: Option<Issue>,
        }

        let issue_data: IssueData = self.execute_graphql(graphql_query, Some(variables)).await?;
        
        let issue = issue_data.issue
            .ok_or_else(|| DomainError::ResourceNotFound(format!("Linear issue not found: {}", issue_id)))?;

        Ok(self.issue_to_resource(issue))
    }

    async fn search(&self, query: &str) -> Result<Vec<Resource>, DomainError> {
        let graphql_query = r#"
            query SearchIssues($query: String!) {
                issueSearch(query: $query) {
                    nodes {
                        id
                        title
                        description
                        createdAt
                        updatedAt
                        state {
                            name
                        }
                        assignee {
                            name
                            email
                        }
                        labels {
                            nodes {
                                name
                            }
                        }
                        project {
                            id
                            name
                        }
                    }
                }
            }
        "#;

        let mut variables = HashMap::new();
        variables.insert("query".to_string(), serde_json::json!(query));

        #[derive(Debug, Deserialize)]
        struct SearchData {
            #[serde(rename = "issueSearch")]
            issue_search: IssuesConnection,
        }

        let search_data: SearchData = self.execute_graphql(graphql_query, Some(variables)).await?;
        
        let resources: Vec<Resource> = search_data.issue_search.nodes
            .into_iter()
            .map(|issue| self.issue_to_resource(issue))
            .collect();

        Ok(resources)
    }

    fn provider_name(&self) -> &'static str {
        "Linear"
    }
}