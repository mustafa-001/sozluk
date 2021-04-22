use crate::colored_print::print_yellow;
use bincode::{deserialize, serialize};
use byteorder::{BigEndian, ReadBytesExt};
use log::{debug, error};
use rand::{thread_rng, Rng};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use smartstring::{LazyCompact, SmartString};
use std::borrow::Borrow;
use std::convert::{AsRef, TryInto};
use std::error::Error;
use std::fmt::{self};
use std::fs::{read, write, File};
use std::hash::{Hash, Hasher};
use std::io::{self, Read, Seek, SeekFrom};
use std::iter::Iterator;
use std::mem::size_of;
use std::path::PathBuf;
use std::string::FromUtf8Error;

/// Holds the location info about a word's corresponding definition entry
/// in the .dict file.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Index {
    pub word: SmartString<LazyCompact>,
    offset: u32,
    size: u32,
}

#[derive(Debug)]
pub enum DictionaryError {
    IOError,
    PathError,
}

impl Error for DictionaryError {}

impl fmt::Display for DictionaryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Error when loading the dictionary!.")
    }
}

impl From<io::Error> for DictionaryError {
    fn from(_t: io::Error) -> DictionaryError {
        DictionaryError::IOError
    }
}

/// A word, it's definition and `Type` info to represent how the definiton
/// field is formatted.
#[derive(Debug, Serialize)]
pub struct Definition {
    pub word: String,
    pub definition: String,
    definition_type: SameTypeSequence,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
enum SameTypeSequence {
    Meaning,
    Locale,
    Xdfx,
    MediaWiki,
    HTML,
    WordNet,
    Resource,
    Picture,
    None,
}

/// A struct representing a dictionary file in Strdict format.
///
/// Supports loading a dictionary making searches
/// in it's words, reading the definiton of a word and caching.
#[derive(Debug)]
pub struct Dictionary {
    pub indices: Vec<Index>,
    pub idx_path: PathBuf,
    pub dict_path: PathBuf,
    pub ifo_path: PathBuf,
    pub cache_path: PathBuf,
    pub bookname: String,
    pub wordcount: u64,
    sametype_sequence: SameTypeSequence,
    pub preferred_algorithm: Option<String>,
    pub preferred_depth: Option<u8>,
}

impl<'a> Dictionary {
    /// Creates a new dictionary with a given .ifo file path. Other paths will be
    /// generated by modifying this paths extension. This function gives no guarantee
    /// about whether given or other assumed paths exist or whole structure of dictionary
    /// files are usable. Instead use `load_dictionary` method for this guarantees.
    pub fn new(ifo_path: &PathBuf) -> Dictionary {
        debug!("Creating a dictionary with path {}", &ifo_path.display());
        //If directory name has a "." in it .with_extension() get broken.

        Dictionary {
            indices: Vec::new(),
            dict_path: ifo_path.with_extension("dict"),
            idx_path: ifo_path.with_extension("idx"),
            ifo_path: ifo_path.clone(),
            cache_path: ifo_path.with_extension("sozl"),
            bookname: String::from("No bookname"),
            sametype_sequence: SameTypeSequence::None,
            wordcount: 0,
            preferred_algorithm: None,
            preferred_depth: None,
        }
    }

