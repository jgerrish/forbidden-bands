//! Print out "Hello, world!" in a PETSCII block graphics border
#![warn(missing_docs)]
#![warn(unsafe_code)]

use forbidden_bands::{petscii::PetsciiString, Config, Configuration};

fn main() {
    let config_fn = String::from("data/config.json");
    let config = Config::load_from_file(&config_fn).expect("Error loading config file");

    let hello_world_data: [u8; 61] = [
        0x0d, 0x0a, 0xb0, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60,
        0x60, 0x60, 0x60, 0xae, 0x0d, 0x0a, 0x7d, 0x20, 0x48, 0x0e, 0x45, 0x4c, 0x4c, 0x4f, 0x2c,
        0x20, 0x57, 0x4f, 0x52, 0x4c, 0x44, 0x21, 0x20, 0x8e, 0x7d, 0x0d, 0x0a, 0xad, 0x60, 0x60,
        0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0xbd, 0x0d,
        0x0a,
    ];

    let ps = PetsciiString::new_with_config(61, hello_world_data, &config.petscii);

    println!("debugging PETSCII string: {:?}", ps);

    println!("printing PETSCII string: {}", ps);

    let s = String::from(ps);

    println!("PETSCII string as String string: {}", s);
}