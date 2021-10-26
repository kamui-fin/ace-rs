use anyhow::{anyhow, Result};
use directories::BaseDirs;
use rusqlite::Connection;
use std::fs;

#[derive(Debug)]
struct Dictionary {
    title: String,
    priority: i8,
}

#[derive(Debug)]
struct YomichanEntryV1 {
    kanji: String,
    reading: String,
    definition_tags: String,
    rule_identifiers: String,
    popularity: i32,
    meanings: Vec<String>,
    sequence: usize,
    term_tags: String,
}

#[derive(Debug)]
pub struct DictConn {
    conn: Connection,
    pub new: bool,
}

impl DictConn {
    pub fn new() -> Result<Self> {
        let basedirs = BaseDirs::new();
        if let Some(basedirs) = basedirs {
            let path = basedirs.data_dir().join("ace");
            fs::create_dir_all(&path)?;
            let db_path = path.join("dict.db");
            let new = !db_path.exists();
            let conn = Connection::open(db_path)?;

            Ok(DictConn { conn, new })
        } else {
            Err(anyhow!("Could not find data directory"))
        }
    }

    pub fn setup_schema(&self) -> rusqlite::Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS dicts (
                  id              INTEGER PRIMARY KEY AUTOINCREMENT,
                  title           TEXT NOT NULL,
                  priority        INTEGER DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS entries (
                  id              INTEGER PRIMARY KEY AUTOINCREMENT,
                  kanji           TEXT NOT NULL,
                  reading         TEXT,
                  meaning         TEXT NOT NULL,
                  dict_id         INTEGER NOT NULL,
                  FOREIGN KEY(dict_id) REFERENCES dicts(id)
            );

            CREATE INDEX IF NOT EXISTS kanji_idx ON entries(kanji); 
            ",
        )
    }
}
