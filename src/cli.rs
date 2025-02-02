use super::*;

pub async fn run_cli(
    args: Vec<String>,
    clear_db: bool,
    max_requests_at_once: Option<usize>,
    fetch_timeout_secs: Option<u64>,
    config: Config,
) -> Result<()> {
    let mut urls: HashSet<String> = HashSet::new();
    for arg in args {
        if arg.starts_with("http") {
            if urls.contains(&arg) {
                warn!("url {arg:?} указан повторно");
            } else {
                urls.insert(arg);
            }
        } else {
            match std::fs::File::open(&arg) {
                Err(err) => {
                    warn!("File::open({arg:?}): {err}");
                }
                Ok(file) => {
                    for s in std::io::BufReader::new(file).lines().map_while(Result::ok) {
                        if s.starts_with("http") {
                            if urls.contains(&s) {
                                warn!("url {s:?} указан повторно в файле {arg:?}");
                            } else {
                                info!("добавлен url {s:?} из файла {arg:?}");
                                urls.insert(s);
                            }
                        } else {
                            warn!("проигнорирована строка {s:?} из файла {arg:?}");
                        }
                    }
                }
            }
        }
    }
    if urls.is_empty() {
        eprintln!("Список URL для обработки пустой, завершаем работу");
    } else {
        let mut db = None;
        let results = process_urls(
            urls,
            max_requests_at_once.unwrap_or(config.max_requests_at_once),
            fetch_timeout_secs.unwrap_or(config.fetch_timeout_secs),
            clear_db,
            config.db_name.as_str(),
            &mut db,
        )
        .await?;

        println!("\n====================");
        println!("Обработка завершена. \nresults: {:#?}", results);
        println!("====================");

        let mut file = File::create("results.json")?;
        serde_json::to_writer_pretty(&mut file, &json!(results))?;
    };
    Ok(())
}
