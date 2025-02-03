### О программе

stilsoft_test_task выполняет следующие действия:
1. получает на вход для обработки список URL
2. берет в обработку одновременно несколько ссылок
3. проверяет для каждой ссылки - а нет ли уже сохраненного результата в БД sled db_result,<br/>
    * если есть,
      - берет результат из БД<br/>
    * если нет,
      - скачивает содержимое страниц в асинхронном режиме (если не получается скачать данные, ссылка пропускается и программа берет следующую ссылку)
      - считает количество строк на странице
      - складывает полученные результаты в БД sled db_result
4. выводит результат обработки на экран
5. записывает результат обработки в файл `results.json`

### Настройка окружения

* Установить rust - https://www.rust-lang.org/ru/tools/install
* Склонировать репозиторий `git clone git@github.com:OD1402/stilsoft_test_task.git`

### Варианты запуска

1. Режим Cli - через аргументы командной строки
     *  Аргументы, которые можно передать:
        * Список url
        * `--max-requests-at-once` - количество ссылок, которые нужно обрабатывать одновременно
        * `--fetch-timeout-secs` - количество секунд, на протяжении которых мы будем пробовать скачать данные по ссылке (если не получилось скачать за "fetch-timeout-secs" секунд, то пропускаем эту ссылку и берем следующую)
        * `--clear-db` - зачистить все результаты в БД перед обработкой ссылок
   * Примеры запуска:
     *  Обработка ссылок из файла  <br/>`cargo run -- cli urls.txt`
     *  Обработка ссылок из аргументов командной строки  <br/>`cargo run -- cli https://example.com https://www.rust-lang.org`
     *  Обработка ссылок из файла с попыткой скачать каждую ссылку максимум за 2 секунды  <br/>`cargo run -- cli urls.txt -- --max-requests-at-once 2` 

2. Режим Server - API
   * Запуск сервера  <br/>`cargo run -- server`

   * Запрос на получение данных из БД:
     <br/>`curl -w "\n" http://localhost:3000/check?url=https://example.com`
     <br/>Ответ: <br/>
     ```
      {
         at: "2025-02-02T22:49:42.454704882Z",
         line_count: 46,
         url: "https://example.com"
      }
      ```

   * Запрос на обработку данных: 
      <br/>`curl -w "\n" http://localhost:3000/fetch_urls \
      -H "Content-Type: application/json" \
      -d '{"urls": ["https://www.example.com", "https://www.rust-lang.org"]}'`
      <br/>Ответ: <br/>
      ```
         {"https://www.example.com":{"Ok":[46,"2025-02-02T22:55:26.464939760Z"]},"https://www.rust-lang.org":{"Ok":[413,"2025-02-02T22:49:42.454623622Z"]}}
      ```

### Вывод результатов обработки

Результаты обработки выводятся на экран терминала, пример вывода:
```
====================
Обработка завершена. 
results: {
    "https://www.youtube.com/?app=desktop&hl=ru": Ok(
        (
            23,
            2025-02-02T22:49:42.454532029Z,
        ),
    ),
    "https://zipal.ru/export/agency/8562/YANDEX": Err(
        "Status code: '404 Not Found' for URL: https://zipal.ru/export/agency/8562/YANDEX\n",
    ),
    "https://crates.io/crates/actix-files": Err(
        "Status code: '403 Forbidden' for URL: https://crates.io/crates/actix-files\n",
    ),
    "https://www.rust-lang.org": Ok(
        (
            413,
            2025-02-02T22:49:42.454623622Z,
        ),
    ),
}
====================
```
