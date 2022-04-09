use anyhow::{Context, Result};
use rand::seq::SliceRandom;
use rand::thread_rng;
use regex::Regex;
use scraper::{Html, Selector};
use serde_json::Value;
use std::fs;
use std::{io::Cursor, path::Path};
use uuid::Uuid;

use crate::anki::Media;
use crate::anki::Source;

fn with_uuid(prefix: String) -> String {
    let uuid = Uuid::new_v4().to_string();
    return format!("{}-{}", prefix, uuid);
}

pub async fn get_sent(word: &str) -> Result<String> {
    let base_url = String::from("https://massif.la/ja/search?q=");
    let resp = reqwest::get(base_url + word).await?;

    let document = Html::parse_document(&resp.text().await?);
    let selector = Selector::parse("li.text-japanese > div:not(.result-meta)").unwrap();
    let sent_div = document.select(&selector).next().unwrap();
    let sent_text = sent_div.text().collect::<Vec<_>>().join("");

    Ok(sent_text)
}

async fn download_file(url: &str, output_path: &Path, extension: Option<&str>) -> Result<()> {
    let response = reqwest::get(url).await?;
    let content_type = response.headers().get("Content-Type");

    let mut full_path = output_path.to_path_buf();

    if let Some(ext) = extension {
        full_path.set_extension(ext);
    } else {
        // probe extension from content-type header
        if let Some(ctype) = content_type {
            let ext = ctype.to_str().unwrap().split('/').collect::<Vec<&str>>()[1];
            full_path.set_extension(ext);
        }
    }

    let bytes = response.bytes().await?;
    let mut file = std::fs::File::create(full_path)?;
    let mut content = Cursor::new(bytes);
    std::io::copy(&mut content, &mut file)?;

    Ok(())
}

pub fn audio_dir(word: &str, regex: &str, num: usize, directory: &Path) -> Result<Vec<Media>> {
    let mut audio_files: Vec<Media> = vec![];
    let mut count = 0;
    let re = Regex::new(&regex.replace("%word%", word)).context("Unable to parse regex")?;
    for entry in fs::read_dir(directory)? {
        if count >= num {
            break;
        }
        let entry = entry?;
        let path = entry.path();
        let path_str = path.to_str().unwrap();
        if path.is_file() && re.is_match(path_str) {
            let filename = with_uuid(word.to_string());
            let media = Media {
                filename,
                source: Source::Path(path_str.to_string()),
            };
            audio_files.push(media);
            count += 1;
        }
    }
    Ok(audio_files)
}

pub async fn forvo(word: &str, num: usize) -> Result<Vec<Media>> {
    let url = format!("https://forvo.com/search/{}/", word);

    let content = reqwest::get(&url).await?.text().await?;

    let mut pronunciations = vec![];

    let regex_sequence_pattern = Regex::new(r"(Play\(\w+,')(\w+=*)").unwrap();
    for caps in regex_sequence_pattern.captures_iter(content.as_str()) {
        let code_sequence = caps.get(2).unwrap().as_str();
        pronunciations.push(code_sequence.to_string());
    }

    let urls = pronunciations[..num]
        .iter()
        .map(|p| {
            let url = String::from("https://forvo.com/player-mp3Handler.php?path=") + p;
            let filename = with_uuid(word.to_string());
            Media {
                source: Source::Url(url),
                filename,
            }
        })
        .collect::<Vec<Media>>();

    Ok(urls)
}

async fn get_fullres_urls(word: &str) -> Result<Vec<String>> {
    let url = format!("https://google.co.jp/search?q={}&tbm=isch", word);
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Linux; Android 9; SM-G960F Build/PPR1.180610.011; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/74.0.3729.157 Mobile Safari/537.36")
        .build()?;
    let content = client.get(&url).send().await?.text().await?;
    let re = Regex::new(r"AF_initDataCallback\((\{key: 'ds:1'.*?)\);</script>").unwrap();
    let found = re.captures(&content);

    let mut results = vec![];
    if let Some(found) = found {
        let cap = found.get(1);
        if let Some(cap) = cap {
            let json: Value = json5::from_str(cap.as_str()).unwrap();
            let decoded = &json.get("data").unwrap()[31][0][12][2];

            for full_res in decoded.as_array().unwrap() {
                let ent = full_res.get(1);
                if let Some(ent) = ent {
                    let url = &ent[3][0];
                    if !url.is_null() {
                        results.push(url.as_str().unwrap().to_string());
                    }
                }
            }
        }
    }

    return Ok(results);
}

pub async fn google_img(word: String, num: usize) -> Result<Vec<Media>> {
    // try to only shuffle first 10 for relevance
    let urls = get_fullres_urls(&word).await?;
    let max_offset = if urls.len() < num {
        urls.len()
    } else if urls.len() < num + 10 {
        num
    } else {
        num + 10
    };
    let mut shuffled = urls[..max_offset].to_vec();
    shuffled.shuffle(&mut thread_rng());
    if num < shuffled.len() {
        shuffled = shuffled[..num].to_vec();
    }
    let medias = shuffled
        .iter()
        .map(|url| {
            let filename = with_uuid(word.clone());
            Media {
                source: Source::Url(url.to_string()),
                filename,
            }
        })
        .collect::<Vec<Media>>();

    Ok(medias)
}
