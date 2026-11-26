//! Grace's local HTTP API (spec §7.2): one API for the desktop window and
//! `grace serve`. It answers only requests addressed to 127.0.0.1 or localhost,
//! requires a random token for `/api/*`, and allows only known origins.

use crate::Redactor;
use crate::config::{Config, ToolConfig};
use crate::discover::{self, DiscoveryEnv};
use crate::scan::{self, ScanReport};
use crate::store::{SessionQuery, Store};
use axum::extract::{Path, Query, Request, State};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::future::Future;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

/// One file of the web UI, for `grace serve`.
pub struct UiFile {
    pub bytes: Cow<'static, [u8]>,
    pub mime: &'static str,
}

/// Looks up a UI file by path (`index.html`, `assets/app.js`, …).
pub type UiAssets = Arc<dyn Fn(&str) -> Option<UiFile> + Send + Sync>;

pub struct ApiOptions {
    pub grace_home: PathBuf,
    pub token: String,
    /// Origins besides the server itself that may call the API (the desktop window).
    pub allowed_origins: Vec<String>,
    pub discovery: DiscoveryEnv,
    /// The web UI to serve at `/`, if any.
    pub ui: Option<UiAssets>,
}

#[derive(Debug, Clone, Default, Serialize)]
struct ScanJob {
    running: bool,
    done: usize,
    total: usize,
    current: Option<String>,
    last: Option<ScanReport>,
    error: Option<String>,
}

struct AppState {
    opts: ApiOptions,
    scan: Mutex<ScanJob>,
}

impl AppState {
    fn job(&self) -> MutexGuard<'_, ScanJob> {
        self.scan
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn db_path(&self) -> PathBuf {
        self.opts.grace_home.join("grace.db")
    }
}

type Shared = Arc<AppState>;

/// A random 256-bit token, hex-encoded.
pub fn random_token() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("the operating system's random number generator works");
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn router(opts: ApiOptions) -> Router {
    let state: Shared = Arc::new(AppState {
        opts,
        scan: Mutex::new(ScanJob::default()),
    });
    Router::new()
        .route("/api/health", get(health))
        .route("/api/doctor", get(doctor))
        .route("/api/config", get(get_config).put(put_config))
        .route("/api/scan", get(scan_status).post(start_scan))
        .route("/api/sessions", get(list_sessions))
        .route("/api/sessions/{id}", get(session_detail))
        .fallback(ui_file)
        .layer(middleware::from_fn_with_state(state.clone(), guard))
        .with_state(state)
}

/// Serves `router` on `listener` until `shutdown` completes.
pub async fn run(
    listener: tokio::net::TcpListener,
    router: Router,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> std::io::Result<()> {
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown)
        .await
}

// ----- access checks -----

fn is_local_host(host: &str) -> bool {
    let name = if let Some(rest) = host.strip_prefix('[') {
        rest.split(']').next().unwrap_or_default()
    } else {
        host.rsplit_once(':').map_or(host, |(name, _)| name)
    };
    matches!(name, "127.0.0.1" | "localhost" | "::1")
}

fn origin_allowed(opts: &ApiOptions, origin: &str, host: &str) -> bool {
    origin == format!("http://{host}")
        || opts.allowed_origins.iter().any(|allowed| allowed == origin)
}

fn token_matches(headers: &HeaderMap, token: &str) -> bool {
    let Some(given) = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
    else {
        return false;
    };
    // Constant-time comparison.
    given.len() == token.len()
        && given
            .bytes()
            .zip(token.bytes())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0
}

fn with_cors(mut response: Response, origin: Option<&str>) -> Response {
    if let Some(origin) = origin.and_then(|o| HeaderValue::from_str(o).ok()) {
        let headers = response.headers_mut();
        headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);
        headers.insert(header::VARY, HeaderValue::from_static("Origin"));
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_HEADERS,
            HeaderValue::from_static("authorization, content-type"),
        );
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_METHODS,
            HeaderValue::from_static("GET, POST, PUT, OPTIONS"),
        );
        headers.insert(
            header::ACCESS_CONTROL_MAX_AGE,
            HeaderValue::from_static("600"),
        );
    }
    response
}

