# LanceDB Integration Guide for Rust MCP Servers

This guide provides a comprehensive approach to integrating LanceDB vector search into rmcp-based MCP servers, with specific focus on the Taproot data analysis/BigQuery tool architecture.

## Overview

LanceDB is a serverless vector database built on the Lance columnar format. In your Taproot server, it will complement:
- **Tantivy**: Full-text search for exact keyword matching
- **Petgraph**: Graph relationships and traversals
- **LanceDB**: Semantic search and similarity matching

## Architecture Design

### Three-Database Pattern

```
Query Flow:
User Query → Routing Logic
              ├→ Tantivy (keyword search)
              ├→ Petgraph (relationship traversal)
              └→ LanceDB (semantic similarity)
                   ↓
            Merge & Rank Results
```

### Data Model

Each record in LanceDB should include:
- **Vector embedding**: Dense vector representation (384-1536 dimensions)
- **Source metadata**: Document ID, type, location
- **Graph metadata**: Node IDs, relationship hints for cross-database queries
- **Content snapshot**: Enough text for context/preview
- **Timestamps**: Created/updated for freshness

## Dependencies

Add to your `Cargo.toml`:

```toml
[dependencies]
# LanceDB and Arrow
lancedb = "0.10"
arrow-array = "53"
arrow-schema = "53"

# Embedding generation (choose one or both)
fastembed = "4"  # Local embeddings, no API needed
reqwest = { version = "0.12", features = ["json"] }  # For OpenAI API

# Async runtime
tokio = { version = "1", features = ["full"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Error handling
anyhow = "1"
thiserror = "1"
```

## Data Structures

### Core Schema

```rust
use arrow_array::{RecordBatch, RecordBatchIterator};
use arrow_schema::{DataType, Field, Schema};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Main document structure for LanceDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Unique identifier
    pub id: String,
    
    /// Document type (e.g., "concept_card", "guide", "source_doc")
    pub doc_type: String,
    
    /// Source text or content snapshot
    pub content: String,
    
    /// Vector embedding (will be stored as FixedSizeList in Arrow)
    pub embedding: Vec<f32>,
    
    /// Graph node IDs this document relates to
    pub graph_node_ids: Vec<String>,
    
    /// Metadata as JSON string
    pub metadata: String,
    
    /// Creation timestamp (Unix epoch)
    pub created_at: i64,
    
    /// Last update timestamp
    pub updated_at: i64,
}

/// Concept card with full graph modeling metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptCard {
    pub id: String,
    pub title: String,
    pub description: String,
    pub content: String,
    
    // Graph metadata
    pub related_concepts: Vec<String>,
    pub parent_concepts: Vec<String>,
    pub child_concepts: Vec<String>,
    pub tags: Vec<String>,
    
    // Source tracking
    pub source_docs: Vec<String>,
    pub confidence_score: f32,
    
    pub created_at: i64,
    pub updated_at: i64,
}

impl ConceptCard {
    /// Convert to Document for LanceDB storage
    pub fn to_document(&self, embedding: Vec<f32>) -> Document {
        let mut graph_node_ids = vec![self.id.clone()];
        graph_node_ids.extend(self.related_concepts.clone());
        
        Document {
            id: self.id.clone(),
            doc_type: "concept_card".to_string(),
            content: format!("{}\n\n{}\n\n{}", 
                self.title, 
                self.description, 
                self.content
            ),
            embedding,
            graph_node_ids,
            metadata: serde_json::to_string(self).unwrap_or_default(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}
```

## LanceDB Manager

### Core Manager Implementation

