//! Some sample test code to create 8-bit strings and convert them to
//! and from Rust strings.
#![warn(missing_docs)]
#![warn(unsafe_code)]

use forbidden_bands::petscii::{PetsciiConfig, PetsciiString};

fn main() {
    let config_fn = String::from("data/config.json");
    let petscii_config =
        PetsciiConfig::load_from_file(&config_fn).expect("Error loading config file");

    // Test config
    let key: String = 84.to_string();
    let res = petscii_config
        .c64_petscii_unshifted_codes_to_screen_codes
        .get(&key);
    println!("res: {:?}", res);

    let ps =
        PetsciiString::new_with_config(6, [0x41, 0x42, 0x43, 0x5c, 0x5e, 0x5f], &petscii_config);

    println!("debugging PETSCII string: {:?}", ps);
    println!("printing  PETSCII string: {}", ps);

    let s = String::from(ps);

    println!("PETSCII string as String string: {:?}", s);

    let ps_shifted = PetsciiString::new_with_config(3, [0x41, 0x42, 0x43], &petscii_config);

    println!("debugging PETSCII string: {:?}", ps_shifted);
    println!("printing  PETSCII string: {}", ps_shifted);

    let s = String::from(ps_shifted);

    println!("PETSCII string as String string: {:?}", s);

    // Use a character not mapped yet
    let ps2 = PetsciiString::new(3, [0x41, 0x42, 0xb2]);

    println!(
        "debugging PETSCII string with unmapped character: {:?}",
        ps2
    );
    println!("printing  PETSCII string with unmapped character: {}", ps2);

    let s = String::from(ps2);

    println!(
        "PETSCII string as String string with unmapped character: {:?}",
        s
    );
}
