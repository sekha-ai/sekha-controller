// use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;
// use uuid::Uuid;

#[derive(Error, Debug)]
pub enum ChromaError {
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),
    #[error("Chroma API error {status}: {message}")]
    ApiError { status: u16, message: String },
    #[error("Collection not found: {0}")]
    CollectionNotFound(String),
    #[error("Embedding dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },
}

#[derive(Debug, Clone)]
pub struct ScoredResult {
    pub id: String,
    pub score: f32,
    pub metadata: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChromaUpsertRequest {
    ids: Vec<String>,
    embeddings: Vec<Vec<f32>>,
    metadatas: Option<Vec<Value>>,
    documents: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChromaQueryRequest {
    query_embeddings: Vec<Vec<f32>>,
    n_results: u32,
    where_clause: Option<Value>,
    include: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ChromaQueryResponse {
    ids: Vec<Vec<String>>,
    distances: Vec<Vec<f32>>,
    metadatas: Option<Vec<Vec<Value>>>,
    documents: Option<Vec<Vec<String>>>,
}

/// Rust-native ChromaDB client using HTTP API v2
pub struct ChromaClient {
    base_url: String,
    client: Client,
    tenant: String,
    database: String,
}

impl ChromaClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: Client::new(),
            tenant: "default_tenant".to_string(),
            database: "default_database".to_string(),
        }
    }

    fn collections_url(&self) -> String {
        format!(
            "{}/api/v2/tenants/{}/databases/{}/collections",
            self.base_url, self.tenant, self.database
        )
    }

    fn collection_url(&self, collection_name: &str) -> String {
        format!(
            "{}/api/v2/tenants/{}/databases/{}/collections/{}",
            self.base_url, self.tenant, self.database, collection_name
        )
    }

    fn collection_operation_url(&self, collection_id: &str, operation: &str) -> String {
        format!(
            "{}/api/v2/tenants/{}/databases/{}/collections/{}/{}",
            self.base_url, self.tenant, self.database, collection_id, operation
        )
    }

    /// Ensure collection exists, create if not
    pub async fn ensure_collection(&self, name: &str, dimension: i32) -> Result<(), ChromaError> {
        let url = self.collections_url();

        // List all collections
        let response = self.client.get(&url).send().await?;
        
        match response.status() {
            StatusCode::OK => {
                let collections: Vec<Value> = response.json().await?;
                let exists = collections.iter().any(|c| c["name"] == name);

                if !exists {
                    tracing::info!("Creating Chroma collection: {}", name);
                    self.create_collection(name, dimension).await?;
                } else {
                    tracing::debug!("Collection {} already exists", name);
                }
                Ok(())
            }
            status => {
                let message = response.text().await?;
                Err(ChromaError::ApiError {
                    status: status.as_u16(),
                    message,
                })
            }
        }
    }

    /// Create a new collection with specified dimension
    async fn create_collection(&self, name: &str, dimension: i32) -> Result<(), ChromaError> {
        let url = self.collections_url();

        let body = json!({
            "name": name,
            "metadata": {
                "hnsw:space": "cosine",
                "dimension": dimension
            }
        });

        let response = self.client.post(&url).json(&body).send().await?;

        match response.status() {
            StatusCode::OK | StatusCode::CREATED => {
                tracing::info!("Created collection {} with dimension {}", name, dimension);
                Ok(())
            }
            status => {
                let message = response.text().await?;
                Err(ChromaError::ApiError {
                    status: status.as_u16(),
                    message,
                })
            }
        }
    }

    /// Store or update a vector with metadata
    pub async fn upsert(
        &self,
        collection: &str,
        id: &str,
        embedding: Vec<f32>,
        metadata: Value,
        document: Option<String>,
    ) -> Result<(), ChromaError> {
        let collection_id = self.get_collection_id(collection).await?;
        let url = self.collection_operation_url(&collection_id, "upsert");

        let request = ChromaUpsertRequest {
            ids: vec![id.to_string()],
            embeddings: vec![embedding],
            metadatas: Some(vec![metadata]),
            documents: document.map(|d| vec![d]),
        };

        let response = self.client.post(&url).json(&request).send().await?;

        match response.status() {
            StatusCode::OK | StatusCode::CREATED => {
                tracing::trace!("Successfully upserted vector: {}", id);
                Ok(())
            }
            status => {
                let message = response.text().await?;
                Err(ChromaError::ApiError {
                    status: status.as_u16(),
                    message,
                })
            }
        }
    }

