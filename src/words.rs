use color_eyre::eyre::{eyre, ContextCompat};
use color_eyre::Result;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::io::{self, BufRead};
use std::path::Path;

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

pub fn get_line_count(file_path: &Path) -> io::Result<u64> {
    let lines = read_lines(file_path)?;
    let mut count: u64 = 0;
    for line in lines {
        if let Ok(_) = line {
            count += 1;
        }
    }

    Ok(count)
}

fn get_random_line(file_path: &Path, total_lines: u64) -> Result<String> {
    // Generate a random index
    let mut rng = rand::thread_rng();
    let target_index: usize = rng.gen_range(0..total_lines).try_into()?;

    let lines = read_lines(file_path)?;
    let mut word = None;
    for (index, line) in lines.enumerate() {
        if index != target_index {
            continue;
        }

        if let Ok(w) = line {
            word = Some(w);
        }
    }

    Ok(word
        .ok_or(eyre!("failed to select a random line"))?
        .trim()
        .to_string())
}

pub async fn get_random_word(file_path: &Path, total_lines: u64) -> Result<WordDefinition> {
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

// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<std::fs::File>>>
where
    P: AsRef<std::path::Path>,
{
    let file = std::fs::File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
