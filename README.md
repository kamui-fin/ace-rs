# ACE (Anime Cards Exporter)

A blazing fast command line program in Rust to automate the creation of [anime cards](https://animecards.site/ankicards/#anime-cardsword-context-cards).

Features include:

- Interactive, hotkey-oriented workflow to swiftly create cards from immersion
- Batch generates Anki cards from the words listed in a text file
- Support for Chinese and Japanese
  - Automatic deinflection for verbs
- Example sentences from [massif.la](https://massif.la/ja) and [Tatoeba](https://tatoeba.org/zh-cn/)
- Audio from [forvo](https://forvo.com/)
  - Custom audio server support
- Images from [google images](https://images.google.com/)
- Definitions from [yomichan](https://foosoft.net/projects/yomichan/#dictionaries) dictionaries of your choice
  - Additional configuration supported such as priority, fallback, etc.
- Pinyin generation
- Frequency-based results ordering
- Duplication handling
- Straightforward TOML configuration
- Cross platform

The word information and accompanying media are packaged into individual cards and are sent straight to your anki deck with [AnkiConnect](https://ankiweb.net/shared/info/2055492159) at once.

![demo](./screenshots/demo.mp4)

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

### Interactive

An alternative "mode", which some may prefer over the batch generation, is interactive on-the-spot card generation enhanced with hotkeys.
There is a subcommand called `add` that has different behavior depending on whether the flag `--with-sentence` (`-s`) is toggled.

Without the flag, it uses the word copied to your clipboard in order to make a card for it.
Conversely, with the flag, a sentence should be copied beforehand and the program waits for `5` seconds for the user to copy the word they wish to make a card for.

It is possible to link hotkeys to this functionality, however, that largely varies depending on your operating system and desktop environment.
For some pointers to configuring this in common environments:

- X11 DE-independant: [sxhkd](https://github.com/baskerville/sxhkd)
- Gnome: [Keyboard settings](https://docs.fedoraproject.org/en-US/quick-docs/proc_setting-key-shortcut/)
- KDE: [KHotkeys](https://docs.kde.org/trunk5/en/khotkeys/kcontrol/khotkeys/khotkeys.pdf)
- Windows: [AHK](https://www.autohotkey.com/docs/commands/Run.htm)
- Mac OS: [Automator](https://www.howtogeek.com/286332/how-to-run-any-mac-terminal-command-with-a-keyboard-shortcut/)

### Managing dictionaries

To get started, you'll need some yomichan dictionaries installed for definitions. This can be done through various subcommands.

```
ace subcommand arg1 arg2
```

#### Importing

Use the `import` subcommand and pass in a label and an absolute path to the directory that holds dictionary files. Currently, only yomichan dictionaries with json files are supported.
This effectively loads it into the database and is indexed for fast lookups.

```
ace import [dict-name] [dict-path]
```

#### Frequency Lists

To use frequency lists for better lookup results that are ranked according to their frequency, use the `frequency` subcommand.
It requires a path to the frequency list, which should be in yomichan format.

```
ace frequency [freq-path]
```

Optional boolean parameters:

1. `avg` - When adding more lists, average all the frequencies of past lists. Defaults to `false`.
2. `corpus` - When the higher list's "frequency" values, the higher the actual frequency. Usually this is the case with corpus lists. Defaults to `false`.

#### Rename

To rename an existing dictionary:

```
ace rename [old-name] [new-name]
```

#### Listing

To get a general overview of the directories that are currently loaded in, use the `get_dicts` subcommand to list them.

```
ace get_dicts
```

Example output:

```
title      | priority   | fallback   | enabled
cedict     | 9999       | false      | true
```
