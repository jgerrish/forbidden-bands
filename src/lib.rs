//! A crate for working with old 8-bit string formats
#![warn(missing_docs)]
#![warn(unsafe_code)]

use std::{fs::File, io::BufReader, path::Path, sync::RwLock};

// See the notes about optional JSON support in the Cargo.toml file
// #[cfg(feature = "json")]
use serde::{Deserialize, Serialize};
// #[cfg(feature = "json")]
// use serde_json::{Map, Value};

pub mod config_data;
pub mod error;
pub mod petscii;

/// An individual system config
/// Contains character set mappings
// #[cfg(feature = "json")]
#[derive(Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    /// Version of this system
    pub version: String,

    /// character_set_map contains the actual mapping from 8-bit characters
    /// to Unicode characters and vice-versa
    ///
    /// Some "legacy computing" forbidden band crates like the
    /// Commodore (CBM) PETSCII crate also have intermediate maps and
    /// tables.  In the case of CBM there is a set of "screen code"
    /// tables that hold information about the in-memory values of
    /// characters on the screen.
    ///
    /// TODO: I want to get dynamic loading and unloading of modules
    /// working.  It will require some refactoring with dyn traits and
    /// serialization / deserialization to make sure everything works.
    pub character_set_map: petscii::PetsciiConfig,
}

/// Configuration format
// #[cfg(feature = "json")]
#[derive(Serialize, Deserialize)]
// TODO: system should be dynamic
pub struct Config {
    /// Version of the configuration root
    pub version: String,
    /// A mapping for PETSCII systems
    /// TODO: Remove this, individual modules should create their own
    /// keys, in an approved namespace like good little modules.
    pub petscii: SystemConfig,
}

/// The global configuration settings
/// This is used by default if a custom configuration isn't used
/// when creating a string.
// Each string with configuration is a "reader" on the config data
// structure.  There may be hundreds or thousands floating around.
// Use a reader-writer lock type to keep track of them.  When the lock
// count reaches zero, we can modify the config.
pub static CONFIG: RwLock<Option<Config>> = RwLock::new(None);

/// Trait that defines a set of methods that allow loading and
/// unloading configuration data
pub trait Configuration {
    /// Load the configuration data from the default configuration
    /// string
    fn load() -> std::result::Result<Config, error::Error>;

    /// Load configuration from a file
    fn load_from_file(filename: &str) -> std::result::Result<Config, error::Error>;
}

impl Configuration for Config {
    fn load() -> std::result::Result<Config, error::Error> {
        let json_str = config_data::CONFIG_DATA;

        let config: Config = serde_json::from_str(json_str)?;

        Ok(config)
    }

    fn load_from_file(filename: &str) -> std::result::Result<Config, error::Error> {
        // read_to_string is inefficient see [``std::io::BufReader``]
        let path = Path::new(filename);
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let config: Config = serde_json::from_reader(reader)?;

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Config, Configuration};

    #[test]
    fn config_works() {
        let config = Config::load().expect("Error loading config");

        // Test config
        let key: String = 167.to_string();
        let res: Option<&serde_json::Value> = config
            .petscii
            .character_set_map
            .c64_petscii_unshifted_codes_to_screen_codes
            .get(&key);
        match res.unwrap() {
            serde_json::Value::Array(v) => {
                assert_eq!(v.first().unwrap().as_u64().unwrap(), 1);
                assert_eq!(v.get(1).unwrap().as_u64().unwrap(), 103);
            }
            _ => {
                assert!(false);
            }
        }

        let key: String = 103.to_string();
        let res = config
            .petscii
            .character_set_map
            .c64_screen_codes_set_1_to_unicode_codes
            .get(&key);
        assert!(res.is_none());

        // let key: String = 92.to_string();
        // let res = config.petscii.character_set_map.get(&key);
        // assert_eq!(res.unwrap(), 163);
    }

    #[test]
    fn config_from_file_works() {
        let config_fn = String::from("data/config.json");
        let config = Config::load_from_file(&config_fn).expect("error loading config file");

        // Test config
        let version: String = config.version;
        assert_eq!(version, "0.2.0");

        // let key: String = 84.to_string();
        // let res = config.petscii.character_set_map.get(&key);
        // assert!(res.is_none());

        // let key: String = 92.to_string();
        // let res = config.petscii.character_set_map.get(&key);
        // assert_eq!(res.unwrap(), 163);
    }
}
