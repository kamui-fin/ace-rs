use anyhow::Result;
use serde_derive::{Deserialize, Serialize};
use serde_json::{json, Value};

pub struct AnkiConnect;

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
pub struct Media {
    pub url: String,
    pub filename: String,
}

#[derive(Debug)]
pub struct NoteData {
    pub word: String,
    pub sentence: String,
    pub meaning: String,
    pub image: Media,
    pub audio: Media,
}

#[derive(Deserialize, Debug)]
pub struct AddResult {
    result: usize,
    error: String,
}

impl AnkiConnect {
    pub fn get_note_json(
        &self,
        deck_model_info: &DeckModelInfo,
        note_data: &NoteData,
    ) -> serde_json::Value {
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
            "audio": [{
                "url": note_data.audio.url,
                "filename": note_data.audio.filename,
                "fields": [
                    deck_model_info.audio_field
                ]
            }],
            "picture": [{
                "url": note_data.image.url ,
                "filename": note_data.image.filename,
                "fields": [
                    deck_model_info.img_field
                ]
            }]
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
        let res = client
            .post("http://localhost:8765")
            .json(&post_data)
            .send()
            .await?
            .json::<Value>()
            .await?;

        Ok(res)
    }
}
