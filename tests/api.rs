use std::{fs::File, path::PathBuf};

use rawrchive::Archive;

#[test]
fn stored() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/libarchive/v5/stored.rar");
    let file = File::open(path).unwrap();

    let a = Archive::open(file);
    // for entry in a.entries() {
    //     println!("{}", entry.name().unwrap());
    // }
}
