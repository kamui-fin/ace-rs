mod config;

extern crate clap;

use clap::{App, Arg};
use config::Config;
use genanki_rs::{Error, Note};
use std::fs;

fn main() {
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
        .get_matches();

    let config = matches.value_of("config").unwrap_or("config.sample.toml");
    let conf_text = fs::read_to_string(config).unwrap();
    let config: Config = toml::from_str(&conf_text).unwrap();
}