    /// Query similar vectors
    pub async fn query(
        &self,
        collection: &str,
        embedding: Vec<f32>,
        limit: u32,
        filters: Option<Value>,
    ) -> Result<Vec<ScoredResult>, ChromaError> {
        let collection_id = self.get_collection_id(collection).await?;
        let url = self.collection_operation_url(&collection_id, "query");

        let request = ChromaQueryRequest {
            query_embeddings: vec![embedding],
            n_results: limit,
            where_clause: filters,
            include: vec!["distances".to_string(), "metadatas".to_string()],
        };

        let response = self.client.post(&url).json(&request).send().await?;

        match response.status() {
            StatusCode::OK => {
                let query_response: ChromaQueryResponse = response.json().await?;
                Ok(self.parse_query_results(query_response)?)
            }
            status => {
                let message = response.text().await?;
                Err(ChromaError::ApiError {
                    status: status.as_u16(),
                    message,
                })
            }
        }
    }

    /// Delete vectors from collection
    pub async fn delete(&self, collection: &str, ids: Vec<String>) -> Result<(), ChromaError> {
        let collection_id = self.get_collection_id(collection).await?;
        let url = self.collection_operation_url(&collection_id, "delete");

        let body = json!({ "ids": ids });

        let response = self.client.post(&url).json(&body).send().await?;

        match response.status() {
            StatusCode::OK => {
                tracing::info!("Deleted {} vectors from {}", ids.len(), collection);
                Ok(())
            }
            status => {
                let message = response.text().await?;
                Err(ChromaError::ApiError {
                    status: status.as_u16(),
                    message,
                })
            }
        }
    }

    /// Get collection ID by name
    async fn get_collection_id(&self, name: &str) -> Result<String, ChromaError> {
        let url = self.collection_url(name);

        let response = self.client.get(&url).send().await?;

        match response.status() {
            StatusCode::OK => {
                let collection: Value = response.json().await?;
                collection["id"]
                    .as_str()
                    .map(|s| s.to_string())
                    .ok_or_else(|| ChromaError::CollectionNotFound(name.to_string()))
            }
            StatusCode::NOT_FOUND => Err(ChromaError::CollectionNotFound(name.to_string())),
            status => {
                let message = response.text().await?;
                Err(ChromaError::ApiError {
                    status: status.as_u16(),
                    message,
                })
            }
        }
    }

    /// Parse query results into ScoredResult structs
    fn parse_query_results(
        &self,
        response: ChromaQueryResponse,
    ) -> Result<Vec<ScoredResult>, ChromaError> {
        let mut results = Vec::new();

        if let Some(ids) = response.ids.first() {
            let distances = response
                .distances
                .first()
                .ok_or_else(|| ChromaError::ApiError {
                    status: 500,
                    message: "No distances returned from Chroma".to_string(),
                })?;

            let metadatas = response.metadatas.as_ref();

            for (idx, id) in ids.iter().enumerate() {
                let score = distances.get(idx).copied().unwrap_or(0.0);
                let metadata = metadatas
                    .and_then(|m| m.first())
                    .and_then(|m| m.get(idx))
                    .cloned()
                    .unwrap_or_else(|| json!({}));

                results.push(ScoredResult {
                    id: id.clone(),
                    score,
                    metadata,
                });
            }
        }

        Ok(results)
    }
    
    /// Health check method - uses v2 API
    pub async fn ping(&self) -> Result<(), ChromaError> {
        let url = format!("{}/api/v2/heartbeat", self.base_url);
        self.client.get(&url).send().await?.error_for_status()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_ensure_collection_creates_if_not_exists() {
        let mock_server = MockServer::start().await;
        let client = ChromaClient::new(mock_server.uri());

        // Mock GET collections (empty)
        Mock::given(method("GET"))
            .and(path("/api/v2/tenants/default_tenant/databases/default_database/collections"))
            .respond_with(ResponseTemplate::new(200).set_body_json(vec![] as Vec<Value>))
            .mount(&mock_server)
            .await;

        // Mock POST collection creation
        Mock::given(method("POST"))
            .and(path("/api/v2/tenants/default_tenant/databases/default_database/collections"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "id": "test-id" })))
            .mount(&mock_server)
            .await;

        let result = client.ensure_collection("test_collection", 384).await;
        assert!(result.is_ok());
    }
}
