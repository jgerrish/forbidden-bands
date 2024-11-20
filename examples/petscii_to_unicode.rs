//! Convert a sequence of PETSCII bytes to Unicode
//! cargo run --example petscii_to_unicode
//! Type some PETSCII and press CTRL-D
//! or pipe PETSCII to it:
//! echo -n -e "\x0eABCD\x8e" | cargo run --example petscii_to_unicode
#![warn(missing_docs)]
#![warn(unsafe_code)]

use std::{
    io::{stdin, Read},
    process::exit,
    sync::RwLock,
};

use forbidden_bands::{
    petscii::{PetsciiConfig, PetsciiString},
    Config, Configuration,
};

/// The forbidden-bands configuration for the image-rider application
pub static CONFIG: RwLock<Option<forbidden_bands::Config>> = RwLock::new(None);

/// Convert a PETSCII byte sequence to Unicode
fn main() {
    let config_result = PetsciiConfig::load();
    let config: Config = match config_result {
        Ok(c) => c,
        Err(e) => {
            println!("Error loading config: {:?}", e);
            exit(-1);
        }
    };

    let mut stdin = stdin();
    let mut input: Vec<u8> = Vec::new();

    let bytes_read = stdin.read_to_end(&mut input).expect("Couldn't read input");

    println!("Bytes read: {bytes_read}");
    if bytes_read > 256 {
        panic!("Can't read in more than 256 bytes, {bytes_read} read in");
    }

    // I've been holding off on accepting slices and
    // variable-length PETSCII strings.  My use case doesn't need it,
    // but others might want it.
    let ps = PetsciiString::<256>::from_byte_slice_strip_shifted_space_with_config(
        input.as_slice(),
        &config.petscii,
    );

    let s: String = ps.into();
    println!("{}", s);
}
