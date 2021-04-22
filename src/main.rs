use ctrlc::set_handler;
use log::{debug, error, info};
use simplelog::{Config, LevelFilter, TermLogger, TerminalMode};
use sozluk::colored_print::{print_green, print_yellow};
use sozluk::dictionary::{Definition, Dictionary, Index};
use sozluk::load_dicts_from_paths_and_subpaths;
use sozluk::morpher::{EnglishMorpher, Morpher, NoMorpher, TurkishMorpher};
use sozluk::performance_log::{Operation, TimeLog, TIMELOG_FILE};
use sozluk::server::serve_http;
use sozluk::settings::Opt;
use sozluk::{build_matcher, indices_to_json, search_in_dicts, IndexDictPair};
use std::fs::{self, OpenOptions};
use std::io::{self};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use structopt::StructOpt;

fn main() -> std::io::Result<()> {
    if cfg!(debug_assertions) {
        TermLogger::init(LevelFilter::Trace, Config::default(), TerminalMode::Stdout).unwrap();
    }
    let mut opt = Opt::from_args();
    opt.apply_settings_file(Opt::clap());

    if !opt.verbose && !cfg!(debug_assertions) {
        TermLogger::init(LevelFilter::Info, Config::default(), TerminalMode::Mixed).unwrap();
    }
    *TIMELOG_FILE.lock().unwrap() = if opt.timelog || cfg!(debug_assertions) {
        Some(
            OpenOptions::new()
                .append(true)
                .create(true)
                .open(&opt.timelog_file)
                .expect("Cannot open log file"),
        )
    } else {
        None
    };

    debug!("{:#?}", &opt);
    let running = Arc::new(AtomicBool::new(false));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(true, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    if opt.list_dictionaries {
        print_dictionaries(opt.paths.unwrap());
        panic!()
    };

    let start_time = Instant::now();
    let mut dicts: Vec<Dictionary> = Vec::new();
    if let Some(ref key) = &opt.group {
        if let Some(group) = opt.groups.get(key) {
            let d = load_dicts_from_paths_and_subpaths(&group.paths);
            if d.len() >= 1 {
                dicts = d;
            }
        } else {
            error!(
                "No dictionary file (dict.dz) or dictionary directory found in given group paths!"
            );
            info!("Falling back to default paths.");
        }
    }
    if dicts.len() == 0 {
        //This unwrap is safe because at this point opt.paths at least have default OS specific paths.
        let d = load_dicts_from_paths_and_subpaths(&opt.paths.as_ref().unwrap());
        if d.len() >= 1 {
            dicts = d
        } else {
            error!("No dictionary file (dict.dz) or dictionary directory found in given paths!");
            return Ok(());
        }
    }

    TimeLog::write(&TIMELOG_FILE, || {
        TimeLog::new()
            .clock(start_time.elapsed())
            .operation(Operation::LoadDictionary)
    });

    let morpher: &dyn Morpher = match opt.morpher.as_ref() {
        "tr" => &TurkishMorpher {},
        "en" => &EnglishMorpher {},
        "none" | _ => &NoMorpher {},
    };

    let default_comp = build_matcher(&opt.search_algorithm, opt.search_depth);

    //TODO Move all this logic to seperate function. Webserver logic should be completely seperate from
    //other parts of the app.
    if opt.background {
        serve_http(&opt);
    }

    loop {
        let possible_roots = morpher.possible_roots(&opt.word);
        let indices_to_list: Vec<IndexDictPair> = possible_roots
            .iter()
            .map(|word| search_in_dicts(&mut dicts.iter(), default_comp.as_ref(), &word))
            .flatten()
            .collect();

        if indices_to_list.len() == 0 {
            if !opt.json_output {
                println!("Found no result!")
            }
        }

        if opt.json_output {
            println!("{}", &indices_to_json(&indices_to_list));
            break;
        } else if opt.list {
            listed_interface(&indices_to_list);
        } else {
            print_defs(&indices_to_list.as_slice());
        }

        if opt.exit {
            break;
        } else {
            if running.load(Ordering::SeqCst) {
                break;
            }
            print_yellow("Enter a word to search or z to exit.");
            let mut buffer = String::new();
            io::stdin().read_line(&mut buffer)?;
            if running.load(Ordering::SeqCst) {
                break;
            }
            if buffer.trim().eq_ignore_ascii_case("z") {
                break;
            } else {
                opt.word = buffer.trim().to_string();
            };
        }
    }

    Ok(())
}

fn print_dictionaries(paths: Vec<PathBuf>) {
    let mut dicts: Vec<Dictionary> = Vec::new();
    for path in &paths {
        for entry in fs::read_dir(path).unwrap() {
            if let Ok(dictionary) = Dictionary::load_dictionary(&entry.as_ref().unwrap().path()) {
                dicts.push(dictionary);
            }
        }
    }
    for (i, entry) in dicts.iter().enumerate() {
        println!(
            "{}:   {}\t {}",
            i + 1,
            entry.bookname,
            entry.idx_path.display()
        );
    }
}

fn listed_interface(pairs: &Vec<IndexDictPair>) {
    let mut index_count = 0;
    for pair in pairs {
        print_green(format!("From {:?}", pair.dict.bookname).as_ref());
        for ind in &pair.index {
            println!("{}:   {:?}", index_count, &ind.word);
            index_count += 1;
        }
        println!()
    }
    loop {
        print_green("Enter the number of word you want to see.");
        let mut buffer = String::new();
        io::stdin().read_line(&mut buffer).unwrap();
        if buffer.trim().eq_ignore_ascii_case("z") {
            break;
        } else {
            match usize::from_str_radix(&buffer.trim(), 10) {
                Ok(n) => {
                    if n < index_count + 1 {
                        let mut previous_lenght: usize = 0;
                        for sub_group in pairs {
                            if previous_lenght < n
                                && n - 1 < previous_lenght + sub_group.index.len()
                            {
                                debug!("Found index corresponding to entered number {}, previous length: {}, sub_group.len: {}, n: {}  ", sub_group.dict.bookname, previous_lenght, sub_group.index.len(), n);
                                let index: &Index =
                                    sub_group.index.get(n - previous_lenght - 1).unwrap();
                                sub_group
                                    .dict
                                    .read_definition(index)
                                    .unwrap()
                                    .print_colored();
                            }
                            previous_lenght += sub_group.index.len();
                        }
                    } else {
                        print_green("Enter a valid number or enter z to exit.");
                    }
                }
                Err(_) => print_green("Enter a valid number or enter z to exit."),
            }
        }
    }
}

fn print_defs(pairs: &[IndexDictPair]) {
    for pair in pairs {
        let defs: Vec<Definition> = pair
            .index
            .iter()
            .filter_map(|ind| pair.dict.read_definition(ind).ok())
            .collect();
        print_green(
            format!(
                "From dictionary {} found {} results. \n",
                &pair.dict.bookname,
                &defs.len()
            )
            .as_ref(),
        );
        for d in &defs {
            d.print_colored();
        }
    }
}
