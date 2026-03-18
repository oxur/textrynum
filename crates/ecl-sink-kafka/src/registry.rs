//! Confluent Schema Registry HTTP client.
//!
//! Provides schema registration and retrieval for Avro schemas,
//! following the Confluent wire format protocol.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::debug;

/// Errors from Schema Registry operations.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RegistryError {
    /// HTTP request failed.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Schema Registry returned an error response.
    #[error("Schema Registry error ({status}): {message}")]
    ApiError {
        /// HTTP status code.
        status: u16,
        /// Error detail.
        message: String,
    },

    /// JSON parsing failed.
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Confluent Schema Registry HTTP client.
#[derive(Debug, Clone)]
pub struct SchemaRegistry {
    base_url: String,
    client: reqwest::Client,
}

/// Response from POST /subjects/{subject}/versions.
#[derive(Debug, Deserialize)]
struct RegisterResponse {
    id: i32,
}

/// Response from GET /schemas/ids/{id}.
#[derive(Debug, Deserialize)]
struct SchemaResponse {
    schema: String,
}

/// Request body for POST /subjects/{subject}/versions.
#[derive(Debug, Serialize)]
struct RegisterRequest<'a> {
    schema: &'a str,
    #[serde(rename = "schemaType")]
    schema_type: &'a str,
}

impl SchemaRegistry {
    /// Create a new Schema Registry client.
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Create with an existing HTTP client (for testing).
    pub fn with_client(base_url: &str, client: reqwest::Client) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client,
        }
    }

    /// Register an Avro schema for a subject. Returns the schema ID.
    ///
    /// # Errors
    ///
    /// Returns `RegistryError` if the request fails or the registry
    /// rejects the schema.
    pub async fn register_schema(
        &self,
        subject: &str,
        schema_json: &str,
    ) -> Result<i32, RegistryError> {
        let url = format!("{}/subjects/{}/versions", self.base_url, subject);

        debug!(subject = subject, "registering Avro schema");

        let body = RegisterRequest {
            schema: schema_json,
            schema_type: "AVRO",
        };

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/vnd.schemaregistry.v1+json")
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            return Err(RegistryError::ApiError {
                status: status.as_u16(),
                message: body_text,
            });
        }

        let resp: RegisterResponse = response.json().await?;
        debug!(schema_id = resp.id, "schema registered");
        Ok(resp.id)
    }

    /// Get a schema by ID.
    ///
    /// # Errors
    ///
    /// Returns `RegistryError` if the request fails or the schema is not found.
    pub async fn get_schema(&self, id: i32) -> Result<String, RegistryError> {
        let url = format!("{}/schemas/ids/{}", self.base_url, id);

        let response = self.client.get(&url).send().await?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().await.unwrap_or_default();
            return Err(RegistryError::ApiError {
                status: status.as_u16(),
                message: body_text,
            });
        }

        let resp: SchemaResponse = response.json().await?;
        Ok(resp.schema)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_schema_registry_register() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/subjects/test-value/versions"))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "id": 42 })),
            )
            .mount(&mock_server)
            .await;

        let registry = SchemaRegistry::new(&mock_server.uri());
        let schema_json = r#"{"type":"record","name":"Test","fields":[{"name":"id","type":"string"}]}"#;
        let id = registry
            .register_schema("test-value", schema_json)
            .await
            .unwrap();
        assert_eq!(id, 42);
    }

    #[tokio::test]
    async fn test_schema_registry_register_error() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("POST"))
            .and(wiremock::matchers::path("/subjects/bad-value/versions"))
            .respond_with(
                wiremock::ResponseTemplate::new(422)
                    .set_body_string(r#"{"error_code":42202,"message":"Invalid schema"}"#),
            )
            .mount(&mock_server)
            .await;

        let registry = SchemaRegistry::new(&mock_server.uri());
        let result = registry
            .register_schema("bad-value", "invalid")
            .await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), RegistryError::ApiError { status: 422, .. }));
    }

    #[tokio::test]
    async fn test_schema_registry_get() {
        let mock_server = wiremock::MockServer::start().await;

        let schema = r#"{"type":"record","name":"Test","fields":[]}"#;

        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/schemas/ids/42"))
            .respond_with(
                wiremock::ResponseTemplate::new(200)
                    .set_body_json(serde_json::json!({ "schema": schema })),
            )
            .mount(&mock_server)
            .await;

        let registry = SchemaRegistry::new(&mock_server.uri());
        let result = registry.get_schema(42).await.unwrap();
        assert_eq!(result, schema);
    }

    #[tokio::test]
    async fn test_schema_registry_get_not_found() {
        let mock_server = wiremock::MockServer::start().await;

        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/schemas/ids/999"))
            .respond_with(
                wiremock::ResponseTemplate::new(404)
                    .set_body_string(r#"{"error_code":40403,"message":"Schema not found"}"#),
            )
            .mount(&mock_server)
            .await;

        let registry = SchemaRegistry::new(&mock_server.uri());
        let result = registry.get_schema(999).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<RegistryError>();
    }
}