```rust
use lancedb::{Connection, Table};
use lancedb::query::{ExecutableQuery, QueryBase};
use anyhow::{Context, Result};

pub struct LanceDBManager {
    connection: Connection,
    embedding_dim: usize,
}

impl LanceDBManager {
    /// Initialize LanceDB connection
    pub async fn new(db_path: &str, embedding_dim: usize) -> Result<Self> {
        let connection = lancedb::connect(db_path)
            .execute()
            .await
            .context("Failed to connect to LanceDB")?;
        
        Ok(Self {
            connection,
            embedding_dim,
        })
    }
    
    /// Create or get table with proper schema
    pub async fn get_or_create_table(&self, table_name: &str) -> Result<Table> {
        let schema = self.create_schema();
        
        // Check if table exists
        let table_names = self.connection.table_names().execute().await?;
        
        if table_names.contains(&table_name.to_string()) {
            self.connection.open_table(table_name).execute().await
                .context("Failed to open existing table")
        } else {
            // Create empty table with schema
            let empty_batch = RecordBatch::new_empty(Arc::new(schema));
            let batches = RecordBatchIterator::new(
                vec![Ok(empty_batch)],
                Arc::new(self.create_schema()),
            );
            
            self.connection
                .create_table(table_name, Box::new(batches))
                .execute()
                .await
                .context("Failed to create table")
        }
    }
    
    /// Create Arrow schema for documents
    fn create_schema(&self) -> Schema {
        Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("doc_type", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
            Field::new(
                "embedding",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    self.embedding_dim as i32,
                ),
                false,
            ),
            Field::new(
                "graph_node_ids",
                DataType::List(Arc::new(Field::new("item", DataType::Utf8, true))),
                false,
            ),
            Field::new("metadata", DataType::Utf8, false),
            Field::new("created_at", DataType::Int64, false),
            Field::new("updated_at", DataType::Int64, false),
        ])
    }
    
    /// Add documents to table
    pub async fn add_documents(
        &self,
        table_name: &str,
        documents: Vec<Document>,
    ) -> Result<()> {
        let table = self.get_or_create_table(table_name).await?;
        
        // Convert documents to RecordBatch
        let batch = self.documents_to_record_batch(documents)?;
        let batches = RecordBatchIterator::new(
            vec![Ok(batch)],
            Arc::new(self.create_schema()),
        );
        
        table.add(Box::new(batches)).execute().await
            .context("Failed to add documents to table")?;
        
        Ok(())
    }
    
    /// Convert documents to Arrow RecordBatch
    fn documents_to_record_batch(&self, documents: Vec<Document>) -> Result<RecordBatch> {
        use arrow_array::{
            StringArray, Int64Array, FixedSizeListArray, ListArray,
            Float32Array,
        };
        
        let ids: StringArray = documents.iter().map(|d| d.id.as_str()).collect();
        let doc_types: StringArray = documents.iter().map(|d| d.doc_type.as_str()).collect();
        let contents: StringArray = documents.iter().map(|d| d.content.as_str()).collect();
        let metadatas: StringArray = documents.iter().map(|d| d.metadata.as_str()).collect();
        let created_ats: Int64Array = documents.iter().map(|d| d.created_at).collect();
        let updated_ats: Int64Array = documents.iter().map(|d| d.updated_at).collect();
        
        // Build embeddings as FixedSizeListArray
        let embedding_values: Float32Array = documents
            .iter()
            .flat_map(|d| d.embedding.iter().copied())
            .collect();
        
        let embeddings = FixedSizeListArray::new(
            Arc::new(Field::new("item", DataType::Float32, true)),
            self.embedding_dim as i32,
            Arc::new(embedding_values),
            None,
        );
        
        // Build graph_node_ids as ListArray
        let mut graph_ids_builder = arrow_array::builder::ListBuilder::new(
            arrow_array::builder::StringBuilder::new()
        );
        
        for doc in &documents {
            for node_id in &doc.graph_node_ids {
                graph_ids_builder.values().append_value(node_id);
            }
            graph_ids_builder.append(true);
        }
        
        let graph_node_ids = graph_ids_builder.finish();
        
        RecordBatch::try_new(
            Arc::new(self.create_schema()),
            vec![
                Arc::new(ids),
                Arc::new(doc_types),
                Arc::new(contents),
                Arc::new(embeddings),
                Arc::new(graph_node_ids),
                Arc::new(metadatas),
                Arc::new(created_ats),
                Arc::new(updated_ats),
            ],
        ).context("Failed to create RecordBatch")
    }
    
    /// Vector similarity search
    pub async fn search(
        &self,
        table_name: &str,
        query_embedding: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<Document>> {
        let table = self.get_or_create_table(table_name).await?;
        
        let results = table
            .query()
            .nearest_to(&query_embedding)?
            .limit(limit)
            .execute()
            .await
            .context("Failed to execute vector search")?;
        
        // Convert results back to Documents
        self.record_batch_to_documents(results)
    }
    
    /// Hybrid search: filter by doc_type, then vector search
    pub async fn search_by_type(
        &self,
        table_name: &str,
        doc_type: &str,
        query_embedding: Vec<f32>,
        limit: usize,
    ) -> Result<Vec<Document>> {
        let table = self.get_or_create_table(table_name).await?;
        
        let results = table
            .query()
            .nearest_to(&query_embedding)?
            .filter(&format!("doc_type = '{}'", doc_type))?
            .limit(limit)
            .execute()
            .await
            .context("Failed to execute filtered search")?;
        
        self.record_batch_to_documents(results)
    }
    
    /// Convert RecordBatch back to Documents
    fn record_batch_to_documents(&self, batch: RecordBatch) -> Result<Vec<Document>> {
        use arrow_array::Array;
        
        let mut documents = Vec::new();
        let num_rows = batch.num_rows();
        
        for i in 0..num_rows {
            let id = batch.column(0)
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Failed to downcast id")?
                .value(i)
                .to_string();
            
            let doc_type = batch.column(1)
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Failed to downcast doc_type")?
                .value(i)
                .to_string();
            
            let content = batch.column(2)
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Failed to downcast content")?
                .value(i)
                .to_string();
            
            // Extract embedding
            let embedding_list = batch.column(3)
                .as_any()
                .downcast_ref::<FixedSizeListArray>()
                .context("Failed to downcast embedding")?;
            
            let embedding_values = embedding_list.value(i);
            let embedding_floats = embedding_values
                .as_any()
                .downcast_ref::<Float32Array>()
                .context("Failed to downcast embedding values")?;
            
            let embedding: Vec<f32> = (0..embedding_floats.len())
                .map(|j| embedding_floats.value(j))
                .collect();
            
            // Extract graph_node_ids
            let graph_list = batch.column(4)
                .as_any()
                .downcast_ref::<ListArray>()
                .context("Failed to downcast graph_node_ids")?;
            
            let graph_values = graph_list.value(i);
            let graph_strings = graph_values
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Failed to downcast graph values")?;
            
            let graph_node_ids: Vec<String> = (0..graph_strings.len())
                .map(|j| graph_strings.value(j).to_string())
                .collect();
            
            let metadata = batch.column(5)
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Failed to downcast metadata")?
                .value(i)
                .to_string();
            
            let created_at = batch.column(6)
                .as_any()
                .downcast_ref::<Int64Array>()
                .context("Failed to downcast created_at")?
                .value(i);
            
            let updated_at = batch.column(7)
                .as_any()
                .downcast_ref::<Int64Array>()
                .context("Failed to downcast updated_at")?
                .value(i);
            
            documents.push(Document {
                id,
                doc_type,
                content,
                embedding,
                graph_node_ids,
                metadata,
                created_at,
                updated_at,
            });
        }
        
        Ok(documents)
    }
}
```

