use color_eyre::eyre::ContextCompat;
use color_eyre::Result;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WordDefinition {
    pub word: String,
    pub meanings: Vec<Meaning>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Meaning {
    pub definitions: Vec<Definition>,
    pub part_of_speech: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Definition {
    pub definition: String,
}

pub fn get_line_count(file_path: &PathBuf) -> io::Result<u64> {
    let mut reader = reader::BufReader::open(file_path)?;
    let mut buffer = String::new();
    let mut count: u64 = 0;

    while let Some(_) = reader.read_line(&mut buffer) {
        count += 1;
    }

    Ok(count)
}

fn get_random_line(file_path: &PathBuf, total_lines: u64) -> io::Result<String> {
    // Open the file
    let mut reader = reader::BufReader::open(file_path)?;
    let mut buffer = String::new();
    let mut current_index: u64 = 0;

    // Generate a random index
    let mut rng = rand::thread_rng();
    let target_index = rng.gen_range(0..total_lines);
    let mut word = String::new();

    while let Some(line) = reader.read_line(&mut buffer) {
        if current_index != target_index {
            current_index += 1;
            continue;
        }

        if let Ok(w) = line {
            word = w.clone();
        }

        break;
    }

    Ok(word.trim().to_string())
}

pub async fn get_random_word(file_path: &PathBuf, total_lines: u64) -> Result<WordDefinition> {
    let word = get_random_line(file_path, total_lines)?;

    get_word(&word).await
}

pub async fn get_word(word: &str) -> Result<WordDefinition> {
    let buf = &mut String::new();
    let word = url_escape::encode_path_to_string(word, buf);
    let body = reqwest::get(format!(
        "https://api.dictionaryapi.dev/api/v2/entries/en/{word}"
    ))
    .await?
    .json::<Vec<WordDefinition>>()
    .await?;

    Ok(body.first().wrap_err("word not found")?.clone())
}

mod reader {
    use std::{
        fs::File,
        io::{self, prelude::*},
    };

    pub struct BufReader {
        reader: io::BufReader<File>,
    }

    impl BufReader {
        pub fn open(path: impl AsRef<std::path::Path>) -> io::Result<Self> {
            let file = File::open(path)?;
            let reader = io::BufReader::new(file);

            Ok(Self { reader })
        }

        pub fn read_line<'buf>(
            &mut self,
            buffer: &'buf mut String,
        ) -> Option<io::Result<&'buf mut String>> {
            buffer.clear();

            self.reader
                .read_line(buffer)
                .map(|u| if u == 0 { None } else { Some(buffer) })
                .transpose()
        }
    }
}
