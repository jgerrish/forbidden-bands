//!
//! PETSCII string library
//!
//! PETSCII is a character set used in Commodore Business Machines'
//! 8-bit computers.  It's based on the 1963 version of ASCII, not the
//! 1967 version.  In addition, it has some custom block graphics
//! characters, geometric shapes and playing card suits.
//!
//! There are actually two PETSCII character sets, an "unshifted" one
//! and a "shifted" one.  Confusingly, the unshifted one has uppercase
//! characters and most of the graphics characters while the shifted
//! one has lowercase characters and uppercase characters.
//!
//! In addition to the PETSCII character sets, the Commodore has an
//! in-memory screen display code format that is different than their
//! character values.  There are also two sets of screen display
//! codes, one containing mostly uppercase characters and graphics
//! symbols (Set 1) and one containing lowercase characters and
//! uppercase characters (Set 2).
//!
//! These tables are partially outlined in the Commodore 64
//! Programmer's Reference Guide under Appendix B (Screen Display
//! Codes) and Appendix C (ASCII and CHR$ Codes)
//!
//! Unicode mappings
//!
//! A couple standards provide mappings between Commodore graphics
//! characters and Unicode:
//!
//! Symbols for Legacy Computing: Unicode Standard 16.0 Section 22.7.4
//!
//! Legacy Computing Sources: 18235-aux-LegacyComputingSources.pdf
//! Provides actual code maps for Unicode symbols to Commodore and
//! other legacy computers graphic characters.
//!
//! The Legacy Computing Standards auxillary supplement to Unicode
//! uses Commodore screen codes to specify the PET/VIC20 and C64/C128
//! characters.  Set 1 is specified with G0 in parentheses after the
//! screen code and Set 2 is specified with G1 in parentheses.
//!
//! Because there are two sets of screen codes and two sets of PETSCII
//! codes, converting between PETSCII characters and Unicode
//! characters isn't a simple single table lookup.
#![warn(missing_docs)]
#![warn(unsafe_code)]

use enumset::{EnumSet, EnumSetType};
use std::{
    fmt::{Debug, Display, Formatter, Result},
    sync::RwLock,
};

// See the notes about optional JSON support in the Cargo.toml file
// #[cfg(feature = "json")]
use serde::{Deserialize, Serialize};
// #[cfg(feature = "json")]
use serde_json::{Map, Value};

use crate::{config_data, Configuration, SystemConfig};

/// A Commodore screen code value and the screen set it is in
///
/// The configuration file uses a two-element tuple or list to store
/// the set and value fields.  The Serde and Serde JSON serializer
/// automatically support deserializing from a tuple into a struct.
/// This may be confusing so this note is here to let people know.
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub struct ScreenCodeValue {
    /// The screen set this code is in
    pub set: u8,
    /// The screen code value
    pub value: u8,
}

/// Commodore 64 character attributes
#[derive(Debug, EnumSetType, Serialize, Deserialize)]
#[enumset(serialize_repr = "u8", repr = "u8")]
pub enum PetsciiCharacterAttributes {
    /// A shifted character
    Shifted,
}

/// The Petscii Code along with whether it's the "shifted" table
/// The unshifted table contains uppercase and graphics characters
/// The shifted table contains lowercase and uppercase characters.
#[derive(Clone, Serialize, Deserialize)]
pub struct PetsciiCodeValue {
    /// Whether the value is shifted and other attributes
    pub attributes: u8,
    /// The PETSCII code value
    pub value: u8,
}

/// Configuration data including character maps for the PETSCII crate
// #[cfg(feature = "json")]
#[derive(Clone, Serialize, Deserialize)]
pub struct PetsciiConfig {
    /// Version of the PETSCII config
    pub version: String,

    /// shifted PETSCII codes to screen codes
    pub c64_petscii_shifted_codes_to_screen_codes: Map<String, Value>,

    /// unshifted PETSCII codes to screen codes
    pub c64_petscii_unshifted_codes_to_screen_codes: Map<String, Value>,

    /// C64 screen codes set 1 to Unicode codes
    pub c64_screen_codes_set_1_to_unicode_codes: Map<String, Value>,
    /// C64 screen codes set 2 to Unicode codes
    pub c64_screen_codes_set_2_to_unicode_codes: Map<String, Value>,

    /// C64 screen codes set 3 (virtual table) to Unicode codes
    pub c64_screen_codes_set_3_to_unicode_codes: Map<String, Value>,

    // Maps from Unicode to PETSCII
    /// Map from Unicode codes to C64 screen codes
    pub unicode_codes_to_c64_screen_codes: Map<String, Value>,

    /// Maps from C64 screen codes set 1 to to PETSCII codes
    pub c64_screen_codes_set_1_to_petscii_codes: Map<String, Value>,
    /// Maps from C64 screen codes set 2 to to PETSCII codes
    pub c64_screen_codes_set_2_to_petscii_codes: Map<String, Value>,

    /// Maps from C64 screen codes set 3 to to PETSCII codes Screen
    /// Code Set 3 is a "virtual" screen code set that doesn't exist
    /// on the actual C64.  It exists here to represent intermediate
    /// control values line line feed and carriage return.
    ///
    /// Trains are hats
    pub c64_screen_codes_set_3_to_petscii_codes: Map<String, Value>,
}

/// Configuration data for the PETSCII crate
///
/// We try to load this once on first use and then only read from it
/// There is an overhead creating each PetsciiString getting a read
/// lock on the config variable.
pub static CONFIG: RwLock<Option<PetsciiConfig>> = RwLock::new(None);