## Embedding Generation

### FastEmbed (Local, No API)

```rust
use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};

pub struct EmbeddingGenerator {
    model: TextEmbedding,
}

impl EmbeddingGenerator {
    pub fn new() -> Result<Self> {
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::AllMiniLML6V2)
                .with_show_download_progress(true)
        ).context("Failed to initialize embedding model")?;
        
        Ok(Self { model })
    }
    
    pub fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let embeddings = self.model
            .embed(vec![text], None)
            .context("Failed to generate embedding")?;
        
        Ok(embeddings.into_iter().next()
            .context("No embedding generated")?
            .into_iter()
            .collect())
    }
    
    pub fn batch_generate(&self, texts: Vec<&str>) -> Result<Vec<Vec<f32>>> {
        let embeddings = self.model
            .embed(texts, None)
            .context("Failed to generate batch embeddings")?;
        
        Ok(embeddings.into_iter()
            .map(|e| e.into_iter().collect())
            .collect())
    }
    
    pub fn embedding_dim(&self) -> usize {
        384 // AllMiniLML6V2 produces 384-dimensional embeddings
    }
}
```

### OpenAI Embeddings (API-based, Higher Quality)

```rust
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct OpenAIEmbeddingRequest {
    input: Vec<String>,
    model: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIEmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

pub struct OpenAIEmbedding {
    client: Client,
    api_key: String,
    model: String,
}

impl OpenAIEmbedding {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model: "text-embedding-3-small".to_string(), // 1536 dimensions
        }
    }
    
    pub async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let embeddings = self.batch_generate(vec![text]).await?;
        Ok(embeddings.into_iter().next().unwrap())
    }
    
    pub async fn batch_generate(&self, texts: Vec<&str>) -> Result<Vec<Vec<f32>>> {
        let request = OpenAIEmbeddingRequest {
            input: texts.iter().map(|s| s.to_string()).collect(),
            model: self.model.clone(),
        };
        
        let response = self.client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .context("Failed to send request to OpenAI")?
            .json::<OpenAIEmbeddingResponse>()
            .await
            .context("Failed to parse OpenAI response")?;
        
        Ok(response.data.into_iter().map(|d| d.embedding).collect())
    }
    
    pub fn embedding_dim(&self) -> usize {
        1536 // text-embedding-3-small
    }
}
```

