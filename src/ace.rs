use crate::anki::{AnkiConnect, DeckModelInfo};
use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::convert::TryInto;
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

pub async fn package_card(dict_db: &DictDb, word: &str) -> Result<Option<NoteData>> {
    let sentence = get_sent(word).await?;
    let meaning = &lookup(dict_db, word.to_string())?;
    if meaning.is_empty() {
        return Ok(None);
    }

    let meaning = meaning[0].meaning.replace("\n", "<br>");
    let mut image = google_img(word.to_string(), 1).await?;
    let mut audio = forvo(word, 1).await?;

    let ndata = NoteData {
        word: word.to_string(),
        sentence,
        meaning: meaning.to_string(),
        image: image.remove(0),
        audio: audio.remove(0),
    };

    Ok(Some(ndata))
}

pub async fn export_words(
    dict_db: &DictDb,
    deck_model_info: DeckModelInfo,
    words_file: &Path,
) -> Result<()> {
    let words = read_words_file(words_file)?;
    let mut notes = vec![];

    let bar = ProgressBar::new(words.len().try_into().unwrap());
    bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos:>7}/{len:7}")
            .progress_chars("#>-"),
    );
    bar.inc(0);
    for word in words {
        let ndata = package_card(&dict_db, &word).await?;
        if let Some(ndata) = ndata {
            notes.push(ndata);
            bar.inc(1);
        }
    }
    bar.finish();

    let anki_connect = AnkiConnect {};
    anki_connect.bulk_add_cards(deck_model_info, notes).await?;
    Ok(())
}
