use anyhow::Result;
use std::fs;

pub fn read_words_file(path: String) -> Result<Vec<String>> {
    let text = fs::read_to_string(path)?;
    let words = text.lines().map(|l| l.to_string()).collect::<Vec<_>>();
    Ok(words)
}
