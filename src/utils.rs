use anyhow::Result;
use regex::Regex;
use scraper::{Html, Selector};
use std::{io::Cursor, path::Path};

#[tokio::main]
pub async fn get_sent(word: &str) -> Result<String> {
    let base_url = String::from("https://massif.la/ja/search?q=");
    let resp = reqwest::get(base_url + word).await?;

    let document = Html::parse_document(&resp.text().await?);
    let selector = Selector::parse("li.text-japanese > div:not(.result-meta)").unwrap();
    let sent_div = document.select(&selector).next().unwrap();
    let sent_text = sent_div.text().collect::<Vec<_>>().join("");

    Ok(sent_text)
}

#[tokio::main]
pub async fn forvo_dl(word: &str, output_dir: &Path) -> Result<()> {
    let url = format!("https://forvo.com/search/{}/", word);

    let content = reqwest::get(&url).await?.text().await?;

    let mut pronunciations = vec![];

    let regex_sequence_pattern = Regex::new(r"(Play\(\w+,')(\w+=*)").unwrap();
    for caps in regex_sequence_pattern.captures_iter(content.as_str()) {
        let code_sequence = caps.get(2).unwrap().as_str();
        pronunciations.push(code_sequence.to_string());
    }

    // for now just get the first pronounciation
    if let Some(pronunciation) = pronunciations.get(0) {
        let dl_link = String::from("https://forvo.com/player-mp3Handler.php?path=") + pronunciation;
        let response = reqwest::get(&dl_link).await?;
        let bytes = response.bytes().await?;
        let mut file = std::fs::File::create(output_dir.join(format!("{}-forvo.mp3", word)))?;
        let mut content = Cursor::new(bytes);
        std::io::copy(&mut content, &mut file)?;
    }

    Ok(())
}
