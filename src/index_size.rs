use std::mem::size_of;
use sozluk::dictionary::Index;

fn main()  {
    let i1 = Index {
        String::from("asdfas"),
        12,
        13,
    };
    dbg!("Size of Index: ", size_of::<Index>());
}