/// Load the configuration data from the PETSCII configuration string
impl Configuration for PetsciiConfig {
    fn load() -> std::result::Result<crate::Config, crate::error::Error> {
        let crate_config = crate::Config::load()?;

        // First see if the configuration is already loaded
        {
            let binding = CONFIG.read().expect("Should be able to get reader lock");

            let test = binding.as_ref();
            // This pattern has a code smell
            // I don't have a good RAII replacement for it.
            // I'm rust.try_once_into_and_or_expect_better_ergonomics_from_compiler_not_speed(|e| { yoda_is_in_lispland(e) });
            if test.is_some() {
                let petscii_config = test.expect("Should be set at this point");

                return Ok(crate::Config {
                    version: crate_config.version,
                    petscii: crate::SystemConfig {
                        version: crate_config.petscii.version,
                        character_set_map: petscii_config.clone(),
                    },
                });
            }
        }

        // If the configuration is not loaded, load it and save it
        let json_str = config_data::C64_PETSCII_MAP;
        let petscii_config: PetsciiConfig =
            serde_json::from_str(json_str).expect("Couldn't load embedded config");

        {
            let mut lock_res = CONFIG
                .write()
                .expect("Should be able to acquire config lock");
            *lock_res = Some(petscii_config.clone());
        }

        Ok(crate::Config {
            version: crate_config.version,
            petscii: crate::SystemConfig {
                version: crate_config.petscii.version,
                character_set_map: petscii_config.clone(),
            },
        })
    }

    fn load_from_file(filename: &str) -> std::result::Result<crate::Config, crate::error::Error> {
        // let path = Path::new(filename);
        // let file = File::open(path)?;
        // let reader = BufReader::new(file);

        // This assumes the root crate knows about this crates config
        // This is a bad design, and should be fixed in future versions
        let crate_config = crate::Config::load_from_file(filename)?;

        // let json: Config = serde_json::from_reader(reader)?;

        Ok(crate_config)
    }
}

/// Commodore 64 character attributes
#[derive(Debug, EnumSetType)]
pub enum CharacterAttributes {
    /// A normal character
    Normal = 0,
    /// A reversed-video character
    Reversed = 1,
}

/// A PETSCII character has a set of associated attributes (normal, reversed, etc.)
/// and PETSCII code
pub struct PetsciiCharacter {
    /// The attributes of this character
    pub attributes: CharacterAttributes,
    /// The character PETSCII code
    pub character: u8,
}

/// A PETSCII string
///
/// A fixed-length PETSCII string
///
/// Later versions may support variable-length strings.  This library
/// was created to help debug C64 file systems, which use fixed-length
/// strings for some of the data structures.
#[derive(Clone, Copy)]
pub struct PetsciiString<'a, const L: usize> {
    /// The length of the string
    pub len: u32,
    /// The string data
    pub data: [u8; L],

    /// The character map for this string
    pub character_map: Option<&'a SystemConfig>,

    /// strip "shifted space" (0xA0) characters in the display of this
    /// PetsciiString.
    /// CBM DOS uses shifted space characters to pad file names and
    /// disk names.
    pub strip_shifted_space: bool,
}

impl<'a, const L: usize> Debug for PetsciiString<'a, L> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "length: {:?}, ", self.len)?;
        write!(f, "data: {:?}, ", self.data)?;
        write!(f, "display: {}", self)
    }
}

impl<'a, const L: usize> Display for PetsciiString<'a, L> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", String::from(self))
    }
}

/// An IntoIter structure for PetsciiStrings
/// We need to keep track of the index of the current element, along
/// with the data.
pub struct IntoIter<'a, const L: usize> {
    index: usize,
    data: PetsciiString<'a, L>,
}

impl<'a, const L: usize> IntoIterator for PetsciiString<'a, L> {
    type Item = u8;
    type IntoIter = IntoIter<'a, L>;
    fn into_iter(self) -> IntoIter<'a, L> {
        IntoIter {
            index: 0,
            data: self,
        }
    }
}

impl<'a, const L: usize> Iterator for IntoIter<'a, L> {
    type Item = u8;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.data.len.try_into().unwrap() {
            self.index += 1;
            Some(self.data.data[self.index - 1])
        } else {
            None
        }
    }
}

impl<'a, const L: usize> From<&'a [u8]> for PetsciiString<'a, L> {
    fn from(s: &'a [u8]) -> PetsciiString<L> {
        let mut bytes: [u8; L] = [0; L];
        if s.len() > L {
            panic!("u8 slice is too large");
        }

        // Replacing the below manual copy loop between slices with
        // the following recomendation from clippy
        // for i in 0..L {
        //     bytes[i] = s[i];
        // }
        bytes[..L].copy_from_slice(&s[..L]);

        PetsciiString {
            len: L as u32,
            data: bytes,
            character_map: None,
            strip_shifted_space: false,
        }
    }
}