    /// Instantiates a dictionary from give directory or .ifo file path.
    /// Does all plumbing necessary to locate .ifo file, parsing .ifo and .idx
    /// files and cache operations. Return `None`on on
    pub fn load_dictionary(path: &PathBuf) -> Result<Dictionary, DictionaryError> {
        let ifo_path = if path.is_dir() {
            match Dictionary::find_ifo_in_dir(path) {
                Some(n) => n,
                None => return Err(DictionaryError::PathError),
            }
        } else {
            path.to_owned()
        };
        let mut dictionary = Dictionary::new(&ifo_path);
        debug!(
            "dictionary ifo path: {}",
            &dictionary.ifo_path.as_path().display()
        );

        dictionary.parse_ifo_file()?;

        if let Err(_) = dictionary.load_cache() {
            debug!("Failed loading the cache from {:?}", &dictionary.cache_path);
            if let Err(_) = dictionary.parse_index_file() {
                return Err(DictionaryError::IOError);
            }
            if let Err(_) = dictionary.save_cache() {
                debug!("Error when saving index cache.");
            }
        }

        Ok(dictionary)
    }
    /// Returns the .ifo file in given path. If no .ifo file found or path is not a directory
    /// returns None. Returned value use for constructing Dictionary structs.
    fn find_ifo_in_dir(dir: &PathBuf) -> Option<PathBuf> {
        debug!("Looking for .ifo file in {:?}", &dir);
        for entry in dir.read_dir().unwrap() {
            if let Ok(entry) = entry {
                if let Some(n) = entry.path().extension() {
                    if n == "ifo" {
                        return Some(PathBuf::from(entry.path()));
                    }
                }
            }
        }
        None
    }

    pub fn select_random_word(&self) -> &Index {
        let n: usize = thread_rng()
            .gen_range(0, self.wordcount)
            .try_into()
            .unwrap();
        &self.indices[n]
    }

    fn save_cache(&self) -> Result<(), io::Error> {
        let idx: Vec<u8> = serialize(&self.indices).unwrap();
        write(&self.cache_path, &idx)?;
        debug!("Writing cache to {:?}", &self.cache_path);
        Ok(())
    }

    fn load_cache(&mut self) -> Result<(), DictionaryError> {
        debug!("Loading cache from {:?}", &self.cache_path);
        let idx: Vec<u8> = read(&self.cache_path)?;
        self.indices = match deserialize(&idx) {
            Ok(n) => n,
            Err(_) => return Err(DictionaryError::IOError),
        };
        Ok(())
    }

    fn parse_index_file(&mut self) -> Result<(), io::Error> {
        let mut index_file = match File::open(&self.idx_path) {
            Ok(n) => n,
            Err(n) => {
                error!("Error opening index file at: {}", &self.idx_path.display());
                return Err(n);
            }
        };
        let mut indices_raw: Vec<u8> = Vec::new();
        index_file.read_to_end(&mut indices_raw).unwrap();
        self.indices = self.parse_index(indices_raw).unwrap();
        Ok(())
    }

    fn parse_ifo_file(&mut self) -> Result<(), io::Error> {
        let mut ifo_file = File::open(&self.ifo_path)?;
        let mut buffer: String = String::new();
        ifo_file.read_to_string(&mut buffer).ok();
        self.sametype_sequence = match self.parse_field_from_ifo(&buffer, "sametypesequence") {
            Some(n) => Definition::match_sametype_sequence(n.as_str()),
            None => SameTypeSequence::None,
        };
        self.wordcount = match self.parse_field_from_ifo(&buffer, "wordcount") {
            Some(n) => n.parse().unwrap(),
            None => 0,
        };
        self.bookname = match self.parse_field_from_ifo(&buffer, "bookname") {
            Some(n) => String::from(n),
            None => {
                println!("Book doesn't have bookname field");
                String::from(self.dict_path.to_str().unwrap())
            }
        };

        Ok(())
    }

    // This function was just a wrapper, now obsolete. 12 July 2020
    pub fn read_multiple_definitions(
        &self,
        indices: &Vec<&Index>,
    ) -> Result<Vec<Definition>, io::Error> {
        let mut results = Vec::new();
        for ind in indices {
            results.push(self.read_definition(ind)?);
        }
        Ok(results)
    }

    /// Returns shared references to `Index` entries that mathches given closure.
    // pub fn fuzzy_search_indices<T: ?Sized+WordMatcher+Sync>(&self, comparator: &T, word: &str) -> Option<Vec<&Index>> {
    pub fn fuzzy_search_indices<F: Fn(&str, &str) -> bool + Sync>(
        &self,
        comparator: F,
        word: &str,
    ) -> Option<Vec<&Index>> {
        debug!("Searching words matching: {} in {}", &word, &self.bookname);
        let results: Vec<&Index> = self
            .indices
            .par_iter()
            .filter(|x| comparator(&word, &x.word))
            .collect();

        if results.is_empty() {
            None
        } else {
            Some(results)
        }
    }

