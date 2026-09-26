use std::path::Path;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use kura_schema::{Part, PartIndex, Shelf, load_dir, to_published_json};
use kura_server::{Health, Registry, router};
use tower::ServiceExt;

fn registry_dir() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../registry"))
}

fn repo_parts() -> Vec<Part> {
    load_dir(registry_dir()).expect("registry is valid")
}

fn minhash() -> Part {
    repo_parts()
        .into_iter()
        .find(|p| p.name == "minhash")
        .expect("registry has minhash")
}

/// 検索のテスト用に固定した2件。registry の件数が増えても結果が変わらないようにする。
fn fixture_parts() -> Vec<Part> {
    let mut parts = vec![minhash()];
    let mut hash = parts[0].clone();
    hash.name = "blake3-hash".into();
    hash.title = "BLAKE3 Hash".into();
    hash.description = "Fast cryptographic hashing for file integrity checks.".into();
    hash.shelves = vec![Shelf::Security];
    hash.reference.name = "hashlib".into();
    parts.push(hash);
    parts
}

fn app(parts: Vec<Part>) -> Router {
    router(Registry::new(parts).unwrap())
}

async fn get(app: Router, uri: &str) -> (StatusCode, axum::http::HeaderMap, String) {
    get_with(app, Request::get(uri).body(Body::empty()).unwrap()).await
}

async fn get_with(app: Router, req: Request<Body>) -> (StatusCode, axum::http::HeaderMap, String) {
    let res = app.oneshot(req).await.unwrap();
    let status = res.status();
    let headers = res.headers().clone();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        headers,
        String::from_utf8_lossy(&bytes).into_owned(),
    )
}

#[tokio::test]
async fn health_reports_part_count() {
    let (status, _, body) = get(app(repo_parts()), "/health").await;
    assert_eq!(status, StatusCode::OK);
    let health: Health = serde_json::from_str(&body).unwrap();
    assert_eq!(health.status, "ok");
    assert_eq!(health.parts, repo_parts().len());
}

#[tokio::test]
async fn index_matches_the_static_site() {
    let parts = repo_parts();
    let (status, headers, body) = get(app(parts.clone()), "/r/index.json").await;
    assert_eq!(status, StatusCode::OK);
    assert!(
        headers[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .starts_with("application/json")
    );
    assert_eq!(body, to_published_json(&PartIndex::new(&parts)).unwrap());
}

#[tokio::test]
async fn part_matches_the_static_site() {
    let (status, _, body) = get(app(repo_parts()), "/r/minhash.json").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, to_published_json(&minhash().published()).unwrap());

    // kura-registry build の出力があれば、それともバイト単位で一致すること
    let built = registry_dir().join("../site/public/r/minhash.json");
    if let Ok(file) = std::fs::read_to_string(built) {
        assert_eq!(body, file);
    }
}

#[tokio::test]
async fn unknown_part_is_404_with_json_error() {
    let (status, _, body) = get(app(repo_parts()), "/r/nope.json").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let err: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(err["error"], "part \"nope\" not found");
}

#[tokio::test]
async fn part_without_json_extension_is_404() {
    let (status, _, _) = get(app(repo_parts()), "/r/minhash").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn unknown_route_is_404() {
    let (status, _, body) = get(app(repo_parts()), "/nope").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(body.contains("\"error\""));
}

async fn search(uri: &str) -> (StatusCode, Vec<String>) {
    let (status, _, body) = get(app(fixture_parts()), uri).await;
    let names = serde_json::from_str::<PartIndex>(&body)
        .map(|i| i.parts.into_iter().map(|p| p.name).collect())
        .unwrap_or_default();
    (status, names)
}

#[tokio::test]
async fn search_without_filters_returns_everything() {
    assert_eq!(
        search("/api/parts").await,
        (StatusCode::OK, vec!["blake3-hash".into(), "minhash".into()])
    );
}

#[tokio::test]
async fn search_filters_by_shelf() {
    assert_eq!(search("/api/parts?shelf=ai").await.1, ["minhash"]);
    assert_eq!(search("/api/parts?shelf=security").await.1, ["blake3-hash"]);
}

#[tokio::test]
async fn search_filters_by_text_case_insensitively() {
    assert_eq!(
        search("/api/parts?q=HASH").await.1,
        ["blake3-hash", "minhash"]
    );
    assert_eq!(search("/api/parts?q=integrity").await.1, ["blake3-hash"]);
    assert_eq!(search("/api/parts?q=datasketch").await.1, ["minhash"]);
    assert_eq!(
        search("/api/parts?shelf=security&q=hash").await.1,
        ["blake3-hash"]
    );
    assert!(search("/api/parts?q=zzz").await.1.is_empty());
}

#[tokio::test]
async fn search_rejects_unknown_shelf() {
    let (status, _, body) = get(app(fixture_parts()), "/api/parts?shelf=web").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.contains("unknown shelf"), "{body}");
}

#[tokio::test]
async fn cors_allows_any_origin() {
    let req = Request::get("/r/index.json")
        .header(header::ORIGIN, "https://example.com")
        .body(Body::empty())
        .unwrap();
    let (_, headers, _) = get_with(app(repo_parts()), req).await;
    assert_eq!(headers[header::ACCESS_CONTROL_ALLOW_ORIGIN], "*");
}

#[tokio::test]
async fn responses_are_compressed_when_accepted() {
    let req = Request::get("/r/minhash.json")
        .header(header::ACCEPT_ENCODING, "gzip")
        .body(Body::empty())
        .unwrap();
    let res = app(repo_parts()).oneshot(req).await.unwrap();
    assert_eq!(res.headers()[header::CONTENT_ENCODING], "gzip");
}

#[test]
fn loading_an_invalid_registry_fails() {
    let dir = std::env::temp_dir().join(format!("kura-server-invalid-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("broken.json"), "{ not json").unwrap();
    assert!(Registry::load(&dir).is_err());
    std::fs::remove_dir_all(dir).ok();
}
