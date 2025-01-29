use sled::{Db as SledDb, Result};

#[derive(Clone)]
pub struct Db {
    pub connection: SledDb,
}

impl Db {
    // подключение к БД
    pub fn connect() -> Result<Self> {
        let db = sled::open("db_result")?;
        Ok(Db { connection: db })
    }

    // получение значения по ключу
    pub fn get(&self, key: &str) -> Option<usize> {
        match self.connection.get(key) {
            Ok(Some(value)) => {
                let string_value = String::from_utf8(value.to_vec()).ok()?;
                string_value.parse::<usize>().ok()
            }
            Ok(None) => None,
            Err(_) => {
                eprintln!("Ошибка при получении данных из БД");
                None
            }
        }
    }

    // запись данных
}