/// Convert a Unicode string slice to a vector of PETSCII bytes
///
/// This current code handles shifted and unshifted PETSCII characters.
/// It assumes the default character set is unshifted and will return
/// to that state at the end of every string.
///
/// So for example, if a string consists of uppercase characters followed
/// by lowercase: ABCabc, it will output:
/// 0x41, 0x42, 0x43, 0x0e, 0x41, 0x42, 0x43, 0x8e
///
/// NOT the following leaving the next possible concatenated string in
/// a shifted state
///
/// 0x41, 0x42, 0x43, 0x0e, 0x41, 0x42, 0x43
///
/// If there are other common uses cases, this could be made a
/// parameter or the default changed.
fn unicode_to_petscii_bytes(s: &str) -> Vec<u8> {
    let mut attributes = EnumSet::new();
    let mut shifted = false;

    let config = PetsciiConfig::load().expect("Error loading config");

    let uc_map = config
        .petscii
        .character_set_map
        .unicode_codes_to_c64_screen_codes;
    let sc1_map = config
        .petscii
        .character_set_map
        .c64_screen_codes_set_1_to_petscii_codes;
    let sc2_map = config
        .petscii
        .character_set_map
        .c64_screen_codes_set_2_to_petscii_codes;
    let sc3_map = config
        .petscii
        .character_set_map
        .c64_screen_codes_set_3_to_petscii_codes;

    attributes.insert(CharacterAttributes::Normal);

    let mut bytes: Vec<u8> = s
        .chars()
        .filter_map(|c| {
            let key = u32::from(c).to_string();

            let screen_code_opt = uc_map.get(&key);

            let screen_code_value = match screen_code_opt {
                Some(s) => s,
                None => {
                    return None;
                }
            };

            let screen_code_res = ScreenCodeValue::deserialize(screen_code_value);
            let screen_code = match screen_code_res {
                Ok(s) => s,
                Err(_) => {
                    return None;
                }
            };

            let key = screen_code.value.to_string();
            let petscii_code_opt = if screen_code.set == 1 {
                sc1_map.get(&key)
            } else if screen_code.set == 2 {
                sc2_map.get(&key)
            } else if screen_code.set == 3 {
                // Screen code set 3 is a "virtual" screen code set
                // It's used to transform control characters like line feed
                // and carriage return
                sc3_map.get(&key)
            } else {
                return None;
            };
            let petscii_code_value = match petscii_code_opt {
                Some(s) => s,
                None => {
                    return None;
                }
            };

            let petscii_code_res = PetsciiCodeValue::deserialize(petscii_code_value);
            let petscii_code = match petscii_code_res {
                Ok(s) => s,
                Err(_) => {
                    return None;
                }
            };

            Some(petscii_code)
        })
        .flat_map(|petscii_code| {
            let mut codes: Vec<u8> = Vec::new();
            let eset: EnumSet<PetsciiCharacterAttributes> =
                EnumSet::from_repr(petscii_code.attributes);

            if eset.contains(PetsciiCharacterAttributes::Shifted) {
                if !shifted {
                    // Output a new shift in character
                    codes.push(0x0E);
                    shifted = true;
                }
            } else if shifted {
                // Output a new shift out character
                codes.push(0x8E);
                shifted = false;
            }
            codes.push(petscii_code.value);
            codes
        })
        .collect();

    // Shift out if we're still shifted at the end of a string
    if shifted {
        bytes.push(0x8E);
    }

    bytes
}

impl<'a, const L: usize> From<&str> for PetsciiString<'a, L> {
    fn from(s: &str) -> PetsciiString<'a, L> {
        let mut final_bytes: [u8; L] = [0; L];

        let bytes = unicode_to_petscii_bytes(s);

        if bytes.len() > L {
            panic!("u8 slice is too large");
        }
        let b = bytes.as_slice();

        final_bytes[..b.len()].copy_from_slice(&b[..b.len()]);

        PetsciiString {
            len: b.len() as u32,
            data: final_bytes,
            character_map: None,
            strip_shifted_space: false,
        }
    }
}

impl<'a, const L: usize> From<PetsciiString<'a, L>> for String {
    /// Create a String from a PetsciiString
    ///
    /// # Examples
    ///
    /// ```
    /// use forbidden_bands::{
    ///     petscii::{PetsciiConfig, PetsciiString},
    ///     Config,
    ///     Configuration,
    /// };
    ///
    /// let config = PetsciiConfig::load().expect("Error loading config file");
    ///
    /// let ps = PetsciiString::new_with_config(6, [0x41, 0x42, 0x43, 0x5c, 0x5e, 0x5f], &config.petscii);
    /// let mut s: String = String::from(ps);
    ///
    /// assert_eq!(s.pop().unwrap(), '←');
    /// assert_eq!(s.pop().unwrap(), '↑');
    /// assert_eq!(s.pop().unwrap(), '£');
    /// assert_eq!(s.pop().unwrap(), 'C');
    /// assert_eq!(s.pop().unwrap(), 'B');
    /// assert_eq!(s.pop().unwrap(), 'A');
    /// ```
    fn from(s: PetsciiString<L>) -> String {
        String::from(&s)
    }
}

