//! Convert a sequence of Unicode characters to PETSCII bytes
//! cargo run --example petscii_to_unicode
//! Type some Unicode and press CTRL-D
//! or pipe Unicode to it:
//! echo -n -e "abcd" | cargo run --example unicode_to_petscii
#![warn(missing_docs)]
#![warn(unsafe_code)]

use std::{
    io::{stdin, Read},
    sync::RwLock,
};

use forbidden_bands::{
    petscii::{PetsciiConfig, PetsciiString},
    Configuration,
};

/// The forbidden-bands configuration for the image-rider application
pub static CONFIG: RwLock<Option<forbidden_bands::Config>> = RwLock::new(None);

/// Convert a Unicode character sequence to a PETSCII byte sequence
fn main() {
    let config = PetsciiConfig::load().expect("Error loading config");

    let mut stdin = stdin();
    let mut input = String::new();

    let bytes_read = stdin
        .read_to_string(&mut input)
        .expect("Couldn't read input");

    if bytes_read > 256 {
        panic!("Can't read in more than 256 bytes, {bytes_read} read in");
    }

    let ps = PetsciiString::<256>::from_str_with_config(input.as_str(), &config.petscii);

    let s: String = ps.into();
    println!("{}", s);
}