    fn parse_field_from_ifo(&self, buffer: &'a str, field: &str) -> Option<String> {
        let pattern = format!("{}=", field);
        for line in buffer.lines() {
            if line.starts_with(&pattern) {
                return Some(line.replace(&pattern, ""));
            }
        }
        None
    }

    /// Reads the definition entry from .dict file for a given `Index`. Return
    /// `io::Error`if failed.
    pub fn read_definition(&self, index: &Index) -> Result<Definition, io::Error> {
        let mut file = File::open(&self.dict_path)?;
        file.seek(SeekFrom::Start(index.offset.into())).ok();

        let mut buffer: Vec<u8> = Vec::new();
        buffer.resize(index.size.try_into().unwrap(), 0);
        file.read_exact(&mut buffer).unwrap();

        Ok(Definition::new_from_utf8(
            &index.word,
            buffer,
            &self.sametype_sequence,
        ))
    }

    fn parse_u32<I>(&self, iter: &mut I) -> Result<u32, io::Error>
    where
        I: Iterator<Item = &'a u8>,
    {
        std::iter::Iterator::take(iter, size_of::<u32>())
            .map(|x| *x)
            .collect::<Vec<_>>()
            .as_slice()
            .read_u32::<BigEndian>()
    }

    fn parse_word<I>(&self, iter: &mut I) -> Result<Option<String>, FromUtf8Error>
    where
        I: Iterator<Item = &'a u8>,
    {
        let mut buffer: Vec<u8> = Vec::new();
        let mut next_byte: u8 = match iter.next() {
            Some(n) => *n,
            None => return Ok(None),
        };
        while next_byte != 0 {
            buffer.push(next_byte);
            next_byte = *iter.next().unwrap();
        }
        let res = String::from_utf8(buffer)?;
        Ok(Some(res))
    }

    fn parse_index(&self, raw_indices: Vec<u8>) -> Option<Vec<Index>> {
        let mut indices = Vec::new();
        let mut iter = raw_indices.iter();
        loop {
            let index = Index {
                word: match self.parse_word(&mut iter) {
                    Ok(n) => match n {
                        Some(n) => SmartString::from(n),
                        None => break,
                    },
                    Err(_) => {
                        error!("Error parsing index file, continuing.");
                        continue;
                    }
                },
                offset: match self.parse_u32(&mut iter) {
                    Ok(offset) => offset,
                    Err(_) => {
                        error!("Error parsing index file, continuing.");
                        continue;
                    }
                },
                size: match self.parse_u32(&mut iter) {
                    Ok(size) => size,
                    Err(_) => {
                        error!("Error parsing index file, continuing.");
                        continue;
                    }
                },
            };

            indices.push(index);
        }
        Some(indices)
    }
}

impl Hash for Dictionary {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.bookname.hash(state);
    }
}

impl Definition {
    fn new_from_utf8(word: &str, mut buffer: Vec<u8>, word_type: &SameTypeSequence) -> Definition {
        let word_type = match word_type {
            SameTypeSequence::None => {
                let (type_char, temp) = buffer.split_at(1);
                let r = Box::new(Definition::match_sametype_sequence(
                    &String::from_utf8_lossy(type_char),
                ));
                buffer = temp.to_vec();
                r
            }
            dic_sametype => Box::new(dic_sametype.clone()),
        };
        let mut definition = String::from_utf8(buffer).unwrap();
        if let SameTypeSequence::HTML = word_type.borrow() {
            definition = definition.trim().to_string();
        }
        //TODO Parse definiton according to to word_type.

        Definition {
            word: String::from(word),
            definition,
            definition_type: word_type.as_ref().clone(),
        }
    }

    pub fn print_colored(&self) {
        //TODO Print definition according to definition type.
        print_yellow(&self.word);
        println!("{}\n", &self.definition);
    }

