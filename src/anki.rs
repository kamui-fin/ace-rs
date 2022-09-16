use anyhow::anyhow;
use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use serde_derive::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::ace::get_config;

pub struct AnkiConnect {
    pub port: usize,
    pub address: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeckModelInfo {
    pub deck: String,
    pub model: String,
    pub word_field: String,
    pub sent_field: String,
    pub dict_field: String,
    pub img_field: String,
    pub audio_field: String,
    pub word_pinyin_field: String,
}

#[derive(Debug, Default)]
pub struct Media {
    pub filename: String,
    pub url: String,
}

#[derive(Debug)]
pub struct NoteData {
    pub word: String,
    pub sentence: String,
    pub meaning: String,
    pub image: Media,
    pub audio: Media,
    pub word_pinyin: String,
}

#[derive(Deserialize, Debug)]
pub struct AddResult {
    result: usize,
    error: String,
}

impl AnkiConnect {
    pub async fn status(&self) -> Result<()> {
        let client = reqwest::Client::new();
        let res = client
            .get(format!("http://{}:{}", self.address, self.port))
            .send()
            .await
            .with_context(|| format!("Failed to connect to AnkiConnect. Is Anki running?"))?
            .text()
            .await?;

        if res.contains("AnkiConnect") {
            return Ok(());
        }

        Err(anyhow!("Another service is already using port 8765"))
    }

    pub fn get_note_json(
        &self,
        deck_model_info: &DeckModelInfo,
        note_data: &NoteData,
    ) -> serde_json::Value {
        let empty_str = String::from("");
        let audio_data = json!({
            "url": note_data.audio.url,
            "filename": note_data.audio.filename,
            "fields": [
                deck_model_info.audio_field
            ]
        });

        let picture_data = {
            json!({
                "url": note_data.image.url,
                "filename": note_data.image.filename,
                "fields": [
                    deck_model_info.img_field
                ]
            })
        };

        json!({
            "deckName": deck_model_info.deck,
            "modelName": deck_model_info.model,
            "fields": {
                &deck_model_info.word_field: note_data.word,
                &deck_model_info.sent_field: note_data.sentence,
                &deck_model_info.dict_field: note_data.meaning,
                &deck_model_info.word_pinyin_field: note_data.word_pinyin,
            },
            "options": {
                "allowDuplicate": false,
                "duplicateScope": "deck",
                "duplicateScopeOptions": {
                    "checkChildren": false,
                    "checkAllModels": false
                }
            },
            "audio": audio_data,
            "picture": picture_data
        })
    }

    pub async fn bulk_add_cards(&self, notes: Vec<NoteData>) -> Result<Value> {
        let config = get_config()?;
        let notes = notes
            .iter()
            .map(|note| self.get_note_json(&config.anki, note))
            .collect::<Vec<serde_json::Value>>();
        let post_data = json!({
            "action": "addNotes",
            "version": 6,
            "params": {
                "notes": notes
            }
        });
        let client = reqwest::Client::new();
        let pb = ProgressBar::new_spinner();
        pb.enable_steady_tick(120);
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
                .template("{spinner:.blue} {msg}"),
        );
        pb.set_message("Exporting notes...");
        let res = client
            .post(format!("http://{}:{}", self.address, self.port))
            .json(&post_data)
            .send()
            .await
            .with_context(|| format!("Failed to connect to AnkiConnect. Is Anki running?"))?
            .json::<Value>()
            .await?;
        pb.finish_with_message("Done");
        Ok(res)
    }
}
