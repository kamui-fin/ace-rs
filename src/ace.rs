use crate::{
    anki::NoteData,
    dict::{lookup, DictDb},
    media::{fetch_audio_server, forvo, get_sent, google_img},
};
use crate::{
    anki::{AnkiConnect, DeckModelInfo, Media},
    config::AnkiConnectConfig,
};
use anyhow::{Context, Result};
use fs::OpenOptions;
use indicatif::{ProgressBar, ProgressStyle};
use pinyin::{to_pinyin_vec, Pinyin};
use std::{convert::TryInto, io::Write};
use std::{fs, path::Path};

pub fn read_words_file(path: &Path) -> Result<Vec<String>> {
    let text = fs::read_to_string(path).with_context(|| "Failed to read words file")?;
    let words = text.lines().map(|l| l.to_string()).collect::<Vec<String>>();
    Ok(words)
}

pub async fn package_card(
    dict_db: &DictDb,
    word: &str,
    forvo_fallback: bool,
    bail_on_empty_media: bool,
    custom_audio_server: &str,
    sort_freq: bool,
    is_japanese: bool,
    add_picture: bool,
) -> Result<Option<NoteData>> {
    let sentence = get_sent(word, is_japanese)
        .await
        .with_context(|| "Failed to fetch sentence")?;

    let defs = &lookup(dict_db, word.to_string(), sort_freq, is_japanese)
        .with_context(|| "Failed to lookup word in dictionary")?;

    if defs.is_empty() {
        return Ok(None);
    }

    let meaning = defs
        .iter()
        .map(|def| def.meaning.replace("\n", "<br>"))
        .collect::<Vec<String>>()
        .join("<br><br>");

    let image_res = if add_picture {
        google_img(word.to_string(), is_japanese)
            .await
            .with_context(|| "Failed to fetch image")
    } else {
        Ok(Media::default())
    };

    let mut audio_res;

    if !custom_audio_server.is_empty() {
        audio_res = fetch_audio_server(word, custom_audio_server)
            .await
            .with_context(|| "Failed to fetch audio");

        if audio_res.is_err() && forvo_fallback {
            audio_res = forvo(word).await.with_context(|| "Failed to fetch audio");
        }
    } else {
        audio_res = forvo(word).await.with_context(|| "Failed to fetch audio");
    }

    let image: Media;
    let audio: Media;

    if (image_res.is_err() || audio_res.is_err()) && bail_on_empty_media {
        image = image_res?;
        audio = audio_res?;
    } else {
        image = image_res.unwrap_or_default();
        audio = audio_res.unwrap_or_default();
    }

    let word_pinyin = if !is_japanese {
        format!(
            "{}[{}]",
            word,
            to_pinyin_vec(word, Pinyin::with_tone_num_end).join(" ")
        )
    } else {
        String::from("")
    };

    let ndata = NoteData {
        word: word.to_string(),
        sentence,
        meaning,
        image,
        audio,
        word_pinyin,
    };

    Ok(Some(ndata))
}

pub async fn export_words(
    dict_db: &DictDb,
    deck_model_info: DeckModelInfo,
    words_file: &Path,
    failed_words_file: &Path,
    forvo_fallback: bool,
    bail_on_empty_media: bool,
    custom_audio_server: String,
    anki_connect_config: AnkiConnectConfig,
    sort_freq: bool,
    is_japanese: bool,
    add_picture: bool,
) -> Result<()> {
    let anki_connect = AnkiConnect {
        port: anki_connect_config.port,
        address: anki_connect_config.address,
    };
    anki_connect.status().await?;

    println!("Starting to generate card data...");
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
        let ndata = package_card(
            &dict_db,
            &word,
            forvo_fallback,
            bail_on_empty_media,
            &custom_audio_server,
            sort_freq,
            is_japanese,
            add_picture,
        )
        .await?;
        if let Some(ndata) = ndata {
            notes.push(ndata);
            bar.inc(1);
        } else if failed_words_file.is_file() {
            let mut file = OpenOptions::new().append(true).open(failed_words_file)?;
            writeln!(file, "{}", word)?;
        }
    }
    bar.finish();

    anki_connect.bulk_add_cards(deck_model_info, notes).await?;

    Ok(())
}