    fn match_sametype_sequence(buffer: &str) -> SameTypeSequence {
        match buffer {
            "m" => SameTypeSequence::Meaning,
            "h" => SameTypeSequence::HTML,
            "l" => SameTypeSequence::Locale,
            "w" => SameTypeSequence::MediaWiki,
            "p" => SameTypeSequence::Picture,
            "n" => SameTypeSequence::WordNet,
            "r" => SameTypeSequence::Resource,
            "x" => SameTypeSequence::Xdfx,
            n => {
                error!(
                    "Unknown or unimplemented sametype sequence  {} \n Falling back to meaning",
                    n
                );
                SameTypeSequence::Meaning
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{io::Write, time::Instant};
    use tempfile::NamedTempFile;

    #[test]
    fn should_parse_index_file() {
        let mut dic = Dictionary::new(&PathBuf::from("notimportant"));
        let idx_content =
            "word1\0\x00\x00\x00\x09\x00\x00\x00\x08word\0\x00\x00\x00\x10\x00\x00\x00\x11"
                .as_bytes();
        println!("index content {:?}", &idx_content);
        let mut idx_file = NamedTempFile::new().unwrap();
        idx_file.write(idx_content).unwrap();
        idx_file.flush().unwrap();
        dic.idx_path = idx_file.path().clone().to_path_buf();
        println!("{:?}", dic.idx_path);
        dic.parse_index_file().unwrap();
        assert_eq!(dic.indices.len(), 2);
    }

    #[test]
    fn should_parse_info_file() {
        let mut dic = Dictionary::new(&PathBuf::from("notimportant"));
        let bookname = "somebookname";
        let sametypesequence = "m";
        let wordcount = 20000;
        let ifo_content = format!(
            "sametypesequence={}\nbookname={}\nwordcount={}\n",
            sametypesequence, bookname, wordcount
        );
        let mut ifo_file = NamedTempFile::new().unwrap();
        ifo_file.write(ifo_content.as_bytes()).unwrap();
        ifo_file.flush().unwrap();
        dic.ifo_path = ifo_file.path().to_path_buf();
        dic.parse_ifo_file().unwrap();
        assert_eq!(dic.bookname, bookname);
        assert_eq!(dic.sametype_sequence, SameTypeSequence::Meaning);
        assert_eq!(dic.wordcount, wordcount);
    }
    #[test]
    fn should_read_definition_from_dict_file() {
        let mut dic = Dictionary::new(&PathBuf::from("notimportant"));
        let dict_content1 = "definition of first word";
        let dict_content2 = "definition of second word";
        let mut dict_file = NamedTempFile::new().unwrap();
        dict_file.write(dict_content1.as_bytes()).unwrap();
        dict_file.write(dict_content2.as_bytes()).unwrap();
        let ind2 = Index {
            word: SmartString::from("word2"),
            offset: dict_content1.len() as u32,
            size: dict_content2.len() as u32,
        };
        dic.dict_path = dict_file.path().to_path_buf();
        dic.sametype_sequence = SameTypeSequence::Meaning;
        let def = dic.read_definition(&ind2).unwrap();
        assert_eq!(def.definition, dict_content2);
    }

    #[test]
    fn test_indexes_sizeof() {
        let _i1 = Index {
            word: SmartString::from("worddfsadfasdf"),
            offset: 10000_u32,
            size: 1111_u32,
        };
        println!("Size of the Index: {:?}", size_of::<Index>());
    }
    #[test]
    fn fuzzy_search() {
    let english= vec![ "suiteth" , "inalterability" , "court-martialled" , "stubbleless" , "returne" , "weak-minded" , "Benin" , "Soton" , "ready-meals" , "outbarks" , "Falcon" , "slaughterhouses" , "Vallone" , "nonweird" , "ball-flower" , "enhardens" , "squirelings" , "tyrannise" , "pennated" , "milting" , "polyed" , "emmarbling" , "secondment" , "suuure" , "degazetting" , "multipoint" , "octaoxygen" , "coaggregate" , "cutinizing" , "poopdecks" , "palaverous" , "quaeritating" , "unguentaria" , "contlines" , "interiorising" , "loanees" , "Utopian" , "metastatic" , "Siu" , "adjuncts" , "disanoint" , "aceprozamine" , "alcoholless" , "911" , "dobupride" , "precognizable" , "anhydrobiosis" , "kegstand" , "orbiculas" , "discocephaline" , ];

    let  turkish = vec![  "patik" , "sunulabilme" , "hipnotizmacı" , "şerefleniş" , "havadarlık" , "vatan" , "sabıkasız" , "temessül" , "karamsarlık" , "tezgâhlayabilmek" , "iktisatsız" , "mümeyyizlik" , "çöpleniş" , "aksam" , "nüzullü" , "kariyer" , "taklip" , "mal" , "inşat" , "toparlanabilmek" , "torlak" , "fizikötesi" , "gerekme" , "başpiskopos" , "göreneksel" , "taslamak" , "denizkedisi" , "totallik" , "rüşvetçi" , "susuz" , "ayazlandırılma" , "gasletme" , "yatırtmak" , "sürtülme" , "peylemek" , "diktirebilmek" , "lika" , "defedivermek" , "Hacıyolu" , "kusuvermek" , "dertleşebilmek" , "kayınbirader" , "operasyon" , "sağcı" , "devirme" , "metalürji" , "şaban" , "katkılı" , "sosyalleşmek" , "nakliyat" , ];

    let french =  vec![  "tôle" , "ponction" , "déplorant" , "dessein" , "traduisons" , "adhérâmes" , "appuyé" , "différent" , "flore" , "accolâmes" , "disgraciâmes" , "effaçai" , "arrangeant" , "créditons" , "poursuivons" , "plongèrent" , "mouvementée" , "grève" , "sifflées" , "préfixe" , "cannibalisme" , "épicent" , "rhum" , "épurant" , "érodées" , "brodées" , "volées" , "fondrière" , "libérent" , "misere" , "acheminèrent" , "achevons" , "artère" , "châtièrent" , "interpola" , "surveillées" , "engendrant" , "l'octave" , "englober" , "cerné" , "anesthésier" , "enfouies" , "endoctriner" , "simplifiés" , "dénigrant" , "robe" , "apprenons" , "bloquez" , "brisés" , "entreposâmes" , ];

    let tr_dict = Dictionary::load_dictionary(&PathBuf::from("dic/gts")).unwrap();
    let en_dict= Dictionary::load_dictionary(&PathBuf::from("dic/wikt-en-en-2018-10-07")).unwrap();
    let fr_dict = Dictionary::load_dictionary(&PathBuf::from("dic/stardict-french-english-2.4.2")).unwrap();
    use crate::matcher::{LevenshteinMatcher, WordMatcher};
    let matcher1 =  LevenshteinMatcher{
        level: 2
    };


    let t1 = Instant::now();
    for w in &turkish {
        tr_dict.fuzzy_search_indices(|w1, w2| matcher1.compare(w1, w2), w);
    }
    println!("Search for {} words in {} took {:?}", turkish.len(), &tr_dict.bookname, t1.elapsed());
    }

    #[test]
    fn should_save_and_restore_index_cache() {
        let mut dic1 = Dictionary::new(&PathBuf::from("notimportant"));
        let i1 = Index {
            word: SmartString::from("a word"),
            offset: 246,
            size: 123,
        };
        let i2 = Index {
            word: SmartString::from("a second word"),
            offset: 492,
            size: 369,
        };
        dic1.cache_path = NamedTempFile::new().unwrap().path().to_path_buf();
        dic1.indices.push(i1);
        dic1.indices.push(i2);
        dic1.save_cache().unwrap();
        let mut dic2 = Dictionary::new(&PathBuf::from("notimportant 2"));
        dic2.cache_path = dic1.cache_path;
        dic2.load_cache().unwrap();
        assert_eq!(dic2.indices[0].offset, 246);
        assert_eq!(dic2.indices[1].word.as_str(), "a second word");
    }
}
