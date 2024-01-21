//! A crate for working with old 8-bit string formats
#![warn(missing_docs)]
#![warn(unsafe_code)]

use std::{fs::read_to_string, sync::RwLock};

// See the notes about optional JSON support in the Cargo.toml file
// #[cfg(feature = "json")]
use serde::{Deserialize, Serialize};
// #[cfg(feature = "json")]
use serde_json::{Map, Value};

pub mod error;
pub mod petscii;

/// An individual system config
/// Contains character set mappings
// #[cfg(feature = "json")]
#[derive(Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    /// Version of this system
    pub version: String,
    /// This table contains the actual mapping from 8-bit characters
    /// to Unicode characters
    pub character_set_map: Map<String, Value>,
}

/// Configuration format
// #[cfg(feature = "json")]
#[derive(Serialize, Deserialize)]
// TODO: system should be dynamic
pub struct Config {
    /// Version of the configuration root
    pub version: String,
    /// A mapping for PETSCII systems
    pub petscii: SystemConfig,
}

/// Embedded config
///
/// Systems can be managed independently and incorporated as long as
/// they're at the same major version level
/// See https://semver.org/ for details
/// Fine-grained versioning of configuration options isn't available
pub static CONFIG_DATA: &str = "
{
    \"version\": \"0.1.0\",
    \"petscii\": {
        \"version\": \"0.1.0\",
	\"character_set_map\": 
	{
	    \"92\": 163,
	    \"94\": 8593,
	    \"95\": 8592
	}
    }
}
";

/// And as a blob
/// This is only a single system's configuration
/// Packing format is pack("<xpypzp", major, minor, patch)
/// where x, y and z are the string length plus one.
/// Followed by a sequence of character code mappings: pack("<BI", src, dst)
/// This means little-endian, with a byte followed by an int
/// For example pack("<2p2p2p", b"0", b"1", b"0")
/// pack("<BI", 92, 163)
///
/// This blob type config is not used anywhere and will probably be
/// deprecated.  But for certain use-cases and business requirements
/// it may be helpful.
pub static CONFIG_DATA_AS_BLOB: &[u8] = &[
    0x01, 0x30, 0x01, 0x31, 0x01, 0x30, 0x5c, 0xa3, 0x00, 0x00, 0x00, 0x5e, 0x91, 0x21, 0x00, 0x00,
    0x5f, 0x90, 0x21, 0x00, 0x00,
];

/// The global configuration settings
/// This is used by default if a custom configuration isn't used
/// when creating a string.
// Each string with configuration is a "reader" on the config data
// structure.  There may be hundreds or thousands floating around.
// Use a reader-writer lock type to keep track of them.  When the lock
// count reaches zero, we can modify the config.
pub static CONFIG: RwLock<Option<Config>> = RwLock::new(None);

/// Load the configuration data from the default configuration string
pub fn load_config() -> std::result::Result<Config, error::Error> {
    let json_str = CONFIG_DATA;

    let json: Config = serde_json::from_str(json_str)?;

    Ok(json)
}

/// Load configuration
pub fn load_config_from_file(filename: &String) -> std::result::Result<Config, error::Error> {
    let json_str = read_to_string(filename)?;

    let json: Config = serde_json::from_str(&json_str)?;

    Ok(json)
}

#[cfg(test)]
mod tests {
    use crate::{load_config, load_config_from_file, Config};
    use std::process::exit;

    #[test]
    fn config_works() {
        let config_result = load_config();
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
        assert!(res.is_none());

        let key: String = 92.to_string();
        let res = config.petscii.character_set_map.get(&key);
        assert_eq!(res.unwrap(), 163);
    }

    #[test]
    fn config_from_file_works() {
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
        let version: String = config.version;
        assert_eq!(version, "0.1.0");

        let key: String = 84.to_string();
        let res = config.petscii.character_set_map.get(&key);
        assert!(res.is_none());

        let key: String = 92.to_string();
        let res = config.petscii.character_set_map.get(&key);
        assert_eq!(res.unwrap(), 163);
    }
}
