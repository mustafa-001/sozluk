use chrono::{DateTime, Local};
use lazy_static::lazy_static;
use serde::Serialize;
use std::fs::File;
use std::io::Write;
use std::sync::Mutex;
use std::time::{Duration, Instant};

lazy_static! {
    pub static ref TIMELOG_FILE: Mutex<Option<File>> = Mutex::new(None);
}

#[derive(Serialize, Debug)]
pub enum Operation {
    Search,
    ReadDefinition,
    LoadDictionary,
    BulkSearch,
    Other,
}

#[derive(Serialize, Debug)]
enum Build {
    Debug,
    Release,
}

#[derive(Serialize, Debug)]
pub struct TimeLog {
    pub clock: Duration,
    pub datetime: DateTime<Local>,
    pub dictionary: Option<String>,
    pub matcher: Option<String>,
    pub word: Option<String>,
    pub operation: Operation,
    pub comment: Option<String>,
    build: Build,
}

impl<'a> TimeLog {
    pub fn new() -> Self {
        const BUILD_TYPE: Build = if cfg!(debug_assertions) {
            Build::Debug
        } else {
            Build::Release
        };

        TimeLog {
            clock: Duration::from_secs(0),
            datetime: Local::now(),
            dictionary: Default::default(),
            matcher: Default::default(),
            word: Default::default(),
            operation: Operation::Other,
            comment: Default::default(),
            build: BUILD_TYPE,
        }
    }

    pub fn write<W, F>(maybe_writable: &Mutex<Option<W>>, func: F)
    where
        W: Write,
        F: FnOnce() -> Self,
    {
        if let Some(ref mut writer) = *maybe_writable.lock().unwrap() {
            let json = serde_json::to_string_pretty(&func()).unwrap();
            write!(writer, "{},", json).unwrap();
        }
    }

    pub fn clock(mut self, clock: Duration) -> Self {
        self.clock = clock;
        self
    }

    pub fn dictionary<T: ToString>(mut self, dictionary: &T) -> Self {
        self.dictionary = Some(dictionary.to_string());
        self
    }

    pub fn matcher<T: ToString>(mut self, matcher: &T) -> Self {
        self.matcher = Some(matcher.to_string());
        self
    }

    pub fn word<T: ToString>(mut self, word: &T) -> Self {
        self.word = Some(word.to_string());
        self
    }

    pub fn operation(mut self, operation: Operation) -> Self {
        self.operation = operation;
        self
    }

    pub fn comment<T: ToString>(mut self, comment: &T) -> Self {
        self.comment = Some(comment.to_string());
        self
    }

    pub fn serialize(&self) -> String {
        serde_json::to_string_pretty(&self).unwrap()
    }

    pub fn span<T, F: FnOnce() -> T>(mut self, func: F) -> (Self, T) {
        let start = Instant::now();
        let rv = func();
        self.clock = start.elapsed();
        (self, rv)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
}