## Population Strategy

### Batch Ingestion Pipeline

```rust
pub struct IngestionPipeline {
    lancedb: LanceDBManager,
    embedder: EmbeddingGenerator,
    batch_size: usize,
}

impl IngestionPipeline {
    pub fn new(lancedb: LanceDBManager, embedder: EmbeddingGenerator) -> Self {
        Self {
            lancedb,
            embedder,
            batch_size: 100,
        }
    }
    
    /// Ingest concept cards with graph metadata
    pub async fn ingest_concept_cards(
        &self,
        cards: Vec<ConceptCard>,
    ) -> Result<()> {
        for chunk in cards.chunks(self.batch_size) {
            let mut documents = Vec::new();
            
            for card in chunk {
                let content = format!("{}\n\n{}\n\n{}", 
                    card.title, 
                    card.description, 
                    card.content
                );
                
                let embedding = self.embedder.generate_embedding(&content)?;
                documents.push(card.to_document(embedding));
            }
            
            self.lancedb.add_documents("concepts", documents).await?;
        }
        
        Ok(())
    }
    
    /// Ingest source documents
    pub async fn ingest_source_documents(
        &self,
        docs: Vec<SourceDocument>,
    ) -> Result<()> {
        for chunk in docs.chunks(self.batch_size) {
            let texts: Vec<&str> = chunk.iter()
                .map(|d| d.content.as_str())
                .collect();
            
            let embeddings = self.embedder.batch_generate(texts)?;
            
            let documents: Vec<Document> = chunk.iter()
                .zip(embeddings.iter())
                .map(|(doc, embedding)| doc.to_document(embedding.clone()))
                .collect();
            
            self.lancedb.add_documents("source_docs", documents).await?;
        }
        
        Ok(())
    }
    
    /// Incremental update: add or update single document
    pub async fn upsert_document(
        &self,
        table_name: &str,
        content: &str,
        doc_type: &str,
        metadata: serde_json::Value,
        graph_node_ids: Vec<String>,
    ) -> Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let embedding = self.embedder.generate_embedding(content)?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;
        
        let document = Document {
            id: id.clone(),
            doc_type: doc_type.to_string(),
            content: content.to_string(),
            embedding,
            graph_node_ids,
            metadata: serde_json::to_string(&metadata)?,
            created_at: now,
            updated_at: now,
        };
        
        self.lancedb.add_documents(table_name, vec![document]).await?;
        Ok(id)
    }
}

/// Source document structure
#[derive(Debug, Clone)]
pub struct SourceDocument {
    pub id: String,
    pub content: String,
    pub source_type: String,
    pub metadata: serde_json::Value,
}

impl SourceDocument {
    pub fn to_document(&self, embedding: Vec<f32>) -> Document {
        Document {
            id: self.id.clone(),
            doc_type: self.source_type.clone(),
            content: self.content.clone(),
            embedding,
            graph_node_ids: vec![self.id.clone()],
            metadata: serde_json::to_string(&self.metadata).unwrap_or_default(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            updated_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        }
    }
}
```

## MCP Integration

### Tool Definitions

