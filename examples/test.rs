//! Some sample test code to create 8-bit strings and convert them to
//! and from Rust strings.
#![warn(missing_docs)]
#![warn(unsafe_code)]

use std::process::exit;

use forbidden_bands::{load_config_from_file, petscii::PetsciiString, Config, CONFIG};

fn main() {
    let config_fn = String::from("data/config.json");
    let config_result = load_config_from_file(&config_fn);
    let config: Config = match config_result {
        Ok(c) => c,
        Err(e) => {
            println!("Error loading config: {:?}", e);
            exit(-1);
        }
    };

    // Test config
    let key: String = 84.to_string();
    let res = config.petscii.character_set_map.get(&key);
    println!("res: {:?}", res);

    let ps =
        PetsciiString::new_with_config(6, [0x41, 0x42, 0x43, 0x5c, 0x5e, 0x5f], &config.petscii);

    println!("debugging PETSCII string: {:?}", ps);
    println!("printing  PETSCII string: {}", ps);

    let s = String::try_from(ps);

    println!("PETSCII string as String string: {:?}", s);

    // This should be called at a higher level than when creating
    // strings usually.  Possibly only once at library
    // initialization.
    {
        let mut lock_res = CONFIG
            .write()
            .expect("Should be able to acquire config lock");
        *lock_res = Some(config);
    }

    let binding = CONFIG.read().expect("Should be able to get reader lock");
    let config = binding.as_ref().unwrap();

    let ps_shifted = PetsciiString::new_with_config(3, [0x41, 0x42, 0x43], &config.petscii);

    println!("debugging PETSCII string: {:?}", ps_shifted);
    println!("printing  PETSCII string: {}", ps_shifted);

    let s = String::try_from(ps_shifted);

    println!("PETSCII string as String string: {:?}", s);

    // Use a character not mapped yet
    let ps2 = PetsciiString::new(3, [0x41, 0x42, 0xb2]);

    println!(
        "debugging PETSCII string with unmapped character: {:?}",
        ps2
    );
    println!("printing  PETSCII string with unmapped character: {}", ps2);

    let s = String::try_from(ps2);

    println!(
        "PETSCII string as String string with unmapped character: {:?}",
        s
    );
}