fn plain(status: StatusCode, message: &'static str) -> Response {
    (status, Json(json!({ "error": message }))).into_response()
}

async fn guard(State(state): State<Shared>, request: Request, next: Next) -> Response {
    let headers = request.headers();
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    if !is_local_host(&host) {
        return plain(
            StatusCode::FORBIDDEN,
            "Grace only answers requests sent to 127.0.0.1.",
        );
    }
    let origin = headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    if let Some(origin) = &origin
        && !origin_allowed(&state.opts, origin, &host)
    {
        return plain(
            StatusCode::FORBIDDEN,
            "This page isn't allowed to talk to Grace.",
        );
    }
    if request.method() == Method::OPTIONS {
        return with_cors(StatusCode::NO_CONTENT.into_response(), origin.as_deref());
    }
    if request.uri().path().starts_with("/api/") && !token_matches(headers, &state.opts.token) {
        let response = plain(StatusCode::UNAUTHORIZED, "Missing or wrong Grace token.");
        return with_cors(response, origin.as_deref());
    }
    let response = next.run(request).await;
    with_cors(response, origin.as_deref())
}

// ----- errors -----

enum ApiError {
    NotFound,
    Internal(anyhow::Error),
}

impl<E: Into<anyhow::Error>> From<E> for ApiError {
    fn from(err: E) -> Self {
        ApiError::Internal(err.into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::NotFound => plain(StatusCode::NOT_FOUND, "Not found."),
            ApiError::Internal(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("{err:#}") })),
            )
                .into_response(),
        }
    }
}

type ApiResult<T> = Result<Json<T>, ApiError>;

/// Runs blocking work (files, SQLite) off the async threads.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> anyhow::Result<T> + Send + 'static,
) -> Result<T, ApiError> {
    Ok(tokio::task::spawn_blocking(work).await??)
}

// ----- handlers -----

async fn health() -> Json<serde_json::Value> {
    Json(json!({ "ok": true, "version": env!("CARGO_PKG_VERSION") }))
}

async fn doctor(State(state): State<Shared>) -> ApiResult<discover::DoctorReport> {
    let env = state.opts.discovery.clone();
    Ok(Json(blocking(move || Ok(discover::doctor(&env))).await?))
}

async fn get_config(State(state): State<Shared>) -> ApiResult<Config> {
    let home = state.opts.grace_home.clone();
    Ok(Json(blocking(move || Config::load(&home)).await?))
}

#[derive(Debug, Deserialize)]
struct ConfigPatch {
    onboarding_done: Option<bool>,
    redact_emails: Option<bool>,
    tools: Option<BTreeMap<String, ToolConfig>>,
}

async fn put_config(
    State(state): State<Shared>,
    Json(patch): Json<ConfigPatch>,
) -> ApiResult<Config> {
    let home = state.opts.grace_home.clone();
    let config = blocking(move || {
        let mut config = Config::load(&home)?;
        if let Some(done) = patch.onboarding_done {
            config.onboarding_done = done;
        }
        if let Some(redact) = patch.redact_emails {
            config.redact_emails = redact;
        }
        if let Some(tools) = patch.tools {
            config.tools.extend(tools);
        }
        config.save(&home)?;
        Ok(config)
    })
    .await?;
    Ok(Json(config))
}

async fn scan_status(State(state): State<Shared>) -> Json<ScanJob> {
    Json(state.job().clone())
}

async fn start_scan(State(state): State<Shared>) -> Response {
    {
        let mut job = state.job();
        if job.running {
            return Json(json!({ "started": false, "running": true })).into_response();
        }
        *job = ScanJob {
            running: true,
            last: job.last.take(),
            ..ScanJob::default()
        };
    }
    let worker = state.clone();
    tokio::task::spawn_blocking(move || run_scan(&worker));
    (StatusCode::ACCEPTED, Json(json!({ "started": true }))).into_response()
}