```rust
use serde_json::json;

pub fn vector_search_tool_schema() -> serde_json::Value {
    json!({
        "name": "vector_search",
        "description": "Search documents using semantic similarity",
        "inputSchema": {
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Natural language search query"
                },
                "doc_type": {
                    "type": "string",
                    "description": "Optional: filter by document type (concept_card, guide, source_doc)",
                    "enum": ["concept_card", "guide", "source_doc"]
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of results to return",
                    "default": 10,
                    "minimum": 1,
                    "maximum": 100
                }
            },
            "required": ["query"]
        }
    })
}

pub fn add_document_tool_schema() -> serde_json::Value {
    json!({
        "name": "add_document",
        "description": "Add a new document to the vector database",
        "inputSchema": {
            "type": "object",
            "properties": {
                "content": {
                    "type": "string",
                    "description": "Document content to index"
                },
                "doc_type": {
                    "type": "string",
                    "description": "Type of document"
                },
                "metadata": {
                    "type": "object",
                    "description": "Additional metadata as JSON object"
                },
                "graph_node_ids": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Related graph node IDs"
                }
            },
            "required": ["content", "doc_type"]
        }
    })
}
```

### Tool Handlers

```rust
use serde_json::Value;

pub struct VectorSearchHandler {
    pipeline: IngestionPipeline,
}

impl VectorSearchHandler {
    pub async fn handle_vector_search(&self, params: Value) -> Result<Value> {
        let query = params["query"].as_str()
            .context("Missing query parameter")?;
        
        let limit = params["limit"].as_u64().unwrap_or(10) as usize;
        
        let doc_type = params["doc_type"].as_str();
        
        // Generate query embedding
        let query_embedding = self.pipeline.embedder.generate_embedding(query)?;
        
        // Perform search
        let results = if let Some(dt) = doc_type {
            self.pipeline.lancedb
                .search_by_type("all_docs", dt, query_embedding, limit)
                .await?
        } else {
            self.pipeline.lancedb
                .search("all_docs", query_embedding, limit)
                .await?
        };
        
        // Format results
        let results_json: Vec<Value> = results.iter()
            .map(|doc| json!({
                "id": doc.id,
                "doc_type": doc.doc_type,
                "content": doc.content,
                "graph_node_ids": doc.graph_node_ids,
                "metadata": serde_json::from_str::<Value>(&doc.metadata)
                    .unwrap_or(json!({})),
            }))
            .collect();
        
        Ok(json!({
            "results": results_json,
            "count": results.len(),
        }))
    }
    
    pub async fn handle_add_document(&self, params: Value) -> Result<Value> {
        let content = params["content"].as_str()
            .context("Missing content parameter")?;
        
        let doc_type = params["doc_type"].as_str()
            .context("Missing doc_type parameter")?;
        
        let metadata = params["metadata"].clone();
        
        let graph_node_ids: Vec<String> = params["graph_node_ids"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        
        let doc_id = self.pipeline.upsert_document(
            "all_docs",
            content,
            doc_type,
            metadata,
            graph_node_ids,
        ).await?;
        
        Ok(json!({
            "id": doc_id,
            "status": "success"
        }))
    }
}
```

## Unified Search Strategy

### Combining All Three Databases