impl<'a, const L: usize> From<&PetsciiString<'a, L>> for String {
    /// Create a String from a reference to a PetsciiString
    ///
    /// # Examples
    ///
    /// ```
    /// use forbidden_bands::{
    ///     petscii::{PetsciiConfig, PetsciiString},
    ///     Config,
    ///     Configuration,
    /// };
    ///
    /// let config = PetsciiConfig::load().expect("Error loading config file");
    ///
    /// let ps = PetsciiString::new_with_config(6, [0x41, 0x42, 0x43, 0x5c, 0x5e, 0x5f], &config.petscii);
    /// let mut s: String = String::from(&ps);
    ///
    /// assert_eq!(s.pop().unwrap(), '←');
    /// assert_eq!(s.pop().unwrap(), '↑');
    /// assert_eq!(s.pop().unwrap(), '£');
    /// assert_eq!(s.pop().unwrap(), 'C');
    /// assert_eq!(s.pop().unwrap(), 'B');
    /// assert_eq!(s.pop().unwrap(), 'A');
    /// ```
    // TODO: Unicode 13 now has "Legacy Computing Sources"
    // (Unicode 13 was released around March 10, 2020).
    fn from(s: &PetsciiString<L>) -> String {
        let mut attributes = EnumSet::new();
        let mut shifted = false;

        attributes.insert(CharacterAttributes::Normal);
        s.into_iter()
            .filter(|c| !s.strip_shifted_space || (*c != 0xA0))
            .filter_map(|c| {
		// TODO: refactor this into another function.
		//
		// It's a good opportunity to learn State patterns and
		// integrate that into this code.
		match c {
		    0x0E => {
			// Switch to lowercase / shifted
			// This is the "shifted" state on the C64
			// Unshifted is an uppercase and graphic
			// character set
			shifted = true;
			return None;
		    },
		    0x12 => {
			attributes.remove(CharacterAttributes::Normal);
			attributes.insert(CharacterAttributes::Reversed);
			return None;
		    },
		    0x8E => {
			// Switch to uppercase / unshifted
			// This is the "unshifted" state on the C64
			// shifted is a lowercase and uppercase
			// character set (business mode)
			shifted = false;
			return None;
		    },
		    0x92 => {
			attributes.remove(CharacterAttributes::Reversed);
			attributes.insert(CharacterAttributes::Normal);
			return None;
		    },
		    _ => {}
		}

		let cm = match &s.character_map {
		    Some(s) => s,
		    None => { return Some(char::from_u32(c as u32).unwrap()); },
		};

		// There are three sets of code that are duplicated in
		// PETSCII
		// They're duplicated in both the PETSCII unshifted
		// and shifted character sets.
		//
		// 192-223 are duplicates of 96-127
		// 224-254 are duplicates of 160-190
		// 255 is a duplicate of 126
		//
		// These should probably be explicity added to the
		// configuration data instead of transformed here.
		let c = match c {
		    0..=191 => c,
		    192..=223 => c - 96,
		    224..=254 => c - 64,
		    255 => 126,
		};

		// Map from PETSCII to screen codes
		let petscii_to_screen_codes = if !shifted {
		    &cm.character_set_map.c64_petscii_unshifted_codes_to_screen_codes
		} else {
		    &cm.character_set_map.c64_petscii_shifted_codes_to_screen_codes
		};
		let key = c.to_string();

		let screen_code_opt: Option<ScreenCodeValue> =
		    petscii_to_screen_codes
		    .get(&key)
		    .and_then(|screen_code_value| {
			ScreenCodeValue::deserialize(screen_code_value).ok()
		    });

		// This chaining of None options is tricky.  return
		// None doesn't always return to the filter_map
		// context in an closure context, but it does in a
		// match context
		let screen_code = match screen_code_opt {
		    Some(s) => s,
		    None => return None,
		};

		// TODO This test may be removed as we implement the full
		// block character graphics set
		if screen_code.value > 127 {
		    panic!("Should not have a screen code greater than 127 before applying reverse video transform");
		}

		let screen_code_value: u32 =
		    if attributes.contains(CharacterAttributes::Reversed) {
			(screen_code.value as u32) + 128
		    } else {
			screen_code.value.into()
		    };

		// Now map from screen codes to Unicode
		let screen_codes_to_unicode = match screen_code.set {
		    1 =>
			&cm.character_set_map.c64_screen_codes_set_1_to_unicode_codes,
		    2 =>
			&cm.character_set_map.c64_screen_codes_set_2_to_unicode_codes,
		    3 =>
			&cm.character_set_map.c64_screen_codes_set_3_to_unicode_codes,
		    _ => {
			panic!("Invalid screen code set");
		    }
		};

		let key = screen_code_value.to_string();
                let d = if screen_codes_to_unicode.contains_key(&key) {
                    match screen_codes_to_unicode.get(&key).unwrap() {
                        serde_json::Value::Number(v) => v.as_u64().unwrap() as u32,
                        _ => 0,
                    }
                } else {
                    c as u32
                };

                Some(char::from_u32(d).unwrap())
            })
            .collect()
    }
}

impl<'a, const L: usize> PetsciiString<'a, L> {
    /// Create a new Petscii string
    ///
    /// # Examples
    ///
    /// ```
    /// use forbidden_bands::petscii::PetsciiString;
    ///
    /// let ps = PetsciiString::new(6, [0x41, 0x42, 0x43]);
    ///
    /// assert_eq!(ps.data[0], 0x41);
    /// assert_eq!(ps.data[1], 0x42);
    /// assert_eq!(ps.data[2], 0x43);
    /// ```
    pub fn new(len: u32, data: [u8; L]) -> Self {
        PetsciiString {
            len,
            data,
            character_map: None,
            strip_shifted_space: false,
        }
    }

    /// Create a new PETSCII string with a given character map
    ///
    /// # Examples
    ///
    /// ```
    /// use forbidden_bands::{
    ///     petscii::{PetsciiConfig, PetsciiString},
    ///     Config,
    ///     Configuration,
    /// };
    ///
    /// let config = PetsciiConfig::load().expect("Error loading config");
    /// let ps = PetsciiString::new(6, [0x41, 0x42, 0x43]);
    ///
    /// assert_eq!(ps.data[0], 0x41);
    /// assert_eq!(ps.data[1], 0x42);
    /// assert_eq!(ps.data[2], 0x43);
    /// ```
    pub fn new_with_config(len: u32, data: [u8; L], character_map: &'a SystemConfig) -> Self {
        PetsciiString {
            len,
            data,
            character_map: Some(character_map),
            strip_shifted_space: false,
        }
    }

