//! A crate for working with old 8-bit string formats
#![warn(missing_docs)]
#![warn(unsafe_code)]

use std::{collections::HashMap, fs::File, io::BufReader, path::Path, sync::RwLock};

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
#[derive(Clone, Debug, Serialize, Deserialize)]
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

    /// This is a new config model.  Slurp in the character_set_maps as
    /// generic serde_json Value types.  Then in each system crate
    /// (petscii.rs, etc.) parse it into a custom type.
    pub character_set_map: serde_json::Value,
}

/// Configuration format
// #[cfg(feature = "json")]
#[derive(Clone, Debug, Serialize, Deserialize)]
// TODO: system should be dynamic
pub struct Config {
    /// Version of the configuration root
    pub version: String,
    /// Map of systems.  This replaced the individual fields for
    /// systems like PETSCII.
    ///
    /// Individual modules are now more loosely coupled.
    pub systems: HashMap<String, SystemConfig>,
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
    // The config system is getting better, including caching.  But we
    // still do a lot of heavy cloning of a data structure that is
    // likely to increase in size.
    //
    // One possible future improvement is a load function that let's
    // you pass in a closure, so we don't have to clone the data,
    // possibly only cloning individual systems, or not even that.
    fn load() -> std::result::Result<Config, error::Error> {
        // First see if the configuration is already loaded
        {
            let binding = CONFIG.read().expect("Should be able to get reader lock");

            let test = binding.as_ref();
            if let Some(config) = test {
                // Config has been loaded
                return Ok(config.clone());
            }
        }

        // Config has not been loaded yet
        let json_str = config_data::CONFIG_DATA;
        let config: Config = serde_json::from_str(json_str)?;
        {
            let mut lock_res = crate::CONFIG
                .write()
                .expect("Should be able to acquire config lock");
            *lock_res = Some(config.clone());
        }

        Ok(config)
    }

    fn load_from_file(filename: &str) -> std::result::Result<Config, error::Error> {
        // First see if the configuration is already loaded
        {
            let binding = CONFIG.read().expect("Should be able to get reader lock");

            let test = binding.as_ref();
            if let Some(config) = test {
                // Config has been loaded
                return Ok(config.clone());
            }
        }

        // Config has not been loaded yet
        // read_to_string is inefficient see [``std::io::BufReader``]
        let path = Path::new(filename);
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let config: Config = serde_json::from_reader(reader)?;
        {
            let mut lock_res = crate::CONFIG
                .write()
                .expect("Should be able to acquire config lock");
            *lock_res = Some(config.clone());
        }

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
        // These tests may be too dependent on the petscii module
        // But I'll keep them in here for now
        let key: String = 167.to_string();

        let char_set_map_val = &config
            .systems
            .get("petscii")
            .as_ref()
            .unwrap()
            .character_set_map;

        let char_set_map: crate::petscii::PetsciiConfig =
            serde_json::from_value(char_set_map_val.clone()).unwrap();

        let res: Option<&serde_json::Value> = char_set_map
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
        let res = char_set_map
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
        assert_eq!(version, "0.3.0");

        // let key: String = 84.to_string();
        // let res = config.petscii.character_set_map.get(&key);
        // assert!(res.is_none());

        // let key: String = 92.to_string();
        // let res = config.petscii.character_set_map.get(&key);
        // assert_eq!(res.unwrap(), 163);
    }
}
