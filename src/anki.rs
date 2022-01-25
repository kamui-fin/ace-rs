use anyhow::anyhow;
use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use serde_derive::{Deserialize, Serialize};
use serde_json::{json, Value};

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
}

#[derive(Debug)]
pub enum Source {
    Path(String),
    Url(String),
}

#[derive(Debug)]
pub struct Media {
    pub filename: String,
    pub source: Source,
}

#[derive(Debug)]
pub struct NoteData {
    pub word: String,
    pub sentence: String,
    pub meaning: String,
    pub image: Vec<Media>,
    pub audio: Vec<Media>,
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
        let audio_data = note_data
            .audio
            .iter()
            .map(|audio| {
                let audio_url;
                let audio_path;

                match &audio.source {
                    Source::Url(url) => {
                        audio_url = url;
                        audio_path = String::from("");
                    }
                    Source::Path(path) => {
                        audio_path = path.to_string();
                        audio_url = &empty_str;
                    }
                };
                json!({
                    "url": audio_url,
                    "path": audio_path,
                    "filename": audio.filename,
                    "fields": [
                        deck_model_info.audio_field
                    ]
                })
            })
            .collect::<Vec<Value>>();

        let picture_data = note_data
            .image
            .iter()
            .map(|pic| {
                let picture_url = if let Source::Url(url) = &pic.source {
                    url
                } else {
                    &empty_str
                };
                json!({
                    "url": picture_url,
                    "filename": pic.filename,
                    "fields": [
                        deck_model_info.img_field
                    ]
                })
            })
            .collect::<Vec<Value>>();

        json!({
            "deckName": deck_model_info.deck,
            "modelName": deck_model_info.model,
            "fields": {
                &deck_model_info.word_field: note_data.word,
                &deck_model_info.sent_field: note_data.sentence,
                &deck_model_info.dict_field: note_data.meaning,
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

    pub async fn bulk_add_cards(
        &self,
        deck_model_info: DeckModelInfo,
        notes: Vec<NoteData>,
    ) -> Result<Value> {
        let notes = notes
            .iter()
            .map(|note| self.get_note_json(&deck_model_info, note))
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
