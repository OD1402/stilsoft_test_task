use axum::extract::Path;
use axum::{routing::get, Router};
use std::net::SocketAddr;

use crate::db1::Db;

pub async fn server_start() {

    // маршрутизатор
    // пример запроса: http://localhost:3000/get_value/100680ad546ce6a577f42f52df33b4cfdca756859e664b8d7de329b150d09ce9
    let app = Router::new()
        .route("/get_value/:hash", get(get_value));

    // Запуск сервера
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

pub async fn get_value(Path(hash): Path<String>) -> String {
    let db = Db::connect().expect("Не удалось подключиться к базе данных");
    match Db::get(&db, &hash) {
        Some(value) => format!("Значение: {}", value),
        None => "Не найдено".to_string(),
    }
}
