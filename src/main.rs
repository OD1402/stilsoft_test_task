use anyhow::{Context, Result};
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;
use sled::Db;
use std::collections::HashMap;
use std::collections::HashSet;
// use std::env;
use std::fs::File;
use std::process;
use std::sync::Arc;
// use tokio::sync::Mutex;
use clap::Parser;

use tokio::fs;
use tokio::sync::Semaphore;
use tokio::task;
use tokio::time::Duration;

#[derive(Parser, Debug)]
struct Args {
    #[clap(long, default_value = "5")]
    max_count_request: usize,
    #[clap(long, default_value = "5")]
    max_count_seconds: u64,
    #[clap(long, default_value = "false")]
    clear_db_data: bool,
    #[clap(long, use_value_delimiter = true)]
    source_urls: Vec<String>,
}

#[derive(Deserialize)]
struct Source {
    source_urls: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let Args {
        max_count_request,
        max_count_seconds,
        clear_db_data,
        source_urls,
    } = Args::parse();

    println!("max_count_request {}", max_count_request);
    println!("max_count_seconds {}", max_count_seconds);
    println!("clear_db_data {}", clear_db_data);
    println!("source_urls {:#?}", source_urls);

    // let args: Vec<String> = env::args().collect();
    let mut urls: Vec<String> = vec![];
    let file_path = "source.json";

    if source_urls.len() > 0 {
        urls = source_urls[0..].to_vec();
    } else {
        println!(
            "Не получили аргументы командной строки, попробуем прочитать список урлов из файла {}\n", file_path
        );

        match fs::read_to_string(file_path).await {
            Err(err) => {
                eprintln!(
                    "Не удалось получить данные из файла: {} {}\n",
                    file_path, err
                );
            }
            Ok(content) => match serde_json::from_str::<Source>(&content) {
                Err(err) => {
                    eprintln!(
                        "Невалидный JSON, возможно файл {} пустой \n{}\n",
                        file_path, err
                    );
                }
                Ok(json) => {
                    urls = json.source_urls;
                }
            },
        }
    }

    if urls.is_empty() {
        eprintln!("Список URL для обработки пустой, завершаем работу");
        process::exit(1);
    }

    // уберем дубли из массива ссылок (с сохранением сортировки)
    let unique_urls: HashSet<String> = urls.clone().into_iter().collect();
    let urls: Vec<String> = unique_urls.into_iter().collect();

    println!("Список URL для обработки {:#?}:", urls);

    // зачищаем БД с ранее сохраненными результатами, если передачи параметр clear_db_data
    if clear_db_data {
        use tokio::fs::remove_dir_all;
        let db: Db = sled::open("db_result")?;
        drop(db);
        remove_dir_all("db_result").await?;
    }

    let db: Db = sled::open("db_result")?;

    let client = Arc::new(Client::new());
    let semaphore = Arc::new(Semaphore::new(max_count_request)); // одновременных запросов

    let mut responses = vec![];
    let results: Arc<std::sync::Mutex<HashMap<String, usize>>> =
        Arc::new(std::sync::Mutex::new(HashMap::new()));

    for url in urls {
        let client = Arc::clone(&client);
        let semaphore_clone = Arc::clone(&semaphore);
        let results_clone = Arc::clone(&results);
        let db_clone = db.clone();

        // асинхронная задача
        let response = task::spawn(async move {
            let _permit = semaphore_clone.acquire().await.unwrap(); // acquire запрашивает разрешение на доступ к ресурсу управляемому семафором

            match get_from_db(&db_clone, &url).await {
                Some(value) => {
                    println!("Получили значение из БД {}: {}", &url, value);
                    // есть запись в БД, не будем скачивать данные
                    results_clone.lock().unwrap().insert(url.clone(), value);
                }
                None => {
                    // нет записи в БД, скачаем данные
                    match download_and_count_first_line(&client, &url, max_count_seconds).await {
                        Ok(value) => {
                            results_clone.lock().unwrap().insert(url.clone(), value);

                            match save_to_db(&db_clone, &url, value).await {
                                Ok(_) => println!("Записали значение в БД {} {}", &url, value),
                                Err(e) => eprintln!("Ошибка при сохранении в БД: {}", e),
                            }
                        }
                        Err(err) => {
                            eprintln!("Failed to fetch {}", err);
                        }
                    }
                }
            }
        });

        responses.push(response);
    }

