mod ace;
mod anki;
mod config;
mod deinflect;
mod dict;
mod media;

use std::{fs, path::Path};

use anyhow::Result;
use clap::{App, Arg, SubCommand};
use config::Config;
use dict::DictDb;
use directories::BaseDirs;

#[tokio::main]
async fn main() -> Result<()> {
    let matches = App::new("ace")
        .version("1.0")
        .author("Kamui")
        .about("Anime card exporter for Anki")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file")
                .takes_value(true),
        )
        .subcommand(
            SubCommand::with_name("import")
                .arg(Arg::with_name("name"))
                .arg(Arg::with_name("path")),
        )
        .subcommand(
            SubCommand::with_name("rename")
                .arg(Arg::with_name("oldname"))
                .arg(Arg::with_name("newname")),
        )
        .subcommand(SubCommand::with_name("get_dicts"))
        .get_matches();

    // Load configuration
    let config_path = matches.value_of("config").unwrap_or("config.sample.toml");
    let config = Config::from_path(config_path);

    // Config change detection
    let mut updated_config = false;
    let last_modified = Path::new(config_path)
        .metadata()
        .unwrap()
        .modified()
        .unwrap();
    let nanos_since = last_modified.elapsed().unwrap().as_nanos();

    let basedirs = BaseDirs::new().unwrap();
    let cache_file = basedirs.cache_dir().join("lmod");

    if cache_file.exists() {
        let past_since: u128 = fs::read_to_string(&cache_file).unwrap().parse().unwrap();

        if nanos_since < past_since {
            updated_config = true;
        }
    }

    fs::write(cache_file, nanos_since.to_string())?;

    let mut dict_db = DictDb::new()?;

    if updated_config {
        for (name, info) in config.dict.iter() {
            let new_fallback = if info.fallback { 1 } else { 0 };
            dict_db.update_dict(name, info.priority, new_fallback);
        }
    }

    if let Some(matches) = matches.subcommand_matches("import") {
        let name = matches.value_of("name").unwrap();
        let path = matches.value_of("path").unwrap();
        dict_db.load_yomichan_dict(Path::new(&path), name.to_string())?;
    }

    if let Some(matches) = matches.subcommand_matches("rename") {
        let old = matches.value_of("oldname").unwrap();
        let new = matches.value_of("newname").unwrap();
        dict_db.rename_dict(old, new)?;
    }

    if matches.subcommand_matches("get_dicts").is_some() {
        let dicts = dict_db.get_all_dicts()?;
        println!(
            "{0: <10} | {1: <10} | {2: <10}",
            "title", "priority", "fallback"
        );
        for dict in dicts {
            println!(
                "{0: <10} | {1: <10} | {2: <10}",
                dict.title, dict.priority, dict.fallback
            );
        }
    }

    Ok(())
}