    /// Get the length of the Petscii string
    ///
    /// TODO: More details on what length means
    /// TODO: Add example about getting number of characters
    ///
    /// # Examples
    ///
    /// ```
    /// use forbidden_bands::petscii::PetsciiString;
    ///
    /// let ps = PetsciiString::new(3, [0x41, 0x42, 0x43]);
    ///
    /// assert_eq!(ps.len(), 3);
    /// ```
    pub fn len(&self) -> usize {
        self.len as usize
    }

    /// Return true if the string is empty
    /// # Examples
    ///
    /// ```
    /// use forbidden_bands::petscii::PetsciiString;
    ///
    /// let ps = PetsciiString::new(0, []);
    ///
    /// assert!(ps.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// This function is the same as the From implementation for byte
    /// slices but it strips any shifted spaces (0xA0) from the end.
    ///
    /// Shifted spaces are used to pad out filenames and disk namss in
    /// CBM DOS
    pub fn from_byte_slice_strip_shifted_space(s: &'a [u8]) -> PetsciiString<L> {
        let mut bytes: [u8; L] = [0; L];
        if s.len() > L {
            panic!("u8 slice is too large");
        }

        // Replacing the below manual copy loop between slices with
        // the following recomendation from clippy
        // for i in 0..s.len() {
        //     bytes[i] = s[i];
        // }
        bytes[..s.len()].copy_from_slice(s);

        PetsciiString {
            len: L as u32,
            data: bytes,
            character_map: None,
            strip_shifted_space: true,
        }
    }

