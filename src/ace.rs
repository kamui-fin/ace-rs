use crate::anki::{AnkiConnect, DeckModelInfo};
use anyhow::Result;
use std::{fs, path::Path};

use crate::{
    anki::NoteData,
    dict::{lookup, DictDb},
    media::{forvo, get_sent, google_img},
};

pub fn read_words_file(path: &Path) -> Result<Vec<String>> {
    let text = fs::read_to_string(path)?;
    let words = text.lines().map(|l| l.to_string()).collect::<Vec<String>>();
    Ok(words)
}

pub async fn package_card(dict_db: &DictDb, word: &str) -> Result<NoteData> {
    let sentence = get_sent(word).await?;
    let meaning = &lookup(dict_db, word.to_string())?[0]
        .meaning
        .replace("\n", "<br>");
    let mut image = google_img(word.to_string(), 1).await?;
    let mut audio = forvo(word, 1).await?;

    let ndata = NoteData {
        word: word.to_string(),
        sentence,
        meaning: meaning.to_string(),
        image: image.remove(0),
        audio: audio.remove(0),
    };

    Ok(ndata)
}

pub async fn export_words(
    dict_db: &DictDb,
    deck_model_info: DeckModelInfo,
    words_file: &Path,
) -> Result<()> {
    let words = read_words_file(words_file)?;
    let mut notes = vec![];

    for word in words {
        let ndata = package_card(&dict_db, &word).await?;
        notes.push(ndata);
    }

    let anki_connect = AnkiConnect {};
    anki_connect.bulk_add_cards(deck_model_info, notes).await?;

    Ok(())
}
