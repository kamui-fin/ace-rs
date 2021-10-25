use serde_derive::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub deck: String,
    pub note_type: String,
    pub word_field: String,
    pub sentence_field: String,
    pub image_field: String,
    pub audio_field: String,
    pub meaning_field: String,
    pub words_file: String,
    pub failed_words_file: String,
}
