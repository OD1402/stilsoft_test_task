use super::*;

#[derive(Debug, Serialize, Deserialize)]
pub struct DbRecord {
    pub url: String,
    pub line_count: usize,
    pub at: DateTime<Utc>,
}

pub fn save_to_db(db: &Db, url_hash: &str, db_record: &DbRecord) -> Result<()> {
    db.insert(url_hash, serde_json::to_string(&db_record)?.as_bytes())?;
    Ok(())
}

pub fn get_from_db(db: &Db, url_hash: &str) -> Result<Option<DbRecord>> {
    match db.get(url_hash) {
        Ok(None) => Ok(None),
        Ok(Some(value)) => String::from_utf8(value.to_vec())
            .map_err(|err| anyhow!(err))
            .and_then(|s| {
                serde_json::from_str::<DbRecord>(&s)
                    .map(Some)
                    .map_err(|err| anyhow!(err))
            }),
        Err(err) => Err(anyhow!(err)),
    }
}