fn run_scan(state: &AppState) {
    let result = (|| -> anyhow::Result<ScanReport> {
        let config = Config::load(&state.opts.grace_home)?;
        let mut store = Store::open(&state.db_path())?;
        let adapters = scan::adapters_for(&config, &state.opts.discovery);
        let redactor = Redactor::new(config.redact_emails);
        scan::scan(&mut store, &adapters, &redactor, &mut |progress| {
            let mut job = state.job();
            job.done = progress.done;
            job.total = progress.total;
            job.current = progress.current.clone();
        })
    })();
    let mut job = state.job();
    job.running = false;
    match result {
        Ok(report) => {
            job.last = Some(report);
            job.error = None;
        }
        Err(err) => job.error = Some(format!("{err:#}")),
    }
}

#[derive(Debug, Deserialize)]
struct SessionsParams {
    q: Option<String>,
    project: Option<String>,
    limit: Option<u32>,
}

async fn list_sessions(
    State(state): State<Shared>,
    Query(params): Query<SessionsParams>,
) -> ApiResult<Vec<crate::store::SessionRow>> {
    let db = state.db_path();
    let rows = blocking(move || {
        Store::open(&db)?.list_sessions(&SessionQuery {
            q: params.q,
            project: params.project,
            limit: params.limit.unwrap_or(200).min(1_000),
        })
    })
    .await?;
    Ok(Json(rows))
}

#[derive(Debug, Deserialize)]
struct DetailParams {
    after: Option<u64>,
    limit: Option<u32>,
}

async fn session_detail(
    State(state): State<Shared>,
    Path(id): Path<i64>,
    Query(params): Query<DetailParams>,
) -> ApiResult<crate::store::SessionDetail> {
    let db = state.db_path();
    let detail = blocking(move || {
        Store::open(&db)?.session_detail(id, params.after, params.limit.unwrap_or(200))
    })
    .await?;
    detail.map(Json).ok_or(ApiError::NotFound)
}

const UI_MISSING: &str = "<!doctype html><title>Grace</title><p>Grace's web UI isn't built yet. \
     Run <code>pnpm -C ui build</code>, then restart <code>grace serve</code>.</p>";

async fn ui_file(State(state): State<Shared>, request: Request) -> Response {
    let path = request.uri().path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    let Some(ui) = &state.opts.ui else {
        return plain(StatusCode::NOT_FOUND, "Not found.");
    };
    match ui(path) {
        Some(file) => {
            let cache = if path == "index.html" {
                "no-cache"
            } else {
                "public, max-age=31536000, immutable"
            };
            (
                [
                    (header::CONTENT_TYPE, file.mime),
                    (header::CACHE_CONTROL, cache),
                ],
                file.bytes,
            )
                .into_response()
        }
        None if path == "index.html" => (
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            UI_MISSING,
        )
            .into_response(),
        None => plain(StatusCode::NOT_FOUND, "Not found."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_hosts_only() {
        for host in [
            "127.0.0.1:4000",
            "localhost:4000",
            "[::1]:4000",
            "127.0.0.1",
        ] {
            assert!(is_local_host(host), "{host}");
        }
        for host in [
            "evil.example",
            "evil.example:4000",
            "127.0.0.1.evil.example:80",
            "",
        ] {
            assert!(!is_local_host(host), "{host}");
        }
    }

    #[test]
    fn tokens_are_long_and_random() {
        let (a, b) = (random_token(), random_token());
        assert_eq!(a.len(), 64);
        assert_ne!(a, b);
    }

    #[test]
    fn token_check() {
        let mut headers = HeaderMap::new();
        assert!(!token_matches(&headers, "abc"));
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_static("Bearer abc"),
        );
        assert!(token_matches(&headers, "abc"));
        assert!(!token_matches(&headers, "abd"));
        assert!(!token_matches(&headers, "abcd"));
    }
}
