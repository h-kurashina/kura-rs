//! kura のレジストリ API。
//!
//! 起動時に `registry/*.json` を kura-schema で検証してメモリに持ち、
//! 静的サイトと同じ JSON（[`kura_schema::to_published_json`]）を返す。

mod error;

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use axum::extract::{Path as UrlPath, Query, State};
use axum::http::{HeaderValue, Method, header};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use kura_schema::{
    Part, PartIndex, PartSummary, RegistryError, Shelf, load_dir, to_published_json,
};
use serde::{Deserialize, Serialize};
use tower_http::compression::CompressionLayer;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

pub use error::ApiError;

/// 起動時に読み込んだレジストリ。配信する JSON は先に作っておく。
#[derive(Debug)]
pub struct Registry {
    parts: Vec<Part>,
    index: PartIndex,
    index_json: String,
    part_json: HashMap<String, String>,
}

impl Registry {
    pub fn new(parts: Vec<Part>) -> Result<Self, serde_json::Error> {
        let index = PartIndex::new(&parts);
        let index_json = to_published_json(&index)?;
        let part_json = parts
            .iter()
            .map(|p| Ok((p.name.clone(), to_published_json(&p.published())?)))
            .collect::<Result<_, serde_json::Error>>()?;
        Ok(Self {
            parts,
            index,
            index_json,
            part_json,
        })
    }

    /// ディレクトリから読み込んで検証する。1 件でも不正なら失敗する。
    pub fn load(dir: &Path) -> Result<Self, LoadRegistryError> {
        Ok(Self::new(load_dir(dir)?)?)
    }

    pub fn len(&self) -> usize {
        self.parts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LoadRegistryError {
    #[error(transparent)]
    Invalid(#[from] RegistryError),
    #[error("cannot serialize registry: {0}")]
    Serialize(#[from] serde_json::Error),
}

pub type AppState = Arc<Registry>;

/// ルーティングとミドルウェア（CORS・圧縮・トレース）をまとめたもの。
pub fn router(registry: Registry) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::HEAD]);

    Router::new()
        .route("/health", get(health))
        .route("/r/index.json", get(index))
        .route("/r/{file}", get(part))
        .route("/api/parts", get(search))
        .fallback(|| async { ApiError::RouteNotFound })
        .with_state(Arc::new(registry))
        .layer(CompressionLayer::new())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Health {
    pub status: String,
    pub parts: usize,
}

async fn health(State(reg): State<AppState>) -> Json<Health> {
    Json(Health {
        status: "ok".into(),
        parts: reg.len(),
    })
}

async fn index(State(reg): State<AppState>) -> Response {
    json_response(reg.index_json.clone())
}

async fn part(
    State(reg): State<AppState>,
    UrlPath(file): UrlPath<String>,
) -> Result<Response, ApiError> {
    let name = file.strip_suffix(".json").ok_or(ApiError::RouteNotFound)?;
    let json = reg
        .part_json
        .get(name)
        .ok_or_else(|| ApiError::PartNotFound(name.to_owned()))?;
    Ok(json_response(json.clone()))
}

/// `/api/parts` のクエリ。どちらも省略可。
#[derive(Debug, Default, Deserialize)]
pub struct SearchParams {
    /// `ai` または `security`
    pub shelf: Option<String>,
    /// 名前・タイトル・説明・参照実装名に対する部分一致（大文字小文字を区別しない）
    pub q: Option<String>,
}

async fn search(
    State(reg): State<AppState>,
    query: Result<Query<SearchParams>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<PartIndex>, ApiError> {
    let Query(params) = query.map_err(|e| ApiError::BadRequest(e.body_text()))?;
    let shelf = params
        .shelf
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(str::parse::<Shelf>)
        .transpose()
        .map_err(ApiError::BadRequest)?;
    let q = params
        .q
        .as_deref()
        .map(str::trim)
        .filter(|q| !q.is_empty())
        .map(str::to_lowercase);

    let parts = reg
        .index
        .parts
        .iter()
        .filter(|p| shelf.is_none_or(|s| p.shelves.contains(&s)))
        .filter(|p| q.as_deref().is_none_or(|q| matches_query(p, q)))
        .cloned()
        .collect();
    Ok(Json(PartIndex { parts }))
}

fn matches_query(p: &PartSummary, q: &str) -> bool {
    [&p.name, &p.title, &p.description, &p.reference]
        .iter()
        .any(|field| field.to_lowercase().contains(q))
}

fn json_response(body: String) -> Response {
    (
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json; charset=utf-8"),
        )],
        body,
    )
        .into_response()
}
