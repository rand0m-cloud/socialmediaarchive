use anyhow::{Context, Result};
use qdrant_client::{
    prelude::*,
    qdrant::{
        point_id::PointIdOptions, vectors_config::Config, with_payload_selector::SelectorOptions,
        RecommendPoints, VectorParams, VectorsConfig, WithPayloadSelector,
    },
};
use serde_json::{from_value, to_value};
use tracing::instrument;

use crate::{Entry, SearchResult};

pub struct VectorDbClient {
    client: QdrantClient,
}

impl VectorDbClient {
    pub fn new() -> Result<Self> {
        Ok(Self {
            client: QdrantClientConfig::from_url(
                &std::env::var("QDRANT_URL").context("QDRANT_URL env variable not set")?,
            )
            .build()
            .context("building QdrantClient failed")?,
        })
    }

    #[instrument(skip_all)]
    pub async fn init(&mut self) -> Result<()> {
        if self
            .client
            .has_collection("my_collection")
            .await
            .context("querying qdrant failed")?
        {
            return Ok(());
        }

        self.client
            .create_collection(&CreateCollection {
                collection_name: "my_collection".into(),
                vectors_config: Some(VectorsConfig {
                    config: Some(Config::Params(VectorParams {
                        size: 1536,
                        distance: Distance::Cosine as i32,
                        ..Default::default()
                    })),
                }),
                ..Default::default()
            })
            .await
            .context("initializing qdrant client failed")?;
        Ok(())
    }

    #[instrument(skip_all)]
    pub async fn insert_vector(
        &mut self,
        vector: Vec<f32>,
        payload: serde_json::Value,
    ) -> Result<()> {
        self.client
            .upsert_points(
                "my_collection",
                vec![PointStruct::new(
                    uuid::Uuid::new_v4().to_string(),
                    vector,
                    from_value(payload)?,
                )],
                None,
            )
            .await
            .context("inserting vector into db failed")?;
        Ok(())
    }

    #[instrument(skip_all)]
    pub async fn search(&mut self, embeddings: Vec<f32>) -> Result<SearchResult> {
        let res = self
            .client
            .recommend(&RecommendPoints {
                collection_name: "my_collection".to_string(),
                limit: 100,
                positive_vectors: vec![embeddings.into()],
                with_payload: Some(WithPayloadSelector {
                    selector_options: Some(SelectorOptions::Enable(true)),
                }),
                ..Default::default()
            })
            .await
            .context("failed to search from qdrant")?;

        Ok(SearchResult(
            res.result
                .into_iter()
                .map(|x| Entry {
                    id: match x.id.unwrap().point_id_options.unwrap() {
                        PointIdOptions::Num(n) => n.to_string(),
                        PointIdOptions::Uuid(n) => n,
                    },
                    score: x.score,
                    payload: to_value(x.payload).unwrap(),
                })
                .collect(),
        ))
    }
}
