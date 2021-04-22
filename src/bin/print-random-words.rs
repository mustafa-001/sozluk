use std::{io, path::PathBuf};
use std::io::Write;

use sozluk::dictionary::Dictionary;
fn main() {
    // let mut dic = Dictionary::new(&PathBuf::from("./dic/wikt-en-en-2018-10-07/wikt-en-en-2018-10-07.ifo"));
    // let mut dic = Dictionary::new(&PathBuf::from("./dic/gts/gts.ifo"));
    let dic = Dictionary::load_dictionary(&PathBuf::from(
        "./dic/stardict-french-english-2.4.2/stardict-french-english-2.4.2.ifo",
    )).unwrap();
    let mut counter = 0;
    
    write!(io::stdout(), "{{ \"french\": [ ").unwrap();
    loop {
        let w = dic.select_random_word();
        if w.word.contains(" ") {
            continue;
        };
        counter += 1;
        write!(io::stdout(), " \"{}\" ,", w.word).unwrap();
        if counter == 50 {
            break;
        }
    }
    println!(" ]}}");
}