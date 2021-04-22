use std::convert::TryFrom;
use std::fmt::{Debug, Formatter, Result, Write};
use strsim::normalized_levenshtein;

pub trait WordMatcher {
    fn compare(&self, first: &str, second: &str) -> bool;
    fn name(&self) -> String;
    // fn best_matches(&self, pool: &Vec<&str>, word: &str, number: usize) -> Vec<Index>;
}

impl Debug for dyn WordMatcher {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.debug_tuple("").field(&self.name()).finish()
    }
}
pub struct ExactMatcher {}

impl WordMatcher for ExactMatcher {
    fn compare(&self, first: &str, second: &str) -> bool {
        first == second
    }

    fn name(&self) -> String {
        String::from("Exact Matcher")
    }
}

pub struct LevenshteinMatcher {
    pub level: usize,
}

impl WordMatcher for LevenshteinMatcher {
    fn compare(&self, first: &str, second: &str) -> bool {
        let delta = i8::try_from(first.chars().count()).unwrap()
            - i8::try_from(second.chars().count()).unwrap();
        if delta > i8::try_from(self.level).unwrap()
            || delta < i8::try_from(self.level).unwrap() * -1
        {
            return false;
        }
        normalized_levenshtein(first, second) > 0.89 - 0.05 * f64::from(self.level as u32)
    }

    fn name(&self) -> String {
        let mut n = String::new();
        write!(n, "Levenshtein matcher {}", self.level).unwrap();
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_plain_matcher_match_same() {
        let matcher = ExactMatcher {};
        assert!(matcher.compare("elma", "elma"));
    }

    #[test]
    fn should_plain_not_matcher_match_different() {
        let matcher = ExactMatcher {};
        assert!(!matcher.compare("elmalar", "elma"));
    }

    #[test]
    fn should_levenshtein_matcher_match_same() {
        let matcher = LevenshteinMatcher { level: 3 };
        assert!(matcher.compare("armut", "ermıt"));
        assert!(matcher.compare("armut", "erm"));
        assert!(matcher.compare("Armut", "armutar"));
    }
}