```rust
pub struct UnifiedSearchEngine {
    lancedb: LanceDBManager,
    tantivy_index: TantivyIndex, // Your Tantivy implementation
    graph_db: PetgraphDB,         // Your Petgraph implementation
    embedder: EmbeddingGenerator,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: String,
    pub content: String,
    pub score: f32,
    pub source: SearchSource,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone)]
pub enum SearchSource {
    Vector,
    FullText,
    Graph,
    Hybrid,
}

impl UnifiedSearchEngine {
    /// Intelligent query routing based on query characteristics
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        // Analyze query to determine best search strategy
        let strategy = self.analyze_query(query);
        
        match strategy {
            QueryStrategy::Keywords => {
                // Exact keyword search via Tantivy
                self.tantivy_search(query, limit).await
            }
            QueryStrategy::Semantic => {
                // Semantic search via LanceDB
                self.vector_search(query, limit).await
            }
            QueryStrategy::Relationships => {
                // Graph traversal via Petgraph
                self.graph_search(query, limit).await
            }
            QueryStrategy::Hybrid => {
                // Combine all three with ranking
                self.hybrid_search(query, limit).await
            }
        }
    }
    
    fn analyze_query(&self, query: &str) -> QueryStrategy {
        // Simple heuristics - can be made more sophisticated
        if query.contains("related to") || query.contains("connected to") {
            QueryStrategy::Relationships
        } else if query.len() < 20 && query.split_whitespace().count() < 4 {
            QueryStrategy::Keywords
        } else if query.contains("similar") || query.contains("like") {
            QueryStrategy::Semantic
        } else {
            QueryStrategy::Hybrid
        }
    }
    
    async fn vector_search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let embedding = self.embedder.generate_embedding(query)?;
        let docs = self.lancedb.search("all_docs", embedding, limit).await?;
        
        Ok(docs.into_iter().map(|doc| SearchResult {
            id: doc.id,
            content: doc.content,
            score: 0.0, // LanceDB returns results ordered by similarity
            source: SearchSource::Vector,
            metadata: serde_json::from_str(&doc.metadata).unwrap_or_default(),
        }).collect())
    }
    
    async fn hybrid_search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        // Run all searches in parallel
        let (vector_results, tantivy_results, graph_results) = tokio::join!(
            self.vector_search(query, limit * 2),
            self.tantivy_search(query, limit * 2),
            self.graph_search(query, limit * 2),
        );
        
        // Merge and rank results using reciprocal rank fusion
        let mut all_results = Vec::new();
        
        if let Ok(results) = vector_results {
            all_results.extend(results);
        }
        if let Ok(results) = tantivy_results {
            all_results.extend(results);
        }
        if let Ok(results) = graph_results {
            all_results.extend(results);
        }
        
        // Deduplicate and rank
        let mut result_map: std::collections::HashMap<String, SearchResult> = 
            std::collections::HashMap::new();
        
        for result in all_results {
            result_map.entry(result.id.clone())
                .and_modify(|e| e.score += result.score)
                .or_insert(result);
        }
        
        let mut final_results: Vec<SearchResult> = result_map.into_values().collect();
        final_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        final_results.truncate(limit);
        
        Ok(final_results)
    }
    
    // Implement tantivy_search and graph_search based on your implementations
    async fn tantivy_search(&self, _query: &str, _limit: usize) -> Result<Vec<SearchResult>> {
        // Your Tantivy implementation
        Ok(vec![])
    }
    
    async fn graph_search(&self, _query: &str, _limit: usize) -> Result<Vec<SearchResult>> {
        // Your Petgraph implementation
        Ok(vec![])
    }
}

#[derive(Debug)]
enum QueryStrategy {
    Keywords,
    Semantic,
    Relationships,
    Hybrid,
}
```

## Best Practices

### 1. Embedding Model Selection

**Local (FastEmbed)**:
- ✅ No API costs
- ✅ Privacy (no data leaves server)
- ✅ Fast for moderate volumes
- ❌ Lower quality than OpenAI
- **Best for**: Development, cost-sensitive deployments, privacy requirements

**OpenAI**:
- ✅ Higher quality embeddings
- ✅ Better semantic understanding
- ❌ API costs
- ❌ Latency for API calls
- **Best for**: Production, high-quality search requirements

### 2. Chunking Strategy

For long documents, split into chunks:

```rust
pub fn chunk_document(text: &str, chunk_size: usize, overlap: usize) -> Vec<String> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut chunks = Vec::new();
    
    let mut i = 0;
    while i < words.len() {
        let end = (i + chunk_size).min(words.len());
        let chunk = words[i..end].join(" ");
        chunks.push(chunk);
        
        i += chunk_size - overlap;
    }
    
    chunks
}
```

### 3. Metadata Strategy

Store rich metadata for filtering:

```rust
#[derive(Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub source_file: String,
    pub creation_date: String,
    pub author: Option<String>,
    pub tags: Vec<String>,
    pub confidence: f32,
    pub processing_version: String,
}
```

### 4. Update Strategy

- **Immutable**: Create new versions rather than updating
- **Versioned**: Keep `version` field in metadata
- **Tombstone**: Mark deleted items with `deleted: true` flag
- **Rebuild**: Periodically rebuild index for compaction

### 5. Performance Optimization

