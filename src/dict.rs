use anyhow::{anyhow, bail, Result};
use directories::BaseDirs;
use glob::glob;
use rusqlite::{params, Connection, Transaction};
use serde_derive::Deserialize;
use std::{fs, path::Path};

use crate::deinflect;

#[derive(Debug)]
pub struct DictConn {
    pub conn: Connection,
    pub new: bool,
}

#[derive(Debug, Deserialize)]
struct YomichanDict {
    title: String,
}

#[derive(Debug)]
struct DbDictionary {
    id: i64,
    title: String,
    priority: i8,
}

#[derive(Debug)]
pub struct DbDictEntry {
    pub id: i64,
    pub kanji: String,
    pub reading: String,
    pub meaning: String,
    pub dict_id: i64,
}

#[derive(Debug, Deserialize)]
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

pub struct DictDb {
    conn: DictConn,
}

impl DictDb {
    pub fn new() -> Result<Self> {
        let conn = DictConn::new()?;
        if conn.new {
            let res = conn.setup_schema();
            if res.is_err() {
                bail!("Unable to setup database schema")
            }
        }
        Ok(DictDb { conn })
    }

    pub fn load_yomichan_dict(&mut self, path: &Path) -> Result<()> {
        if Self::validate_yomichan(path) {
            // setup transaction for faster writes
            let tx = self.conn.get_transaction()?;

            let dictmeta_text = std::fs::read_to_string(path.join("index.json"))?;
            let dictmeta: YomichanDict = serde_json::from_str(&dictmeta_text)?;
            let dict_id = Self::insert_dict(&dictmeta.title, &tx)?;

            let term_banks = glob(path.join("term_bank_*.json").to_str().unwrap())?;
            for term_bank in term_banks {
                let text = std::fs::read_to_string(term_bank.unwrap())?;
                let data: Vec<YomichanEntryV1> = serde_json::from_str(&text)?;
                for entry in data {
                    Self::insert_entry(entry, dict_id, &tx)?;
                }
            }
            if tx.commit().is_err() {
                bail!("Unable to commit transaction");
            }
        }
        Ok(())
    }

    pub fn validate_yomichan(path: &Path) -> bool {
        let is_dir = path.is_dir();
        let has_index = path.join("index.json").exists();
        let has_termbanks = path.join("term_bank_1.json").exists();
        is_dir && has_index && has_termbanks
    }

    // Inserts dictionary metadata and returns primary key
    fn insert_dict(title: &str, tx: &Transaction) -> rusqlite::Result<i64> {
        tx.query_row::<i64, _, _>(
            "INSERT INTO dicts (title) VALUES (?1) RETURNING id",
            params![title],
            |r| r.get(0),
        )
    }

    fn insert_entry(
        entry: YomichanEntryV1,
        dict_id: i64,
        tx: &Transaction,
    ) -> rusqlite::Result<()> {
        for meaning in entry.meanings {
            tx.execute(
                "INSERT INTO entries (kanji, reading, meaning, dict_id) VALUES (?1, ?2, ?3, ?4)",
                params![entry.kanji, entry.reading, meaning, dict_id],
            )?;
        }

        Ok(())
    }

    pub fn lookup_word(&self, word: &str) -> rusqlite::Result<Vec<DbDictEntry>> {
        let mut stmt = self
            .conn
            .conn
            .prepare("SELECT * FROM entries WHERE kanji = :word")?;
        let rows = stmt.query_map(&[(":word", word)], |row| {
            Ok(DbDictEntry {
                id: row.get(0)?,
                kanji: row.get(1)?,
                reading: row.get(2)?,
                meaning: row.get(3)?,
                dict_id: row.get(4)?,
            })
        })?;

        let mut entries: Vec<DbDictEntry> = vec![];
        for row in rows {
            entries.push(row?);
        }

        Ok(entries)
    }
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

    pub fn get_transaction(&mut self) -> rusqlite::Result<Transaction> {
        self.conn.transaction()
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

pub fn lookup(dict_db: DictDb, word: String) -> Result<Vec<DbDictEntry>> {
    let mut results: Vec<DbDictEntry> = vec![];
    let deinflect_json = include_str!("../data/deinflect.json");
    let deinflector = deinflect::Deinflector::new(deinflect_json);
    let deinflected_forms = deinflector.deinflect(word);

    for form in deinflected_forms {
        let lookup_res = dict_db.lookup_word(&form.term)?;
        results.extend(lookup_res);
    }

    Ok(results)
}