    // ждем завершения всех задач
    for response in responses {
        let _ = response.await;
    }

    // сохраняем результат в файл
    let results = results.lock().unwrap();
    let json_results = json!(*results);
    let mut file = File::create("results.json")?;
    serde_json::to_writer_pretty(&mut file, &json_results)?;

    // Выведем на экран результат обработки
    println!("\n====================");
    println!("Обработка завершена. \nresults: {:#?}", results);
    println!("====================");

    Ok(())
}

async fn get_from_db(db: &Db, url: &str) -> Option<usize> {
    // block_in_place синхронный вызов в асинхронном контексте
    tokio::task::block_in_place(|| match db.get(url) {
        Ok(Some(value)) => {
            let string_value = String::from_utf8(value.to_vec()).ok()?;
            string_value.parse::<usize>().ok()
        }
        Ok(None) => None,
        Err(_) => {
            eprintln!("Ошибка при получении данных из БД");
            None
        }
    })
}

async fn save_to_db(db: &Db, url: &str, value: usize) -> Result<()> {
    task::block_in_place(move || {
        db.insert(url, value.to_string().as_bytes())?;
        Ok(())
    })
}

async fn download_and_count_first_line(
    client: &Arc<Client>,
    url: &str,
    max_count_seconds: u64,
) -> Result<usize> {
    let response = client
        .get(url)
        .timeout(Duration::from_secs(max_count_seconds)) // Пытаемся скачать данные максимум max_count_seconds секунд
        .send()
        .await
        .map_err(|e| {
            anyhow::anyhow!("Не удалось скачать файл за 5 секунд для URL {}: {}", url, e)
        })?;
    // .with_context(|| format!("Failed to send request to {}", url))?; // Обработка ошибок запроса

    match response.status() {
        reqwest::StatusCode::OK => {
            let text = response
                .text()
                .await
                .with_context(|| format!("Failed to read response text from {}", url))?;

            // Сколько символов в первой строке
            let first_line_length = text.lines().next().map_or(0, |line| line.len());
            Ok(first_line_length)
        }
        // reqwest::StatusCode::NOT_FOUND => {
        //     Err(anyhow::anyhow!("404 Not Found for URL: {}", url))
        // }
        _ => {
            // если код не 200, пропускаем ссылку
            Err(anyhow::anyhow!(
                "Status code: '{}' for URL: {}",
                response.status(),
                url
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::mock;
    use reqwest::StatusCode;

    #[tokio::test]
    async fn test_download_success() {
        let _m = mock("GET", "/test_url")
            .with_status(StatusCode::OK.as_u16().into())
            .with_body("test!!\nThis is a test.")
            .create();

        let client = Arc::new(Client::new());
        let result = download_and_count_first_line(
            &client,
            &format!("{}{}", mockito::server_url(), "/test_url"),
        )
        .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 6);
    }

    #[tokio::test]
    async fn test_download_404() {
        let _m = mock("GET", "/test_url")
            .with_status(StatusCode::NOT_FOUND.as_u16().into())
            .with_body("test!!\nThis is a test.")
            .create();

        let client = Arc::new(Client::new());
        let result = download_and_count_first_line(
            &client,
            &format!("{}{}", mockito::server_url(), "/test_url"),
        )
        .await;

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Status code: '404 Not Found' for URL: http://127.0.0.1:1234/test_url".to_string()
        );
    }
}