    /// Create a PetsciiString from a string slice
    ///
    /// I think I'm going to have to decide on what to do about
    /// configs.. boxes or arcs or passing around the RwLock or
    /// whatever
    ///
    /// TODO: Figure this out and remove this function and the
    /// with_config functions
    pub fn from_str_with_config(s: &str, character_map: &'a SystemConfig) -> PetsciiString<'a, L> {
        let mut final_bytes: [u8; L] = [0; L];

        let bytes = unicode_to_petscii_bytes(s);

        if bytes.len() > L {
            panic!("u8 vector is too large");
        }
        let b = bytes.as_slice();

        final_bytes[..b.len()].copy_from_slice(&b[..b.len()]);

        PetsciiString {
            len: b.len() as u32,
            data: final_bytes,
            character_map: Some(character_map),
            strip_shifted_space: false,
        }
    }

    /// Create a PetsciiString from a byte slice
    /// strip shifted spaces
    /// with a config
    pub fn from_byte_slice_strip_shifted_space_with_config(
        s: &'a [u8],
        character_map: &'a SystemConfig,
    ) -> PetsciiString<'a, L> {
        let mut bytes: [u8; L] = [0; L];
        if s.len() > L {
            panic!("u8 slice is too large");
        }

        // Replacing the below manual copy loop between slices with
        // the following recomendation from clippy
        // for i in 0..s.len() {
        //     bytes[i] = s[i];
        // }
        bytes[..s.len()].copy_from_slice(s);

        PetsciiString {
            len: L as u32,
            data: bytes,
            character_map: Some(character_map),
            strip_shifted_space: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write;

    use crate::{
        petscii::{PetsciiConfig, PetsciiString, CONFIG},
        Config, Configuration,
    };

    // #[cfg(feature = "external-json")]
    // use crate::load_config_from_file;

    /// Test loading the configuration works
    ///
    /// There are two tricky problems with testing the config and
    /// static RWLock.  First, tests may be run out-of-order.
    /// Normally they are run alphabetically, but I don't want to play
    /// Yellow Pages games with test names.  I'm sure there are test
    /// crates out there that augment the test framework with
    /// deterministic ordering, but it should be in the core Rust
    /// library.
    ///
    /// Clearing the config in this test and resetting it works
    /// around that issue.
    ///
    /// The other issue is that tests may be run concurrently in
    /// future versions of Rust or custom configurations.
    ///
    /// In this case, there is a small window where a newer config may
    /// be replaced by an older config.
    ///
    /// We have version strings in configurations.  We could add a new
    /// runtime version counter and only accept configurations that
    /// match the expected version or a greater version.
    ///
    /// But that's not implemented yet.  If you are hit by a
    /// concurrency bug in tests in the future, I'm sorry.
    #[test]
    fn petscii_load_config_works() {
        // This test creates a race condition, where another test
        // may try loading config while this test is running.
        //
        // Since tests can be run out-of-order we can't assume that
        // the configuration is uninitialized.
        //
        // This test function acquires a write-lock for the duration of the test
        // Then it saves the old config, replacing it with None.
        // It tests this default value
        // Then it calls load_config normally, tests that it was
        // successful, and then swaps in the original value.
        let mut saved_config: Option<PetsciiConfig> = None;

        // Just read from saved_config so we don't get unused
        // assignment warnings
        // I had an ignore(unused_assignments) which will be a hard
        // error soon!  For now just test it to prevent an unused
        // assignments clippy warning.
        assert!(saved_config.is_none());

        {
            let mut lock_res = CONFIG
                .write()
                .expect("Should be able to acquire config lock");
            // *lock_res = Some(config);
            saved_config = lock_res.take();
        }

        {
            // Now test that a "first" read of the config fails.
            let binding = CONFIG.read().expect("Should be able to get reader lock");
            // Reading an unloaded config should fail
            assert!(binding.as_ref().is_none());
        }

        // Now call load_config and test for a good result
        let config_result = PetsciiConfig::load();
        assert!(config_result.is_ok());

        // Now we should have a Some value in the Option
        {
            let binding = CONFIG.read().expect("Should be able to get reader lock");
            // Reading an loaded config should work
            assert!(binding.as_ref().is_some());
        }

        // Now swap back in the original value
        let mut lock_res = CONFIG
            .write()
            .expect("Should be able to acquire config lock");
        *lock_res = saved_config.take();
    }

    #[test]
    fn petscii_struct_works() {
        let ps = PetsciiString::new(3, [0x41, 0x42, 0x43]);
        assert_eq!(ps.len, 3);
        assert_eq!(ps.data, [0x41, 0x42, 0x43]);
    }

    #[test]
    fn petscii_with_config_works() {
        let config = PetsciiConfig::load().expect("Error loading config file");
        let ps = PetsciiString::new_with_config(
            6,
            [0x41, 0x42, 0x43, 0x5c, 0x5e, 0x5f],
            &config.petscii,
        );
        let mut s: String = String::from(ps);
        assert_eq!(s.pop().unwrap(), '←');
        assert_eq!(s.pop().unwrap(), '↑');
        assert_eq!(s.pop().unwrap(), '£');
        assert_eq!(s.pop().unwrap(), 'C');
        assert_eq!(s.pop().unwrap(), 'B');
        assert_eq!(s.pop().unwrap(), 'A');
    }

    #[test]
    fn petscii_with_config_unmapped_character_works() {
        let config_result = PetsciiConfig::load();
        let config: Config = match config_result {
            Ok(c) => c,
            Err(e) => {
                panic!("Error loading config file: {e}");
            }
        };
        let ps = PetsciiString::new_with_config(2, [0x41, 0xb2], &config.petscii);
        let mut s: String = String::from(ps);
        assert_eq!(s.pop().unwrap(), '┬');
        assert_eq!(s.pop().unwrap(), 'A');
    }

    #[test]
    fn petscii_without_config_works() {
        let ps = PetsciiString::new(6, [0x41, 0x42, 0x43, 0x5c, 0x5e, 0x5f]);
        let mut s: String = String::from(ps);
        assert_eq!(s.pop().unwrap(), '_');
        assert_eq!(s.pop().unwrap(), '^');
        assert_eq!(s.pop().unwrap(), '\\');
        assert_eq!(s.pop().unwrap(), 'C');
        assert_eq!(s.pop().unwrap(), 'B');
        assert_eq!(s.pop().unwrap(), 'A');
    }

    #[test]
    fn petscii_len_unfilled_works() {
        let ps = PetsciiString::new(6, [0x41, 0x42, 0x43]);

        assert_eq!(ps.len(), 6);
    }

    #[test]
    fn petscii_len_7bit_characters_works() {
        let ps = PetsciiString::new(6, [0x41, 0x42, 0x43, 0x41, 0x42, 0x43]);

        assert_eq!(ps.len(), 6);
    }

    #[test]
    fn petscii_len_8bit_characters_works() {
        let ps = PetsciiString::new(7, [0xa5, 0x74, 0x67, 0x7d, 0x68, 0x79, 0xa7]);

        assert_eq!(ps.len(), 7);
    }

    #[test]
    fn petscii_len_from_8bit_character_slice_works() {
        let ps = PetsciiString::new(7, [0xa5, 0x74, 0x67, 0x7d, 0x68, 0x79, 0xa7]);
        let s: String = String::from(ps);

        assert_eq!(s.len(), 9);
        assert_eq!(s.chars().count(), 7);
    }

    #[test]
    fn petscii_len_from_8bit_character_slice_with_config_works() {
        let config = {
            let config_fn = String::from("data/config.json");
            PetsciiConfig::load_from_file(&config_fn).expect("Error loading config file")
        };

        // This should be called at a higher level than when creating
        // strings usually.  Possibly only once at library
        // initialization.
        {
            let mut lock_res = crate::CONFIG
                .write()
                .expect("Should be able to acquire config lock");
            *lock_res = Some(config);
        }

        let binding = crate::CONFIG
            .read()
            .expect("Should be able to get reader lock");
        let config = binding.as_ref().unwrap();

        let ps = PetsciiString::new_with_config(
            6,
            [0x74, 0x67, 0x62, 0x7d, 0x68, 0x79],
            &config.petscii,
        );
        let s: String = String::from(ps);

        // All six charactes are mapped to 32-bit unicode characters
        assert_eq!(s.len(), 24);
        assert_eq!(s.chars().count(), 6);
    }

    /// Test that the Display trait implementation works for
    /// PetsciiString
    ///
    /// This also tests other stuff like the virtual screen code map
    /// and PETSCII to Unicode conversion.
    #[test]
    fn petscii_display_works() {
        let config_fn = String::from("data/config.json");
        let config = Config::load_from_file(&config_fn).expect("Error loading config file");

        let hello_world_data: [u8; 61] = [
            0x0d, 0x0a, 0xb0, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60,
            0x60, 0x60, 0x60, 0x60, 0xae, 0x0d, 0x0a, 0x7d, 0x20, 0x48, 0x0e, 0x45, 0x4c, 0x4c,
            0x4f, 0x2c, 0x20, 0x57, 0x4f, 0x52, 0x4c, 0x44, 0x21, 0x20, 0x8e, 0x7d, 0x0d, 0x0a,
            0xad, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60,
            0x60, 0x60, 0xbd, 0x0d, 0x0a,
        ];

        let expected_unicode: [u32; 59] = [
            13, 10, 9484, 129913, 129913, 129913, 129913, 129913, 129913, 129913, 129913, 129913,
            129913, 129913, 129913, 129913, 129913, 129913, 9488, 13, 10, 129907, 32, 72, 101, 108,
            108, 111, 44, 32, 119, 111, 114, 108, 100, 33, 32, 129907, 13, 10, 9492, 129913,
            129913, 129913, 129913, 129913, 129913, 129913, 129913, 129913, 129913, 129913, 129913,
            129913, 129913, 129913, 9496, 13, 10,
        ];

        let ps = PetsciiString::new_with_config(61, hello_world_data, &config.petscii);

        let mut string_buf = String::new();

        write!(string_buf, "{}", ps).unwrap();

        let bytes: Vec<u32> = string_buf.chars().map(|c| u32::from(c)).collect();

        assert_eq!(Vec::from(expected_unicode), bytes);
    }

    /// Test "shifted" PETSCII lowercase characters
    ///
    /// The default PETSCII character set has uppercase and graphics
    /// characters.  When the character set is shifted, it has
    /// lowercase and uppercase characters.
    #[test]
    fn petscii_test_shifted_lowercase_characters_works() {
        // This data contains a "switch to lower case" PETSCII control
        // character, followed by lowercase characters a through z,
        // followed by a "switch to upper case" character.
        //
        // The behavior of 0x08 "disables shift C= / lock case" and
        // 0x09 "enables shift C= / unlock case" needs to be tested in
        // an emulator to see what needs to be implemented.  It may
        // lead to "switch to lower case" and "switch to upper case"
        // being disabled in the PETSCII data stream.
        let data: [u8; 28] = [
            0x0e, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x4b, 0x4c, 0x4d,
            0x4e, 0x4f, 0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x8e,
        ];

        let config = {
            let config_fn = String::from("data/config.json");
            PetsciiConfig::load_from_file(&config_fn).expect("Error loading config file")
        };

        let ps = PetsciiString::new_with_config(28, data, &config.petscii);

        assert_eq!(ps.len(), 28);

        let s: String = String::from(ps);
        let expected = "abcdefghijklmnopqrstuvwxyz";

        assert_eq!(s, expected);
    }

    /// Test converting a PETSCII string with a Block Elements Unicode
    /// character.
    ///
    /// PETSCII 0xB9 (decimal 185), is a lower three eighths block.
    /// This maps to screen code 0x69 (decimal 121) which is not in
    /// the Unicode Legacy Computing Sources table.
    ///
    /// TODO: Test all of the block element characters
    #[test]
    fn from_petscii_with_block_elements_graphic_character() {
        // This is a PETSCII sequence that contains:
        // lower 3/8 block
        //
        // PETSCII 0xB9 (decimal 185), is a lower three eighths block.
        // This maps to screen code 0x69 (decimal 121) which is not in
        // the Unicode Legacy Computing Sources table.  But it is in the
        // normal Unicode Block Elements code tables as 0x2583
        let data: [u8; 0x01] = [0xB9];

        let config = {
            let config_fn = String::from("data/config.json");
            PetsciiConfig::load_from_file(&config_fn).expect("Error loading config file")
        };

        let ps = PetsciiString::new_with_config(1, data, &config.petscii);

        let s: String = String::from(ps);
        let c = s.chars().nth(0).unwrap();
        let expected: char = char::from_u32(0x2583).unwrap();

        assert_eq!(c, expected);
    }

    // The following test is disabled for now
    //
    // There are quite a few block element characters and they're not
    // all in one place in the Unicode standard.  Some are under Block
    // Elements, some are under Symbols for Legacy Computing or Legacy
    // Computing Sources.
    //
    // In addition, some we have to "infer" or "interpolate" ,
    // e.g. converting a three eighths top block element to five
    // eighths bottom by using reverse characters.
    //
    // Version 0.2.0 of this crate gets Unicode to PETSCII and PETSCII
    // to Unicode working with basic characters including lowercase.
    // Full block elements are a future version.
    //
    // /// Test converting a PETSCII string with reversed-video
    // /// characters
    // #[test]
    // fn from_petscii_with_reversed_works() {
    // 	// This is a PETSCII sequence that contains:
    // 	// REVERSE (RVS) ON, lower three eighths block, REVERSE (RVS) OFF
    // 	//
    // 	// So it should generate a string with an upper five eighths
    // 	// block (0x1FB83 in Symbols for Legacy Computing)
    // 	let data: [u8; 0x03] = [0x12, 0xB9, 0x92];

    //     let config = {
    //         let config_fn = String::from("data/config.json");
    //         PetsciiConfig::load_from_file(&config_fn)
    //             .expect("Error loading config file")
    //     };

    // 	let ps = PetsciiString::new_with_config(3, data, &config.petscii);

    // 	let s: String = String::from(ps);
    // 	let c = s.chars().nth(0).unwrap();
    // 	let expected: char = char::from_u32(0x1FB84).unwrap();

    // 	assert_eq!(c, expected);
    // }

    // // Test some non "Legacy Computing Sources" Unicode characters
    // // These tests also exercise the reverse video control
    // // characters and high-bit PETSCII maps

    #[test]
    fn test_petscii_7bit_playing_cards_to_unicode() {
        // Test low-bits PETSCII playing card characters
        // 0x61 is a black spade
        // 0x73 is a black heart
        // 0x78 is a black club
        // 0x7a is a black diamond
        let data: [u8; 4] = [0x61, 0x73, 0x78, 0x7a];

        let config = PetsciiConfig::load().expect("Error loading config file");

        let ps = PetsciiString::new_with_config(4, data, &config.petscii);
        let s: String = String::from(ps);
        let expected = "♠♥♣♦";

        assert_eq!(s, expected);
    }

    #[test]
    fn test_petscii_8bit_playing_cards_to_unicode() {
        // Test high-bit PETSCII playing card characters
        // 0xc1 is a black spade
        // 0xd3 is a black heart
        // 0xd8 is a black club
        // 0xda is a black diamond
        let data: [u8; 4] = [0xc1, 0xd3, 0xd8, 0xda];

        let config = PetsciiConfig::load().expect("Error loading config");
        let ps = PetsciiString::new_with_config(4, data, &config.petscii);
        let s: String = String::from(ps);
        let expected = "♠♥♣♦";

        assert_eq!(s, expected);
    }

    #[test]
    fn test_petscii_7bit_reversed_video_playing_cards_to_unicode() {
        // Test low-bits PETSCII reversed-video playing card characters
        // 0x61 in reversed video is a white spade
        // 0x73 in reversed video is a white heart
        // 0x78 in reversed video is a white club
        // 0x7a in reversed video is a white diamond
        let data: [u8; 6] = [0x12, 0x61, 0x73, 0x78, 0x7a, 0x92];

        let config = PetsciiConfig::load().expect("Error loading config");
        let ps = PetsciiString::new_with_config(6, data, &config.petscii);
        let s: String = String::from(ps);
        let expected = "♤♡♧♢";

        assert_eq!(s, expected);
    }

    #[test]
    fn test_petscii_8bit_reversed_video_playing_cards_to_unicode() {
        // Test high-bit PETSCII reversed-video playing card characters
        // 0xc1 in reversed video is a white spade
        // 0xd3 in reversed video is a white heart
        // 0xd8 in reversed video is a white club
        // 0xda in reversed video is a white diamond
        let data: [u8; 6] = [0x12, 0xc1, 0xd3, 0xd8, 0xda, 0x92];

        let config = PetsciiConfig::load().expect("Error loading config");
        let ps = PetsciiString::new_with_config(6, data, &config.petscii);
        let s: String = String::from(ps);
        let expected = "♤♡♧♢";

        assert_eq!(s, expected);
    }

    #[test]
    fn into_iter_works() {
        #[cfg(not(feature = "external-json"))]
        let config = PetsciiConfig::load().expect("Error loading config");
        #[cfg(feature = "external-json")]
        let config = {
            let config_fn = String::from("data/config.json");
            PetsciiConfig::load_from_file(&config_fn).expect("Error loading config file")
        };

        let ps = PetsciiString::new_with_config(3, [0x41, 0x42, 0x43], &config.petscii);

        let mut iter = ps.into_iter();

        assert_eq!(iter.next(), Some(0x41));
        assert_eq!(iter.next(), Some(0x42));
        assert_eq!(iter.next(), Some(0x43));
        assert_eq!(iter.next(), None);
    }

    // Tests from Unicode to PETSCII

    /// Test basic uppercase Unicode to PETSCII works
    #[test]
    fn petscii_test_from_unicode_uppercase_characters_works() {
        let uppercase = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let s: String = String::from(uppercase);

        let expected: [u8; 26] = [
            0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x4b, 0x4c, 0x4d, 0x4e,
            0x4f, 0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a,
        ];

        let config = PetsciiConfig::load().expect("Error loading config");

        let ps = PetsciiString::<26>::from_str_with_config(&s, &config.petscii);

        assert_eq!(ps.len(), 26);
        assert_eq!(ps.data, expected);

        let s: String = String::from(ps);

        assert_eq!(s, uppercase);
    }

    /// Test "shifted" PETSCII lowercase characters
    ///
    /// The default PETSCII character set has uppercase and graphics
    /// characters.  When the character set is shifted, it has
    /// lowercase and uppercase characters.
    ///
    /// I love that this test found a possible bug where I wasn't
    /// shifting out when there were no uppercase characters at the
    /// end of a string.  We'll assume the user wants the state of the
    /// character set to return to the default, but that should be
    /// specified.
    #[test]
    fn petscii_test_from_unicode_lowercase_characters_works() {
        let lowercase = "abcdefghijklmnopqrstuvwxyz";
        let s: String = String::from(lowercase);

        // This data contains a "switch to lower case" PETSCII control
        // character, followed by lowercase characters a through z,
        // followed by a "switch to upper case" character.
        //
        // The behavior of 0x08 "disables shift C= / lock case" and
        // 0x09 "enables shift C= / unlock case" needs to be tested in
        // an emulator to see what needs to be implemented.  It may
        // lead to "switch to lower case" and "switch to upper case"
        // being disabled in the PETSCII data stream.
        let expected: [u8; 28] = [
            0x0e, 0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x4b, 0x4c, 0x4d,
            0x4e, 0x4f, 0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a, 0x8e,
        ];

        let config = PetsciiConfig::load().expect("Error loading config");

        let ps = PetsciiString::<28>::from_str_with_config(&s, &config.petscii);

        assert_eq!(ps.len(), 28);
        assert_eq!(ps.data, expected);

        let s: String = String::from(ps);

        assert_eq!(s, lowercase);
    }
}