```rust
// Create index for faster filtering
async fn create_index(table: &Table) -> Result<()> {
    table
        .create_index(&["doc_type"], lancedb::index::Index::BTree)
        .execute()
        .await?;
    
    Ok(())
}

// Use appropriate distance metrics
// L2 (Euclidean): Default, good for most embeddings
// Cosine: Better for normalized vectors
// Dot product: Fastest, use with normalized embeddings
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_lancedb_initialization() {
        let db = LanceDBManager::new("test_db", 384).await.unwrap();
        let table = db.get_or_create_table("test_table").await.unwrap();
        assert!(table.name() == "test_table");
    }
    
    #[tokio::test]
    async fn test_document_insertion() {
        let db = LanceDBManager::new("test_db", 384).await.unwrap();
        let embedder = EmbeddingGenerator::new().unwrap();
        
        let content = "Test document content";
        let embedding = embedder.generate_embedding(content).unwrap();
        
        let doc = Document {
            id: "test-1".to_string(),
            doc_type: "test".to_string(),
            content: content.to_string(),
            embedding,
            graph_node_ids: vec![],
            metadata: "{}".to_string(),
            created_at: 0,
            updated_at: 0,
        };
        
        db.add_documents("test_table", vec![doc]).await.unwrap();
    }
    
    #[tokio::test]
    async fn test_vector_search() {
        let db = LanceDBManager::new("test_db", 384).await.unwrap();
        let embedder = EmbeddingGenerator::new().unwrap();
        
        // Add test documents
        let docs = vec![
            "The quick brown fox jumps over the lazy dog",
            "Machine learning is a subset of artificial intelligence",
            "Python is a popular programming language",
        ];
        
        let mut documents = Vec::new();
        for (i, content) in docs.iter().enumerate() {
            let embedding = embedder.generate_embedding(content).unwrap();
            documents.push(Document {
                id: format!("doc-{}", i),
                doc_type: "test".to_string(),
                content: content.to_string(),
                embedding,
                graph_node_ids: vec![],
                metadata: "{}".to_string(),
                created_at: 0,
                updated_at: 0,
            });
        }
        
        db.add_documents("test_table", documents).await.unwrap();
        
        // Search
        let query_embedding = embedder.generate_embedding("AI and ML").unwrap();
        let results = db.search("test_table", query_embedding, 1).await.unwrap();
        
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("Machine learning"));
    }
}
```

## Monitoring and Maintenance

### Metrics to Track

```rust
pub struct LanceDBMetrics {
    pub total_documents: usize,
    pub total_tables: usize,
    pub average_query_time_ms: f64,
    pub index_size_bytes: u64,
    pub last_update: i64,
}

impl LanceDBManager {
    pub async fn get_metrics(&self) -> Result<LanceDBMetrics> {
        let table_names = self.connection.table_names().execute().await?;
        
        // Implement metric collection
        Ok(LanceDBMetrics {
            total_documents: 0,
            total_tables: table_names.len(),
            average_query_time_ms: 0.0,
            index_size_bytes: 0,
            last_update: 0,
        })
    }
}
```

## Migration and Backup

```rust
/// Export table to JSON for backup
pub async fn export_table(table: &Table, output_path: &str) -> Result<()> {
    let results = table.query().limit(1000000).execute().await?;
    
    // Convert to JSON and write
    let json = serde_json::to_string_pretty(&results)?;
    std::fs::write(output_path, json)?;
    
    Ok(())
}

/// Import from backup
pub async fn import_from_backup(
    db: &LanceDBManager,
    table_name: &str,
    backup_path: &str,
) -> Result<()> {
    let json = std::fs::read_to_string(backup_path)?;
    let documents: Vec<Document> = serde_json::from_str(&json)?;
    
    db.add_documents(table_name, documents).await?;
    
    Ok(())
}
```

## Summary

This guide provides a complete foundation for integrating LanceDB into your Rust MCP server. Key points:

1. **Use FastEmbed for local, cost-effective embeddings** or OpenAI for higher quality
2. **Structure documents with graph metadata** to enable cross-database queries
3. **Implement batch ingestion** for efficient initial population
4. **Support incremental updates** for real-time additions
5. **Combine with Tantivy and Petgraph** for comprehensive search capabilities
6. **Monitor and maintain** the vector database for optimal performance

The three-database architecture (LanceDB + Tantivy + Petgraph) provides:
- **Semantic search** (LanceDB): "Find documents similar to X"
- **Keyword search** (Tantivy): "Find exact mentions of Y"
- **Relationship queries** (Petgraph): "What's connected to Z?"

This creates a powerful, flexible search system for the Taproot data analysis tool.
