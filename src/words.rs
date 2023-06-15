use color_eyre::eyre::ContextCompat;
use color_eyre::Result;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{self, BufRead};
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

fn get_random_line(file_path: &PathBuf) -> io::Result<String> {
    // Open the file
    let file = File::open(file_path)?;
    let reader = io::BufReader::new(file);

    // Read the lines into a vector
    let lines: Vec<_> = reader.lines().collect::<io::Result<_>>()?;

    // Generate a random index
    let mut rng = rand::thread_rng();
    let index = rng.gen_range(0..lines.len());

    // Return the random line
    Ok(lines[index].trim().to_string())
}

pub async fn get_random_word(file_path: &PathBuf) -> Result<WordDefinition> {
    let word = get_random_line(file_path)?;

    get_word(&word).await
}

pub async fn get_word(word: &str) -> Result<WordDefinition> {
    let body = reqwest::get(format!(
        "https://api.dictionaryapi.dev/api/v2/entries/en/{word}"
    ))
    .await?
    .json::<Vec<WordDefinition>>()
    .await?;

    Ok(body.first().wrap_err("word not found")?.clone())
}
