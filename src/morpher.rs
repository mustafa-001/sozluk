pub trait Morpher {
    fn possible_roots(&self, word: &str) -> Vec<String>;
}

pub struct NoMorpher {}
impl Morpher for NoMorpher {
    fn possible_roots(&self, word: &str) -> Vec<String> {
        vec![String::from(word)]
    }
}

pub struct EnglishMorpher {}

impl Morpher for EnglishMorpher {
    fn possible_roots(&self, word: &str) -> Vec<String> {
        vec![String::from(word)]
    }
}
pub struct TurkishMorpher {}

impl Morpher for TurkishMorpher {
    fn possible_roots(&self, word: &str) -> Vec<String> {
        let mut roots = vec![String::from(word)];
        if word.ends_with("ler") {
            roots.push(word.strip_suffix("ler").unwrap().to_string())
        }
        roots
    }
}
