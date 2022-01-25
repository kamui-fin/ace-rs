# ACE (Anime Cards Exporter)

A blazing fast command line program in Rust to automate the creation of [anime cards](https://animecards.site/ankicards/#anime-cardsword-context-cards).

Features include:
- Batch generates Anki cards from the words listed in a text file 
- Example sentences from [massif.la](https://massif.la/ja)
- Audio from [forvo](https://forvo.com/) 
- Custom audio directory support
- Images from [google images](https://images.google.com/)
- Definitions from [yomichan](https://foosoft.net/projects/yomichan/#dictionaries) dictionaries of your choice

This information is packaged into cards and are sent straight to your anki deck with [AnkiConnect](https://ankiweb.net/shared/info/2055492159).

## Installation

```
$ git clone https://github.com/kamui-fin/ace-rs.git
$ cd ace-rs
$ cargo build --release
$ sudo mv ./target/release/ace /usr/local/bin
```

Once you have ace installed, you will need to install the [AnkiConnect](https://ankiweb.net/shared/info/2055492159) plugin in Anki.

## Configuration

Begin by copying over the sample configuration file by running:

```
$ mkdir -p ~/.config/ace
$ mv ./config.sample.toml ~/.config/ace/config.toml
```

Open the config file in your text editor of choice and fill out the keys. All of the variables are documented with comments.

## Usage

If ran without any subcommand, the tool will simply start the card generation and indicate progress. There are a few parameters to tweak some behavior:

- `--config` - Use a custom path for the configuration file

- `--wordfile` - Specify a different file to generate words from

### Managing dictionaries

To get started, you'll need some yomichan dictionaries installed for definitions. This can be done through various subcommands. Here's example of how you would run one:
```
ace subcommand arg1 arg2
```

#### Importing

Use the `import` subcommand and pass in a label and an absolute path to the directory that holds dictionary files. Currently, only yomichan dictionaries with json files are supported.
This effectively loads it into the database and is indexed for fast lookups.

#### Rename

The `rename` subcommand simply takes in `from` and `to` parameters and performs a rename.

#### Listing

To get a general overview of the directories that are currently loaded in, use the `get_dicts` subcommand to list them.
