use log::{debug, warn};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::convert::TryInto;
use std::env::{current_dir, home_dir};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Write};
use std::path::PathBuf;
use structopt::clap::App;
use structopt::StructOpt;

const SETTINGS_PATH: &str = "~/.config/sozluk/settings.json";

#[derive(Debug, Serialize, Deserialize)]
pub struct LangGroup {
    pub paths: Vec<PathBuf>,
    pub matcher_type: String,
    pub matcher_depth: usize,
    pub morpher: String,
}
/// Options structure that manages how program operates. Parses cli arguments,
/// updates them with settings file argument.
#[derive(Debug, StructOpt, Serialize, Deserialize)]
#[structopt(about = "Dictonary for Stardict format.")]
pub struct Opt {
    #[structopt(parse(from_os_str), short, long)]
    pub paths: Option<Vec<PathBuf>>,

    #[structopt(parse(from_os_str), long, default_value = SETTINGS_PATH)]
    pub settings_path: PathBuf,

    #[structopt(short, long)]
    pub group: Option<String>,

    #[structopt(skip)]
    pub groups: HashMap<String, LangGroup>,

    #[structopt(short = "-a", long, default_value = "levenshtein")]
    pub search_algorithm: String,

    #[structopt(short = "-d", long, default_value = "2")]
    pub search_depth: usize,

    #[structopt(short = "-m", long, default_value = "none")]
    pub morpher: String,

    #[structopt(short, long)]
    pub list: bool,

    #[structopt(long)]
    pub list_dictionaries: bool,

    #[structopt(short = "-x", long)]
    pub exit: bool,

    #[structopt(long = "--json")]
    pub json_output: bool,

    #[structopt(long)]
    pub background: bool,

    #[structopt(long, default_value = "timelog.json")]
    pub timelog_file: PathBuf,

    #[structopt(long)]
    pub timelog: bool,

    #[structopt(short = "v")]
    pub verbose: bool,

    pub word: String,
}

