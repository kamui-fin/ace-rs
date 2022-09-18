mod ace;
mod anki;
mod config;
mod deinflect;
mod dict;
mod media;

use ace::{get_config, package_card};
use anki::{AnkiConnect, NoteData};
use core::time;
use once_cell::sync::OnceCell;
use std::{fs, path::Path, thread::sleep_ms};

use anyhow::{bail, Result};
use clap::{App, Arg, ArgMatches, SubCommand};
use cli_clipboard::{ClipboardContext, ClipboardProvider};
use config::Config;
use dict::DictDb;
use directories::BaseDirs;

static CONFIG: OnceCell<Config> = OnceCell::new();

fn get_matches() -> ArgMatches<'static> {
    let matches = App::new("ace")
        .version("1.0")
        .author("Kamui")
        .about("Vocab card exporter for Anki")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("wordfile")
                .short("f")
                .long("wordfile")
                .value_name("FILE")
                .help("Use a different words file")
                .takes_value(true),
        )
        .subcommand(
            SubCommand::with_name("add").arg(
                Arg::with_name("with-sentence")
                    .long("with-sentence")
                    .short("s")
                    .takes_value(false),
            ),
        )
        .subcommand(
            SubCommand::with_name("import")
                .arg(Arg::with_name("name"))
                .arg(Arg::with_name("path")),
        )
        .subcommand(
            SubCommand::with_name("frequency")
                .arg(Arg::with_name("path"))
                .arg(
                    Arg::with_name("avg")
                        .validator(|arg| {
                            arg.parse::<bool>()
                                .map(|_| ())
                                .map_err(|_| String::from("Must pass a boolean value"))
                        })
                        .default_value("false"),
                )
                .arg(
                    Arg::with_name("corpus")
                        .validator(|arg| {
                            arg.parse::<bool>()
                                .map(|_| ())
                                .map_err(|_| String::from("Must pass a boolean value"))
                        })
                        .default_value("false"),
                ),
        )
        .subcommand(
            SubCommand::with_name("rename")
                .arg(Arg::with_name("oldname"))
                .arg(Arg::with_name("newname")),
        )
        .subcommand(SubCommand::with_name("get_dicts"))
        .get_matches();
    matches
}

fn get_config_path(matches: &ArgMatches, basedirs: &BaseDirs) -> Result<String> {
    let config_dir = basedirs.config_dir().join("ace");
    std::fs::create_dir_all(&config_dir)?;
    let config_file = config_dir.join("config.toml");
    let config_file_string = config_file.to_str().unwrap();

    let config_path = matches.value_of("config").unwrap_or(config_file_string);

    if config_path == config_file.to_str().unwrap() && !config_file.exists() {
        bail!("No configuration file exists")
    }

    let string_path = config_path.to_string();
    Ok(string_path)
}

fn has_updated_config(config_path: String, basedirs: &BaseDirs) -> Result<bool> {
    // Config change detection
    let mut updated_config = false;
    let last_modified = Path::new(&config_path)
        .metadata()
        .unwrap()
        .modified()
        .unwrap();
    let nanos_since = last_modified.elapsed().unwrap().as_nanos();

    let cache_file = basedirs.cache_dir().join("lmod");

    if cache_file.exists() {
        let past_since: u128 = fs::read_to_string(&cache_file).unwrap().parse().unwrap();

        if nanos_since < past_since {
            updated_config = true;
        }
    } else {
        updated_config = true;
    }

    fs::write(cache_file, nanos_since.to_string())?;

    Ok(updated_config)
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut dict_db = DictDb::new()?;
    let matches = get_matches();

    if let Some(matches) = matches.subcommand_matches("frequency") {
        let path = match matches.value_of("path") {
            Some(val) => val,
            None => bail!("Must pass in a frequency list path"),
        };
        let avg = matches.value_of("avg").unwrap().parse::<bool>().unwrap();
        let corpus = matches.value_of("corpus").unwrap().parse::<bool>().unwrap();
        dict_db
            .update_frequency(Path::new(&path), avg, corpus)
            .unwrap();
        return Ok(());
    }

    if let Some(matches) = matches.subcommand_matches("import") {
        let name = match matches.value_of("name") {
            Some(val) => val,
            None => bail!("Must pass in a dictionary name"),
        };
        let path = match matches.value_of("path") {
            Some(val) => val,
            None => bail!("Must pass in a dictionary path"),
        };
        dict_db.load_yomichan_dict(Path::new(&path), name.to_string())?;
        return Ok(());
    }

    let basedirs = BaseDirs::new().expect("Failed to query base directories");
    let config_path = get_config_path(&matches, &basedirs)?;
    let config = Config::from_path(&config_path)?;
    CONFIG.set(config).unwrap();
    let config = get_config()?;

    let updated_config = has_updated_config(config_path, &basedirs)?;

    if updated_config {
        for (name, info) in config.dict.iter() {
            let new_fallback = if info.fallback { 1 } else { 0 };
            let new_enabled = if info.enabled { 1 } else { 0 };
            dict_db.update_dict(name, info.priority, new_fallback, new_enabled)?;
        }
    }

    if let Some(matches) = matches.subcommand_matches("add") {
        let anki_connect = AnkiConnect {
            port: config.ankiconnect.port,
            address: config.ankiconnect.address.clone(),
        };
        anki_connect.status().await?;
        let mut ctx = ClipboardContext::new()?;

        let mut sentence = String::new();
        let mut word = sentence.clone();
        if matches.is_present("with-sentence") {
            sentence = ctx.get_contents()?;

            let mut elapsed_ms = 0;
            let wait_time_ms = 500; // ms
            let max_time = 5000;

            notifica::notify(
                "Vocab Card",
                &format!(
                    "Copy a word to the clipboard within {} seconds",
                    max_time / 1000
                ),
            );

            while elapsed_ms <= max_time {
                sleep_ms(wait_time_ms);
                elapsed_ms += wait_time_ms;
                word = ctx.get_contents()?;
                if !word.is_empty() && word != sentence {
                    break;
                }
            }

            if word.is_empty() || word == sentence {
                bail!("No word provided");
            }
        } else {
            word = ctx.get_contents()?;
        }
        let note_data = package_card(&dict_db, &word, sentence).await?;
        if let Some(note_data) = note_data {
            anki_connect.add_card(note_data).await?;
        }

        return Ok(());
    }

    if let Some(matches) = matches.subcommand_matches("rename") {
        let old = match matches.value_of("oldname") {
            Some(val) => val,
            None => bail!("Must pass in the old name"),
        };
        let new = match matches.value_of("newname") {
            Some(val) => val,
            None => bail!("Must pass in a new name"),
        };
        dict_db.rename_dict(old, new)?;
        return Ok(());
    }

    if matches.subcommand_matches("get_dicts").is_some() {
        let dicts = dict_db.get_all_dicts()?;
        println!(
            "{0: <10} | {1: <10} | {2: <10} | {3: <10}",
            "title", "priority", "fallback", "enabled"
        );
        for dict in dicts {
            println!(
                "{0: <10} | {1: <10} | {2: <10} | {3: <10}",
                dict.title, dict.priority, dict.fallback, dict.enabled
            );
        }
        return Ok(());
    }

    let finalized_path = &config.words_file.clone();
    let words_file = Path::new(matches.value_of("wordfile").unwrap_or(finalized_path));

    ace::export_words(&dict_db, words_file).await?;

    Ok(())
}
