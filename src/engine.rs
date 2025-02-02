use super::*;

pub async fn process_urls(
    urls: HashSet<String>,
    max_requests_at_once: usize,
    fetch_timeout_secs: u64,
    clear_db: bool,
    db_name: &str,
    db: &mut Option<Db>,
) -> Result<HashMap<String, Result<(usize, DateTime<Utc>), String>>> {
    let mut urls = urls.into_iter().collect::<Vec<_>>();
    println!("Список URL для обработки {:#?}:\n", urls);
    if clear_db {
        if db.is_some() {
            *db = None;
        }
        if tokio::fs::remove_dir_all(&db_name).await.is_ok() {
            info!("did remove_dir_all'{db_name}'");
        }
    }
    // зачищаем БД с ранее сохраненными результатами, если передачи параметр clear_db
    if db.is_none() {
        *db = Some(sled::open(db_name)?);
    }
    let client = Arc::new(Client::new());

    let mut fut_queue = futures::stream::FuturesUnordered::new();
    while fut_queue.len() < max_requests_at_once {
        if let Some(url) = urls.pop() {
            fut_queue.push(process_url(
                url,
                db.as_ref().unwrap(),
                client.clone(),
                fetch_timeout_secs,
            ))
        } else {
            break;
        }
    }

    let mut results: HashMap<String, Result<(usize, DateTime<Utc>), String>> = HashMap::new();
    let start = std::time::Instant::now();
    loop {
        futures::select! {
            ret = fut_queue.select_next_some() => {
                let ProcessUrlRet { url, res, elapsed_millis} = ret;
                if let Err(err) = &res {
                    eprintln!("Failed to fetch {err}");
                }
                info!("{elapsed_millis} msecs заняла обработка url {url}");
                results.insert(url, res.map(|DbRecord { line_count, at, url: _}| (line_count, at)));
                if let Some(url) = urls.pop() {
                    fut_queue.push(process_url(url, db.as_ref().unwrap(), client.clone(), fetch_timeout_secs))
                }
            }
            complete => {
                break;
            }
        }
    }
    info!(
        "{} msecs заняла обработка всех url'ов ",
        std::time::Instant::now().duration_since(start).as_millis()
    );
    Ok(results)
}

pub fn url_to_hash(url: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(url);
    let finalize = hasher.finalize();
    hex::encode(finalize)
}

struct ProcessUrlRet {
    url: String,
    res: Result<DbRecord, String>,
    elapsed_millis: u128,
}
async fn process_url(
    url: String,
    db: &Db,
    client: Arc<Client>,
    fetch_timeout_secs: u64,
) -> ProcessUrlRet {
    let start = std::time::Instant::now();

    let url_hash = url_to_hash(&url);
    let res = match get_from_db(db, &url_hash) {
        Ok(Some(value)) => {
            info!(
                "Получили значение из БД: {} \n{} {}\n",
                &url,
                &url_hash,
                serde_json::to_string_pretty(&value).unwrap()
            );
            // есть запись в БД, не будем скачивать данные
            Ok(value)
        }
        Err(_) | Ok(None) => {
            // нет записи в БД, скачаем данные
            match download_and_line_count(&client, &url, fetch_timeout_secs).await {
                Err(err) => Err(err.to_string()),
                Ok(value) => {
                    match save_to_db(db, &url_hash, &value) {
                        Ok(_) => info!(
                            "Записали значение в БД: {} => {}\n",
                            &url_hash,
                            serde_json::to_string_pretty(&value).unwrap()
                        ),
                        Err(e) => error!("Ошибка при сохранении в БД: {}", e),
                    }
                    Ok(value)
                }
            }
        }
    };
    ProcessUrlRet {
        url,
        res,
        elapsed_millis: std::time::Instant::now().duration_since(start).as_millis(),
    }
}

async fn download_and_line_count(
    client: &Arc<Client>,
    url: &str,
    fetch_timeout_secs: u64,
) -> Result<DbRecord> {
    let at = chrono::Utc::now();
    let response = client
        .get(url)
        .timeout(Duration::from_secs(fetch_timeout_secs)) // Пытаемся скачать данные максимум fetch_timeout_secs секунд
        .send()
        .await
        .map_err(|err| {
            anyhow!("Не удалось скачать файл за {fetch_timeout_secs} секунд для URL {url}: {err}",)
        })?;

    match response.status() {
        reqwest::StatusCode::OK => {
            let text = response
                .text()
                .await
                .with_context(|| format!("Failed to read response text from {url}"))?;

            // Сколько строк на странице
            let line_count = text.lines().count();
            Ok(DbRecord {
                line_count,
                at,
                url: url.to_string(),
            })
        }
        _ => {
            // если код не 200, пропускаем ссылку
            Err(anyhow::anyhow!(
                "Status code: '{}' for URL: {url}\n",
                response.status(),
            ))
        }
    }
}
