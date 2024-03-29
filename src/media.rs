use anyhow::{Context, Result};
use rand::seq::SliceRandom;
use rand::thread_rng;
use regex::Regex;
use scraper::{Html, Selector};
use serde_json::Value;
use std::{collections::HashMap, io::Cursor, path::Path};
use uuid::Uuid;

use crate::anki::Media;

fn with_uuid(prefix: String) -> String {
    let uuid = Uuid::new_v4().to_string();
    format!("{}-{}", prefix, uuid)
}

async fn general_text_select(url: &str, selector: &str) -> Result<String> {
    let resp = reqwest::get(url).await?;
    let document = Html::parse_document(&resp.text().await?);
    let selector = Selector::parse(selector).unwrap();
    let sent_div = document.select(&selector).next().context("No sentence")?;
    let sent_text = sent_div.text().collect::<Vec<_>>().join("");

    Ok(sent_text)
}

async fn fetch_massif(word: &str) -> Result<String> {
    general_text_select(
        format!("https://massif.la/ja/search?q={}", word).as_str(),
        "li.text-japanese > div:not(.result-meta)",
    )
    .await
}

fn trim_number(text: String) -> String {
    let mut offset_map = HashMap::new();
    offset_map.insert('1', 2); // exception: 1 and space
    offset_map.insert('一', 2);
    offset_map.insert('(', 3);
    offset_map.insert('（', 3);
    offset_map.insert('一', 2);

    if text.starts_with("1 ") {
        offset_map.insert('1', 1);
    }

    let new_text = text.replace(' ', "");
    let mut chars = new_text.chars().peekable();
    for _ in 0..offset_map[chars.peek().unwrap()] {
        chars.next();
    }
    chars.collect::<String>()
}

async fn fetch_zaojv(word: &str) -> Result<String> {
    let mut params = HashMap::new();
    params.insert("wo", word);

    let client = reqwest::Client::builder().build()?;
    let resp = client
        .post("https://zaojv.com/wordQueryDo.php")
        .form(&params)
        .send()
        .await?;

    let document = Html::parse_document(&resp.text().await?);
    let selector = Selector::parse(".dotline a").unwrap();
    let doc_link = document
        .select(&selector)
        .next()
        .context("No sentence")?
        .value()
        .attr("href")
        .context("No href")?;

    let text_res = general_text_select(
        format!("https://zaojv.com/{}", doc_link).as_str(),
        "#student > div",
    )
    .await;

    if let Ok(text) = &text_res {
        return Ok(trim_number(text.to_string()));
    }
    text_res
}

async fn fetch_chineseboost(word: &str) -> Result<String> {
    let sentence = general_text_select(
        format!(
            "https://www.chineseboost.com/chinese-example-sentences?query={}",
            word
        )
        .as_str(),
        ".liju .hanzi.sentence",
    )
    .await;
    if let Ok(sent) = &sentence {
        if sent.contains(word) {
            return sentence;
        }
    }
    // try to search in zaojv.com as fallback
    return fetch_zaojv(word).await;
}

pub async fn get_sent(word: &str, is_japanese: bool) -> Result<String> {
    if is_japanese {
        fetch_massif(word).await
    } else {
        fetch_chineseboost(word).await
    }
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

pub async fn fetch_audio_server(word: &str, custom_audio_server: &str) -> Result<Media> {
    let url = custom_audio_server.replacen("{}", word, 1);
    reqwest::get(&url).await?.error_for_status()?;
    let filename = with_uuid(word.to_string());

    Ok(Media { url, filename })
}

pub async fn forvo(word: &str) -> Result<Media> {
    let url = format!("https://forvo.com/search/{}/", word);

    let content = reqwest::get(&url).await?.text().await?;

    let regex_sequence_pattern = Regex::new(r"(Play\(\w+,')(\w+=*)").unwrap();
    let code_sequence = regex_sequence_pattern
        .captures_iter(content.as_str())
        .next()
        .ok_or("Could not find forvo pronunciation")
        .unwrap()
        .get(2)
        .unwrap()
        .as_str();
    let url = String::from("https://forvo.com/player-mp3Handler.php?path=") + code_sequence;
    let filename = with_uuid(word.to_string());
    Ok(Media { url, filename })
}

fn filter_nested_value(value: &Value) -> Vec<&[Value]> {
    match value {
        Value::Array(arr) if arr.len() == 3 => vec![arr.as_slice()],
        Value::Array(arr) => arr.iter().flat_map(filter_nested_value).collect(),
        Value::Object(obj) => obj.values().flat_map(filter_nested_value).collect(),
        _ => vec![],
    }
}

async fn get_fullres_urls(word: &str, is_japanese: bool) -> Result<Vec<String>> {
    // TODO: use better image source for Chinese
    let country = if is_japanese { "co.jp" } else { "com.hk" };

    let url = format!("https://google.{}/search?q={}&tbm=isch", country, word);
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Linux; Android 9; SM-G960F Build/PPR1.180610.011; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/74.0.3729.157 Mobile Safari/537.36")
        .build()?;
    let content = client.get(&url).send().await?.text().await?;
    let re = Regex::new(r"AF_initDataCallback\((\{key: 'ds:1'.*?)\);</script>").unwrap();
    let found = re.captures(&content);

    if let Some(found) = found {
        let cap = found.get(1);
        if let Some(cap) = cap {
            let json: Value = json5::from_str(cap.as_str()).unwrap();
            let decoded = &json.get("data").unwrap()[56];
            let urls: Vec<String> = filter_nested_value(decoded)
                .into_iter()
                .filter_map(|arr| match arr {
                    [Value::String(string_val), Value::Number(_), Value::Number(_)]
                        if !string_val.starts_with("https://encrypted-") =>
                    {
                        Some(string_val.to_string())
                    }
                    _ => None,
                })
                .collect();

            return Ok(urls);
        }
    }

    Ok(vec![])
}

pub async fn google_img(word: String, is_japanese: bool) -> Result<Media> {
    let urls = get_fullres_urls(&word, is_japanese).await?;
    let max_offset = if urls.len() < 10 { urls.len() } else { 10 };
    let mut shuffled = urls[..max_offset].to_vec();
    shuffled.shuffle(&mut thread_rng());
    let url = shuffled.get(0).ok_or("Could not find image").unwrap();
    let filename = with_uuid(word.clone());
    Ok(Media {
        url: url.to_string(),
        filename,
    })
}
