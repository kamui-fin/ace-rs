use crate::anki::{AnkiConnect, Media};
use crate::{
    anki::NoteData,
    config::Config,
    dict::{lookup, DictDb},
    media::{fetch_audio_server, forvo, get_sent, google_img},
    CONFIG,
};
use anyhow::{anyhow, Context, Result};
use fs::OpenOptions;
use indicatif::{ProgressBar, ProgressStyle};
use lazy_static::lazy_static;
use pinyin::{to_pinyin_vec, Pinyin};
use pinyin_parser::PinyinParser;
use regex::Regex;
use std::{collections::HashMap, convert::TryInto, io::Write};
use std::{fs, path::Path};

pub fn get_config() -> Result<&'static Config> {
    CONFIG.get().context("Failed to read config")
}

// Returns [(word, sentence)]
pub fn read_words_file(path: &Path) -> Result<Vec<(String, String)>> {
    let text = fs::read_to_string(path).with_context(|| "Failed to read words file")?;
    let mut word_sentence_pairs: Vec<(String, String)> = vec![];
    for line in text.lines() {
        if let Some((word, sentence)) = line.split_once(" ") {
            word_sentence_pairs.push((word.to_string(), sentence.to_string()))
        } else {
            word_sentence_pairs.push((line.to_string(), "".to_string()))
        }
    }
    Ok(word_sentence_pairs)
}

fn pinyin_from_definition(meaning: &String) -> Option<Vec<String>> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"\w\s\[(.*)\]").unwrap();
    }
    if let Some(captures) = RE.captures(meaning) {
        let pinyin = captures.get(0).unwrap().as_str().to_string();
        let pinyin = PinyinParser::strict(&pinyin)
            .into_iter()
            .collect::<Vec<_>>();

        let syllables = [
            (1, ['ā', 'ē', 'ī', 'ō', 'ū', 'ǖ']),
            (2, ['á', 'é', 'í', 'ó', 'ú', 'ǘ']),
            (3, ['ǎ', 'ě', 'ǐ', 'ǒ', 'ǔ', 'ǚ']),
            (4, ['à', 'è', 'ì', 'ò', 'ù', 'ǜ']),
            (5, ['a', 'e', 'i', 'o', 'u', 'ü']),
        ];

        let mut map = HashMap::new();
        let mut without_tone = HashMap::new();
        for (tone, vowels) in syllables {
            let vowels = vowels.to_vec();
            for (i, vowel) in vowels.iter().enumerate() {
                if tone != 5 {
                    map.insert(*vowel, tone);
                }
                without_tone.insert(*vowel, syllables[4].1[i]);
            }
        }

        let mut tone_numbers = vec![];
        for pinyin_syllable in pinyin {
            let mut num = 5;
            let mut new_tone = String::new();
            for c in pinyin_syllable.chars() {
                if map.contains_key(&c) {
                    num = map[&c];
                    break;
                }
            }
            for c in pinyin_syllable.chars() {
                if map.contains_key(&c) {
                    new_tone += &without_tone[&c].to_string();
                } else {
                    new_tone += c.to_string().as_str();
                }
            }
            new_tone += &num.to_string();
            tone_numbers.push(new_tone);
        }

        return Some(tone_numbers);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::pinyin_from_definition;
    #[test]
    fn it_works() {
        assert_eq!(
            pinyin_from_definition(&"俄罗斯 [éluósi]".to_string()),
            Some(vec![
                "e2".to_string(),
                "luo2".to_string(),
                "si5".to_string()
            ])
        );
    }
}

pub async fn package_card(
    dict_db: &DictDb,
    word: &str,
    sentence: String,
) -> Result<Option<NoteData>> {
    let config = get_config()?;
    let sentence = if sentence.is_empty() {
        get_sent(word, config.is_japanese)
            .await
            .with_context(|| "Failed to fetch sentence")?
    } else {
        sentence
    };

    let defs = &lookup(dict_db, word.to_string())
        .with_context(|| "Failed to lookup word in dictionary")?;

    if defs.is_empty() {
        return Ok(None);
    }

    let meaning = defs
        .iter()
        .map(|def| def.meaning.replace("\n", "<br>"))
        .collect::<Vec<String>>()
        .join("<br><br>");

    let image_res = if config.media.add_picture {
        google_img(word.to_string(), config.is_japanese)
            .await
            .with_context(|| "Failed to fetch image")
    } else {
        Err(anyhow!("Image not required"))
    };

    let mut audio_res;

    if !config.media.custom_audio_server.is_empty() {
        audio_res = fetch_audio_server(word, &config.media.custom_audio_server)
            .await
            .with_context(|| "Failed to fetch audio");

        if audio_res.is_err() && config.media.fallback_forvo {
            audio_res = forvo(word).await.with_context(|| "Failed to fetch audio");
        }
    } else {
        audio_res = forvo(word).await.with_context(|| "Failed to fetch audio");
    }

    let image: Option<Media>;
    let audio: Option<Media>;

    if (image_res.is_err() || audio_res.is_err()) && config.media.bail_on_empty {
        image = Some(image_res?);
        audio = Some(audio_res?);
    } else {
        image = image_res.ok();
        audio = audio_res.ok();
    }

    let word_pinyin = if !config.is_japanese {
        let tone_num = ['1', '2', '3', '4'];
        let pinyin_vec = if let Some(pinyin_vec) = pinyin_from_definition(&meaning) {
            pinyin_vec
        } else {
            to_pinyin_vec(word, Pinyin::with_tone_num_end)
                .iter()
                .map(|&tone| {
                    if !tone_num.contains(&tone.chars().next_back().unwrap_or_default()) {
                        format!("{}5", tone)
                    } else {
                        tone.to_string()
                    }
                })
                .collect::<Vec<_>>()
        };
        format!("{}[{}]", word, pinyin_vec.join(" "),)
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

pub async fn export_words(dict_db: &DictDb, words_file: &Path) -> Result<()> {
    let config = get_config()?;
    let failed_file = Path::new(&config.failed_words_file);
    let anki_connect = AnkiConnect {
        port: config.ankiconnect.port,
        address: config.ankiconnect.address.clone(),
    };
    anki_connect.status().await?;

    println!("Starting to generate card data...");
    let word_sentences = read_words_file(words_file)?;
    let mut notes = vec![];

    let bar = ProgressBar::new(word_sentences.len().try_into().unwrap());
    bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos:>7}/{len:7}")
            .progress_chars("#>-"),
    );
    bar.inc(0);
    for pair in word_sentences {
        let (word, sentence) = pair;
        let ndata = package_card(dict_db, &word, sentence).await;
        if let Ok(Some(ndata)) = ndata {
            notes.push(ndata);
            bar.inc(1);
        } else if failed_file.is_file() {
            println!("{} {:#?}", word, ndata);
            let mut file = OpenOptions::new().append(true).open(failed_file)?;
            writeln!(file, "{}", word)?;
        }
    }

    bar.finish();
    anki_connect.bulk_add_cards(notes).await?;

    Ok(())
}