impl<'a> Opt {
    ///Reads and returns a corresponding `serde::json::Value` from settings file.
    ///Returns None on failure to find key on the file.
    fn from_settings_file<S: ToString + ?Sized>(&self, key: &'a S) -> Option<Value> {
        let settings_file: File = match File::open(&self.settings_path) {
            Ok(n) => n,
            Err(_) => {
                debug!("Corrupt or nonexisting settings file.");
                return None;
            }
        };
        println!("Reading settings from {:?}", &self.settings_path);

        let v: Option<Value> = serde_json::from_reader(BufReader::new(settings_file)).unwrap();

        v.and_then(|x| x.get(key.to_string()).and_then(|x| Some(x.clone())))
    }

    /// Replaces default values with values from settings file. Keeps the values that user themselves has given.
    pub fn apply_settings_file(&mut self, cli_clap: App) {
        let argmatches = cli_clap.get_matches();
        if argmatches.occurrences_of("paths") == 0 {
            if let Some(Value::String(n)) = self.from_settings_file("paths") {
                self.paths = Some(vec![PathBuf::from(&n)]);
            } else {
                let mut home = home_dir().unwrap();
                let current_dir = current_dir().unwrap();

                let default_paths: Vec<PathBuf> = if cfg!(unix) {
                    debug!("Using Unix specific paths.");
                    home.push(PathBuf::from(".sozluk"));
                    vec![home, current_dir]
                } else if cfg!(windows) {
                    debug!("Using Windows specific paths.");
                    home.push(PathBuf::from(".sozluk"));
                    vec![home, current_dir]
                } else {
                    home.push(PathBuf::from(".sozluk"));
                    debug!("Can't recognize the OS.");
                    vec![current_dir]
                };
                self.paths = Some(default_paths);
            };
        };
        // first if let groups = Value::Object(Map (
        // for ---------------------->key: String, group: Value::Object( <- second if let
        //               |
        //               |----------> key: String, group: Value::Object(
        if let Some(Value::Object(n)) = self.from_settings_file("groups") {
            debug!("groups object {:?}", n);
            for (key, value) in n {
                if let Value::Object(group) = value {
                    let mut paths: Vec<PathBuf> = Vec::new();
                    if let Some(Value::Array(paths_j)) = group.get("paths") {
                        for path_j in paths_j {
                            if let Value::String(path) = path_j {
                                paths.push(PathBuf::from(path));
                            }
                        }
                    }

                    let matcher_type = match group.get("matcher_type") {
                        Some(Value::String(m)) => m.clone(),
                        _ => {
                            warn!("Empty matcher field on group, possibly misconfigurated file.");
                            String::default()
                        }
                    };

                    let matcher_depth = match group.get("matcher_depth") {
                        Some(Value::Number(m)) => m.as_u64().unwrap() as usize,
                        _ => {
                            warn!("Empty matcher field on group, possibly misconfigurated file.");
                            0
                        }
                    };

                    let morpher = match group.get("morpher") {
                        Some(Value::String(m)) => m.clone(),
                        _ => {
                            warn!("Empty matcher field on group, possibly misconfigurated file.");
                            String::default()
                        }
                    };
                    self.groups.insert(
                        key,
                        LangGroup {
                            paths,
                            matcher_type,
                            matcher_depth,
                            morpher,
                        },
                    );
                };
            }
        };

        if let Some(Value::String(n)) = self.from_settings_file("search_algorithm") {
            if argmatches.occurrences_of("search_algorithm") == 0 {
                self.search_algorithm = String::from(n);
            }
        };
        if let Some(Value::Number(n)) = self.from_settings_file("search_depth") {
            if argmatches.occurrences_of("search_depth") == 0 {
                self.search_depth = n.as_u64().unwrap().try_into().unwrap();
            }
        };
    }

    /// Creates an empty settings file on default path.
    pub fn print_settings_file(&self) {
        let mut settings_file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(".settings.json")
            .expect("Cannot open log file");
        let json = serde_json::to_string_pretty(&self).unwrap();
        writeln!(settings_file, "{}", json).unwrap();
    }

    pub fn new() -> Opt {
        Opt {
            paths: Some(vec![PathBuf::from("")]),
            group: None,
            groups: HashMap::new(),
            settings_path: PathBuf::from(""),
            search_algorithm: String::from(""),
            search_depth: 0,
            morpher: String::default(),
            list: false,
            list_dictionaries: false,
            exit: false,
            json_output: false,
            timelog: false,
            timelog_file: PathBuf::from(""),
            background: false,
            verbose: false,
            word: String::from(""),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn should_parse_settings_file() {
        let mut opt = Opt::new();
        opt.paths = Some(vec![PathBuf::from("./dic")]);
        opt.groups = HashMap::new();
        // opt.groups.insert(
        //     "en".to_string(),
        //     vec![PathBuf::from("oxford"), PathBuf::from("gnu")],
        // );
        // opt.groups.insert(
        //     "tr".to_string(),
        //     vec![PathBuf::from("tdk"), PathBuf::from("kubbealtı")],
        // );
        let json = serde_json::to_string(&opt).unwrap();
        println!("index content {:?}", &json);
        let mut settings_file = NamedTempFile::new().unwrap();
        opt.settings_path = settings_file.path().to_path_buf();
        settings_file.write(json.as_bytes()).unwrap();
        settings_file.flush().unwrap();
        assert_eq!(
            opt.from_settings_file("paths").unwrap(),
            Value::Array(vec!(Value::String("./dic".to_string())))
        );
        assert!(opt.from_settings_file("groups").unwrap().is_object());
        if let Value::Object(map) = opt.from_settings_file("groups").unwrap() {
            assert!(map.contains_key("en"));
            assert!(map.contains_key("tr"));
            if let Some(Value::Array(en_dicts)) = map.get("en") {
                assert!(en_dicts.contains(&Value::String("oxford".to_string())));
                assert!(en_dicts.contains(&Value::String("gnu".to_string())));
            };
        }
    }

    #[test]
    fn should_apply_default() {
        let mut default_opt = Opt::new();
        let mut command_line_opt = Opt::new();
        default_opt.word = String::from("default");
        command_line_opt.word = String::from("");
    }

    #[test]
    fn should_read_dictionary_groups() {
        let mut opt = Opt::new();
        opt.groups = HashMap::new();
        opt.groups.insert(
            "en".to_string(),
            LangGroup {
                paths: vec![PathBuf::from("oxford"), PathBuf::from("gnu")],
                matcher_type: String::from("en"),
                matcher_depth: 2,
                morpher: String::from("en"),
            },
        );
        opt.groups.insert(
            "tr".to_string(),
            LangGroup {
                paths: vec![PathBuf::from("tdk"), PathBuf::from("kubbealtı")],
                matcher_type: "tr".to_string(),
                matcher_depth: 2,
                morpher: "tr".to_string(),
            },
        );
        let json = serde_json::to_string(&opt).unwrap();
        let mut settings_file = NamedTempFile::new().unwrap();
        opt.settings_path = settings_file.path().to_path_buf();
        settings_file.write(json.as_bytes()).unwrap();
        settings_file.flush().unwrap();
        opt.apply_settings_file(Opt::clap());
        assert!(opt.groups.contains_key("en"));
        assert_eq!(opt.groups.get("en").unwrap().matcher_type, "en".to_string());
        assert!(opt
            .groups
            .get("tr")
            .unwrap()
            .paths
            .contains(&PathBuf::from("tdk")));
        assert!(opt
            .groups
            .get("tr")
            .unwrap()
            .paths
            .contains(&PathBuf::from("kubbealtı")));
    }

    #[test]
    fn should_not_apply_default() {}
}
