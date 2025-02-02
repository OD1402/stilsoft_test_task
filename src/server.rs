use super::*;

pub async fn start_server(port: u16, config: Config) {
    let shared_state = Arc::new(tokio::sync::RwLock::new(AppState { db: None, config }));
    let app = Router::new()
        .route("/fetch_urls", post(fetch_urls))
        .route("/check", get(check_url))
        .layer(Extension(shared_state.clone()));

    // Запуск сервера
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[derive(Deserialize)]
pub struct CheckUrlParams {
    url: String,
}
pub async fn check_url(state: Extension<SharedState>, params: Query<CheckUrlParams>) -> Response {
    {
        if state.read().await.db.is_none() {
            let AppState { db, config } = &mut *state.write().await;
            *db = Some(match sled::open(&config.db_name) {
                Ok(db) => db,
                Err(err) => {
                    return (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response();
                }
            });
        }
    }
    let AppState { db, .. } = &*state.read().await;
    let CheckUrlParams { url } = params.0;
    let url_hash = url_to_hash(&url);
    match get_from_db(db.as_ref().unwrap(), &url_hash) {
        Ok(None) => Json(json!(None::<String>)).into_response(),
        Ok(Some(value)) => Json(json!(value)).into_response(),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct FetchUrlsParams {
    max_requests_at_once: Option<usize>,
    fetch_timeout_secs: Option<u64>,
    clear_db: Option<bool>,
    urls: HashSet<String>,
}

pub async fn fetch_urls(
    state: Extension<SharedState>,
    Json(FetchUrlsParams {
        max_requests_at_once,
        fetch_timeout_secs,
        clear_db,
        urls,
    }): Json<FetchUrlsParams>,
) -> Response {
    if urls.is_empty() {
        (StatusCode::BAD_REQUEST, "пустой urls").into_response()
    } else {
        let AppState { db, config } = &mut *state.write().await;
        match process_urls(
            urls,
            max_requests_at_once.unwrap_or(config.max_requests_at_once),
            fetch_timeout_secs.unwrap_or(config.fetch_timeout_secs),
            clear_db.unwrap_or(false),
            config.db_name.as_str(),
            db,
        )
        .await
        {
            Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()).into_response(),
            Ok(results) => Json(json!(results)).into_response(),
        }
    }
}

pub struct AppState {
    db: Option<Db>,
    config: Config,
}
pub type SharedState = Arc<tokio::sync::RwLock<AppState>>;
