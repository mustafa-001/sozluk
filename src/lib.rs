pub mod colored_print;
pub mod dictionary;
pub mod matcher;
pub mod morpher;
pub mod performance_log;
pub mod server;
pub mod settings;

use dictionary::{Definition, Dictionary, Index};
use log::{debug, error, info};
use matcher::{ExactMatcher, LevenshteinMatcher, WordMatcher};
use morpher::{EnglishMorpher, Morpher, NoMorpher, TurkishMorpher};
use performance_log::{Operation, TimeLog, TIMELOG_FILE};
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self};
use std::path::PathBuf;
use std::time::Instant;

pub struct IndexDictPair<'a> {
    pub index: Vec<&'a Index>,
    pub dict: &'a Dictionary,
}

pub fn build_matcher(algorithm: &str, depth: usize) -> Box<dyn WordMatcher + Sync> {
    let comp: Box<dyn WordMatcher + Sync> = match algorithm {
        "levenshtein" => Box::from(LevenshteinMatcher { level: depth }),
        _ => Box::from(ExactMatcher {}),
    };
    comp
}

pub fn search_in_dicts<'a, D, M: ?Sized + WordMatcher + Sync>(
    dicts: &mut D,
    comp: &M,
    word: &str,
) -> Vec<IndexDictPair<'a>>
where
    D: Iterator<Item = &'a Dictionary>,
{
    let mut indices_to_list: Vec<IndexDictPair> = Vec::new();
    for dic in dicts {
        let start_time = Instant::now();
        let indices = dic.fuzzy_search_indices(|w1, w2| comp.compare(w1, w2), word);
        TimeLog::write(&TIMELOG_FILE, || {
            TimeLog::new()
                .clock(start_time.elapsed())
                .dictionary(&dic.bookname)
                .word(&word)
                .operation(Operation::Search)
                .matcher(&comp.name())
        });

        if let Some(indices) = indices {
            indices_to_list.push(IndexDictPair {
                index: indices,
                dict: &dic,
            });
        } else {
            debug!("Found no result in {}", &dic.bookname);
        }

        debug!(
            "Searched {} with {} in {:?}.",
            word,
            comp.name(),
            start_time.elapsed()
        );
    }
    indices_to_list
}
pub fn indices_to_json(pairs: &Vec<IndexDictPair>) -> String {
    let mut output: HashMap<String, Vec<Definition>> = HashMap::new();
    for pair in pairs {
        let mut words = Vec::new();
        for index in &pair.index {
            words.push(pair.dict.read_definition(index).unwrap());
        }
        output.insert(pair.dict.bookname.clone(), words);
    }
    serde_json::to_string_pretty(&output).unwrap()
}

pub fn load_dicts_from_paths_and_subpaths(paths: &Vec<PathBuf>) -> Vec<Dictionary> {
    let mut dicts: Vec<Dictionary> = Vec::new();
    for path in paths {
        debug!("Trying to load from {:?} ", &path);
        if path.as_path().is_dir() {
            //Try to load sub-directories.
            let sub_paths: Vec<PathBuf> = fs::read_dir(&path)
                .unwrap()
                .filter_map(|x| x.ok())
                .map(|x| x.path())
                .filter(|x| x.is_dir())
                .collect();

            dicts.append(
                &mut sub_paths
                    .par_iter()
                    .filter_map(|x| Dictionary::load_dictionary(x).ok())
                    .collect(),
            );

            //Try to load this directory itself.
            if let Ok(n) = Dictionary::load_dictionary(&path) {
                dicts.push(n)
            }

        //TODO If .gz or some sort of default_compressed dictionary file.
        } else {
            // dicts.push(Dictionary::load_dictionary(path).unwrap());
        }
    }
    dicts
}
