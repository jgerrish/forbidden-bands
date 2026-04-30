//!
//! ZSCII string library
//!
//! ZSCII is the character set used by the Z-machine, the virtual
//! machine that runs Infocom adventure games and other games targeted
//! to the VM.
//!
//! ZSCII stands for Zork Standard Code for Information Interchange
//! It rhymes with "xyzzy".
//!
//! ZSCII is encoded as a series of Z-characters.  This library will
//! allow you to read a byte stream or buffer of those characters and
//! decode them into ZSCII and then into Unicode.  Or you can convert
//! from Unicode to Z-characters.
//!
//! Currently a very small subset of ZSCII is supported.  Basic
//! decoding of ZSCII into the three short-word alphabets (A0, A1, and
//! A2) is supported.
//!
//! Encoding from Unicode to ZSCII and Z-characters is not supported
//! yet.
//!
//! There is configuration data for the default Unicode table that
//! ZSCII supports included, but encoding and decoding those
//! characters isn't supported yet.
#![warn(missing_docs)]
#![warn(unsafe_code)]

use std::sync::RwLock;

// See the notes about optional JSON support in the Cargo.toml file
// #[cfg(feature = "json")]
use serde::{Deserialize, Serialize};
// #[cfg(feature = "json")]
use serde_json::{Map, Value};

use crate::Configuration;

/// A ZSCII string
///
/// A fixed-length ZSCII string
///
/// ZSCII stands for Zork Standard Code for Information Interchange
/// It rhymes with "xyzzy".
///
/// ZSCII is encoded as a sequence of two-byte words.  Each word
/// contains three 5-bit Z-characters and an end-of-text flag.
///
/// These Z-characters then encode ZSCII character codes.
#[derive(Clone, Copy, Debug)]
pub struct ZsciiString<const L: usize> {
    /// Version of the Z-Machine for this ZSCII string
    pub version: u8,

    /// The length of the string
    pub len: u32,

    /// The string data
    pub data: [u8; L],
}

/// The error types the Z-Machine can return
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Error {
    /// An invalid character code
    InvalidCharacter,

    /// The feature is unimplemented
    Unimplemented,
}

/// Configuration data including character maps for the ZSCII crate
// #[cfg(feature = "json")]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZsciiConfig {
    /// Version of the ZSCII config
    pub version: String,

    /// Version of the z-machine
    pub zmachine_version: Option<u8>,

    /// ZSCII codes to Unicode
    pub zscii_extra_characters_to_unicode_default_table: Map<String, Value>,

    /// Unicode to ZSCII
    pub zscii_unicode_to_extra_characters_default_table: Map<String, Value>,
}

/// Configuration data for the ZSCII crate
///
/// We try to load this once on first use and then only read from it
/// There is an overhead creating each ZsciiString getting a read
/// lock on the config variable.
pub static CONFIG: RwLock<Option<ZsciiConfig>> = RwLock::new(None);

/// Load the configuration data from the ZSCII configuration string
impl ZsciiConfig {
    /// Load the configuration data from the default configuration
    /// string
    pub fn load() -> std::result::Result<ZsciiConfig, crate::error::Error> {
        // First see if the configuration is already loaded
        {
            let binding = CONFIG.read().expect("Should be able to get reader lock");

            let test = binding.as_ref();

            if let Some(zscii_config) = test {
                return Ok(zscii_config.clone());
            }
        }

        // Config was not already loaded
        let crate_config = crate::Config::load()?;
        if !crate_config.systems.contains_key("zscii") {
            return Err(crate::error::Error::new_with_message(String::from(
                "no key for ZSCII system in config",
            )));
        }
        let zscii = crate_config
            .systems
            .get("zscii")
            .expect("Should find ZSCII system");

        let char_set_map = &zscii.character_set_map;
        let zscii_config: ZsciiConfig = serde_json::from_value(char_set_map.clone())?;

        {
            let mut lock_res = CONFIG
                .write()
                .expect("Should be able to acquire config lock");
            *lock_res = Some(zscii_config.clone());
        }

        Ok(zscii_config)
    }

    /// Load configuration from a file
    pub fn load_from_file(filename: &str) -> std::result::Result<ZsciiConfig, crate::error::Error> {
        let crate_config = crate::Config::load_from_file(filename)?;

        if !crate_config.systems.contains_key("zscii") {
            return Err(crate::error::Error::new_with_message(String::from(
                "no key for ZSCII system in config",
            )));
        }

        let zscii = crate_config
            .systems
            .get("zscii")
            .expect("Should find ZSCII system");

        let char_set_map = &zscii.character_set_map;
        let zscii_config: ZsciiConfig = serde_json::from_value(char_set_map.clone())?;

        Ok(zscii_config)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Alphabet {
    /// The first alphabet
    /// It's the numbers 0x06 to 0x1f mapped to the lowercase letters a-z
    A0 = 0,
    /// The second alphabet
    /// It's the numbers 0x06 to 0x1f mapped to the uppercase letters A-Z
    A1 = 1,
    /// The third alphabet
    /// It's the numbers 0x07 to 0x1f mapped to:
    ///   \r 0 1 2 3 4 5 6 7 8 9 . , ! ? _ # ` " / \ - : ( )
    /// Character 6 is not used, Character 7 is a carrige return / newline
    A2 = 2,
}

/// The current temporary shift state
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ShiftState {
    /// Shift is disabled
    Disabled,
    /// Shift is enabled and characters are shifted up
    Up,
    /// Shift is enabled and characters are shifted down
    Down,
}

/// The state of the ZSCII processor
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct State {
    /// The version of the z-machine
    pub version: u8,
    /// The current alphabet
    pub alphabet: Alphabet,
    /// Whether temporary shift state is currently disabled, up or
    /// down
    pub shifted: ShiftState,
    /// True if the last Z-character seen was 6 in alphabet A2, which
    /// means the next two Z-characters form a ten-bit ZSCII code.
    pub ten_bit_zscii_mode: bool,
    /// The most-significant 5 bits of the ten-bit ZSCII word
    pub ten_bit_zscii_word: Option<u8>,
}

impl State {
    /// Create a new State for a given Z-Machine version
    fn new(version: u8) -> State {
        State {
            version,
            alphabet: Alphabet::A0,
            shifted: ShiftState::Disabled,
            ten_bit_zscii_mode: false,
            ten_bit_zscii_word: None,
        }
    }
}

/// Whether a shift is temporary (for the next character only) or
/// permanent (for every succeeding character)
enum ShiftType {
    /// The shift is only for the next character
    Temporary,
    /// The shift changes the alphabet permanently
    Permanent,
}

/// Shift an alphabet up.
/// Returns the new alphabet
fn shift_alphabet_up(alphabet: Alphabet) -> Alphabet {
    match alphabet {
        Alphabet::A0 => Alphabet::A1,
        Alphabet::A1 => Alphabet::A2,
        Alphabet::A2 => Alphabet::A0,
    }
}

/// Shift an alphabet down.
/// Returns the new alphabet
fn shift_alphabet_down(alphabet: Alphabet) -> Alphabet {
    match alphabet {
        Alphabet::A0 => Alphabet::A2,
        Alphabet::A1 => Alphabet::A0,
        Alphabet::A2 => Alphabet::A1,
    }
}

/// Shift an alphabet up or down.
/// Returns the new alphabet
fn shift_alphabet(alphabet: Alphabet, shift_state: ShiftState) -> Alphabet {
    match shift_state {
        ShiftState::Disabled => alphabet,
        ShiftState::Up => shift_alphabet_up(alphabet),
        ShiftState::Down => shift_alphabet_down(alphabet),
    }
}

/// Shift the alphabet in the current state up, either temporarily or
/// permanently.
/// Returns the new state with the changes applied.
/// Changes the State
fn shift_up(state: &mut State, shift_type: ShiftType) -> &mut State {
    match shift_type {
        ShiftType::Temporary => {
            state.shifted = ShiftState::Up;
        }
        ShiftType::Permanent => {
            state.alphabet = shift_alphabet(state.alphabet, ShiftState::Up);
        }
    }

    state
}

/// Shift the alphabet in the current state down, either temporarily or
/// permanently.
/// Returns the new state with the changes applied
/// Changes the State
fn shift_down(state: &mut State, shift_type: ShiftType) -> &mut State {
    match shift_type {
        ShiftType::Temporary => {
            state.shifted = ShiftState::Down;
        }
        ShiftType::Permanent => {
            state.alphabet = shift_alphabet(state.alphabet, ShiftState::Down);
        }
    }

    state
}

/// Shift the alphabet in the current state up or down, either
/// temporarily or permanently.
/// Returns the new state with the changes applied
/// Changes the State
fn shift(state: &mut State, shift_state: ShiftState, shift_type: ShiftType) -> &mut State {
    match shift_state {
        ShiftState::Disabled => state,
        ShiftState::Up => shift_up(state, shift_type),
        ShiftState::Down => shift_down(state, shift_type),
    }
}

/// Get an ASCII character code given an alphabet and ZSCII code
///
/// I need to be clear about what a value is, whether it is a
/// Z-character, ZSCII, ASCII or Unicode character
fn get_character(alphabet: &Alphabet, code: u8) -> Result<u8, Error> {
    let a2: [u8; 25] = [
        0x0d, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x2e, 0x2c, 0x21, 0x3f,
        0x5f, 0x23, 0x60, 0x22, 0x2f, 0x5c, 0x2d, 0x3a, 0x28, 0x29,
    ];

    match alphabet {
        Alphabet::A0 => match code {
            0x00..=0x05 => Err(Error::InvalidCharacter),
            0x06..=0x1f => Ok(97 - 6 + code),
            0x20..=u8::MAX => Err(Error::InvalidCharacter),
        },
        Alphabet::A1 => match code {
            0x00..=0x05 => Err(Error::InvalidCharacter),
            0x06..=0x1f => Ok(65 - 6 + code),
            0x20..=u8::MAX => Err(Error::InvalidCharacter),
        },
        Alphabet::A2 => match code {
            0x00..=0x06 => Err(Error::InvalidCharacter),
            0x07..=0x1f => Ok(a2[code as usize - 7]),
            0x20..=u8::MAX => Err(Error::InvalidCharacter),
        },
    }
}

/// Decodes a ten-bit z-character
///
/// The basic ASCII letters, numbers and a few symbols are available
/// using 5-bit z-characters.  Additional characters and special
/// sequences are encoded using ten bits.
fn decode_ten_bit_zchar(ten_bit_zchar: u16) -> Result<Option<u8>, Error> {
    match ten_bit_zchar {
        0 => Ok(Some(0x00)),
        1..=7 => Ok(None),
        8 => Ok(Some(0x7f)),
        9 => Ok(Some(0x09)),
        10 => Ok(None),
        11 => Ok(Some(0x20)),
        12 => Ok(None),
        13 => Ok(Some(0x0d)),
        14..=26 => Ok(None),
        27 => Ok(Some(0x1b)),
        28..=31 => Ok(None),
        32..=126 => Ok(Some(ten_bit_zchar as u8)),
        127..=128 => Ok(None),
        // TODO: Implement cursor up, down, left and right
        129..=132 => Err(Error::Unimplemented),
        // TODO: Implement function keys F1 - F12
        133..=144 => Err(Error::Unimplemented),
        // TODO: keypad 0-9
        145..=154 => Err(Error::Unimplemented),
        155..=251 => Err(Error::Unimplemented),
        // TODO: Implment menu click (v6)
        252 => Err(Error::Unimplemented),
        // TODO: Implment double click (v6)
        253 => Err(Error::Unimplemented),
        // TODO: Implment single click
        254 => Err(Error::Unimplemented),
        255..=1023 => Ok(None),
        1024..=u16::MAX => Err(Error::InvalidCharacter),
    }
}

impl<const L: usize> ZsciiString<L> {
    /// Create a new ZSCII string with a given character map
    ///
    /// # Examples
    ///
    /// ```
    /// use forbidden_bands::{
    ///     zscii::{ZsciiConfig, ZsciiString},
    ///     Config,
    ///     Configuration,
    /// };
    ///
    /// let mut config = ZsciiConfig::load().expect("Error loading config file");
    /// config.zmachine_version = Some(3);
    ///
    /// let ps = ZsciiString::new_with_config(&config, 2, [0x1c, 0xd9]);
    /// let mut s: String = String::from(ps);
    ///
    /// assert_eq!(s.pop().unwrap(), 't');
    /// assert_eq!(s.pop().unwrap(), 'a');
    /// assert_eq!(s.pop().unwrap(), 'b');
    /// ```
    pub fn new_with_config(config: &ZsciiConfig, len: u32, data: [u8; L]) -> Self {
        ZsciiString {
            len,
            data,
            version: config.zmachine_version.unwrap(),
        }
    }
}

/// An IntoIter structure for ZsciiStrings
/// We need to keep track of the index of the current element, along
/// with the data.
///
/// TODO: We may need to keep track of the end-of-text marker which is
/// the most-significant bit in each 2-byte word
pub struct IntoIter<const L: usize> {
    index: usize,
    data: ZsciiString<L>,
    /// The MSB marker used to indicate end-of-text
    end_of_text_marker: bool,
    chars_to_send: Vec<u8>,
}

// Two normal bytes (16-bits) contain three Z-characters
// Each ZSCII character code is a 5-bit value.
//
// This iterator will feed 5-bits at a time to the user.
// It is then up to the other logic to decode those 5-bits
impl<const L: usize> IntoIterator for ZsciiString<L> {
    type Item = u8;
    type IntoIter = IntoIter<L>;
    fn into_iter(self) -> IntoIter<L> {
        IntoIter {
            index: 0,
            data: self,
            end_of_text_marker: false,
            chars_to_send: Vec::new(),
        }
    }
}

impl<const L: usize> IntoIter<L> {
    fn get_next_words(&mut self) -> Vec<u8> {
        let first_byte = self.data.data[self.index];
        let second_byte = self.data.data[self.index + 1];

        self.end_of_text_marker = (first_byte & 0b10000000) == 0b10000000;

        let char1 = (first_byte & 0b01111100) >> 2;
        let char2 = ((first_byte & 0b00000011) << 3) | ((second_byte & 0b11100000) >> 5);
        let char3 = second_byte & 0b00011111;
        // TODO: Figure if there is a way to feed these characters
        // individually without state
        //
        // The characters are added in backwards order here so we can
        // dequeue them using a standard Vec and pop()
        vec![char3, char2, char1]
    }
}

// Two normal bytes (16-bits) contain three Z-characters
// This seems like the natural place to setup the character code iterator
// Each ZSCII character code is a 5-bit value.
// The iterator will feed 5-bits at a time to the user.
// It's then up to the other logic to decode those 5-bits
impl<const L: usize> Iterator for IntoIter<L> {
    type Item = u8;
    fn next(&mut self) -> Option<Self::Item> {
        // Check first_byte, because we want to return the whole three
        // characters and only finish on the next 2-byte sequence
        if self.chars_to_send.is_empty() {
            // if self.end_of_text_marker && self.first_byte() {
            // 	return None;
            println!(
                "self.data.len: {}, self.index: {}",
                self.data.len, self.index
            );
            if self.index < (self.data.len).try_into().unwrap() {
                let mut words: Vec<u8> = self.get_next_words();
                self.chars_to_send.append(&mut words);
                self.index += 2;
            } else {
                return None;
            }
        };
        self.chars_to_send.pop()
    }
}

/// This function decodes a 5-bit z-character
fn decode_char(state: &mut State, c: u8) -> Option<char> {
    println!("Decoding char: {:?}", c);

    let d: Option<char> = match c {
        0x00 => char::from_u32(0x20),
        0x01 => {
            match state.version {
                0 => None,
                1 => char::from_u32(0x0A),
                2..=u8::MAX => {
                    // TODO: Print abbreviation from config
                    None
                }
            }
        }
        0x02 => {
            match state.version {
                0 => None,
                // Versions 1 and 2 use character two as a shift up
                // character for the next character only.
                1..=2 => {
                    shift(state, ShiftState::Up, ShiftType::Temporary);
                    None
                }
                3..=u8::MAX => {
                    // TODO: Print abbreviation from config
                    None
                }
            }
        }
        0x03 => {
            match state.version {
                0 => None,
                // Versions 1 and 2 use character three as a shift down
                // character for the next character only.
                1..=2 => {
                    shift(state, ShiftState::Down, ShiftType::Temporary);
                    None
                }
                3..=u8::MAX => {
                    // TODO: Print abbreviation from config
                    None
                }
            }
        }
        0x04 => {
            match state.version {
                0 => None,
                // Versions 1 and 2 use character four as a shift up
                // character that permanently changes the alphabet
                1..=2 => {
                    shift(state, ShiftState::Up, ShiftType::Permanent);
                    None
                }
                // Versions 3 and 4 use character four as a temporary
                // shift up for one character only
                3..=4 => {
                    shift(state, ShiftState::Up, ShiftType::Temporary);
                    None
                }
                5..=u8::MAX => {
                    // TODO: Print abbreviation from config
                    None
                }
            }
        }
        0x05 => {
            match state.version {
                0 => None,
                // Versions 1 and 2 use character four as a shift down
                // character that permanently changes the alphabet
                1..=2 => {
                    shift(state, ShiftState::Down, ShiftType::Permanent);
                    None
                }
                // Versions 3 and 4 use character four as a temporary
                // shift down for one character only
                3..=4 => {
                    shift(state, ShiftState::Down, ShiftType::Temporary);
                    None
                }
                5..=u8::MAX => {
                    // TODO: Print abbreviation from config
                    None
                }
            }
        }
        0x06 => {
            // Z-Character code 6 means the next two Z-characters form
            // a ten-bit ZSCII character code.
            let alphabet = match state.shifted {
                ShiftState::Disabled => state.alphabet,
                ShiftState::Up => {
                    state.shifted = ShiftState::Disabled;
                    shift_alphabet(state.alphabet, ShiftState::Up)
                }
                ShiftState::Down => {
                    state.shifted = ShiftState::Disabled;
                    shift_alphabet(state.alphabet, ShiftState::Down)
                }
            };
            match alphabet {
                Alphabet::A0 => char::from_u32(
                    get_character(&alphabet, c)
                        .unwrap_or_else(|_| {
                            panic!("Should decode character: {:?} {}", &alphabet, c)
                        })
                        .into(),
                ),
                Alphabet::A1 => {
                    char::from_u32(
                        get_character(&alphabet, c)
                            .unwrap_or_else(|_| {
                                panic!("Should decode character: {:?} {}", &alphabet, c)
                            })
                            .into(),
                    )
                    // let zchar_result = get_character(&alphabet, c);
                    // match zchar_result {
                    // 	Ok(zchar) => char::from_u32(zchar.into()),
                    // 	Err(e) => panic!("{:?}", e),
                    // }
                }
                Alphabet::A2 => {
                    state.ten_bit_zscii_mode = true;
                    None
                }
            }
        }
        0x07..=0x1f => {
            let alphabet = match state.shifted {
                ShiftState::Disabled => state.alphabet,
                ShiftState::Up => {
                    state.shifted = ShiftState::Disabled;
                    shift_alphabet(state.alphabet, ShiftState::Up)
                }
                ShiftState::Down => {
                    state.shifted = ShiftState::Disabled;
                    shift_alphabet(state.alphabet, ShiftState::Down)
                }
            };
            char::from_u32(
                get_character(&alphabet, c)
                    .unwrap_or_else(|_| panic!("Should decode character: {:?} {}", &alphabet, c))
                    .into(),
            )
        }
        _ => None,
    };

    d
}

impl<const L: usize> From<&ZsciiString<L>> for String {
    /// Create a String from a reference to a ZsciiString
    ///
    /// # Examples
    ///
    /// ```
    /// use forbidden_bands::{
    ///     zscii::{ZsciiConfig, ZsciiString},
    ///     Config,
    ///     Configuration,
    /// };
    ///
    /// let mut config = ZsciiConfig::load().expect("Error loading config file");
    /// config.zmachine_version = Some(3);
    ///
    /// let ps = ZsciiString::new_with_config(&config, 4, [0x65, 0xae, 0xa9, 0x65]);
    /// let mut s: String = String::from(ps);
    ///
    /// assert_eq!(s.pop().unwrap(), 'f');
    /// assert_eq!(s.pop().unwrap(), 'e');
    /// assert_eq!(s.pop().unwrap(), 'i');
    /// assert_eq!(s.pop().unwrap(), 'h');
    /// assert_eq!(s.pop().unwrap(), 't');
    /// ```
    fn from(s: &ZsciiString<L>) -> String {
        let mut state = State::new(s.version);

        s.into_iter()
            .filter_map(|c| {
                println!("Next char: {}", c);
                // the ZsciiString iterator feeds us 5-bit words, we
                // don't need to worry about the encoding of those
                // words into bytes.
                if state.ten_bit_zscii_mode {
                    // Start decoding a ten-bit ZSCII character
                    if let Some(high_bits) = state.ten_bit_zscii_word {
                        let ten_bit_zchar: u16 = ((high_bits << 5) | c).into();
                        let decode_ten_bit_zchar_result = decode_ten_bit_zchar(ten_bit_zchar);
                        match decode_ten_bit_zchar_result {
                            Ok(r) => match r {
                                Some(x) => char::from_u32(x.into()),
                                None => None,
                            },
                            Err(e) => {
                                panic!("Error: {:?}", e);
                            }
                        }
                    } else {
                        state.ten_bit_zscii_word = Some(c);
                        None
                    }
                } else {
                    decode_char(&mut state, c)
                }
            })
            .collect()
    }
}

impl<const L: usize> From<ZsciiString<L>> for String {
    /// Create a String from a ZsciiString
    ///
    /// # Examples
    ///
    /// ```
    /// use forbidden_bands::{
    ///     zscii::{ZsciiConfig, ZsciiString},
    ///     Config,
    ///     Configuration,
    /// };
    ///
    /// let mut config = ZsciiConfig::load().expect("Error loading config file");
    /// config.zmachine_version = Some(3);
    ///
    /// let ps = ZsciiString::new_with_config(&config, 4, [0x65, 0xae, 0xa9, 0x65]);
    /// let mut s: String = String::from(ps);
    ///
    /// assert_eq!(s.pop().unwrap(), 'f');
    /// assert_eq!(s.pop().unwrap(), 'e');
    /// assert_eq!(s.pop().unwrap(), 'i');
    /// assert_eq!(s.pop().unwrap(), 'h');
    /// assert_eq!(s.pop().unwrap(), 't');
    /// ```
    fn from(s: ZsciiString<L>) -> String {
        String::from(&s)
    }
}

#[cfg(test)]
mod tests {
    // use std::fmt::Write;

    use crate::zscii::{
        get_character, shift, shift_alphabet, shift_alphabet_down, shift_alphabet_up, shift_down,
        shift_up, Alphabet, ShiftState, ShiftType, State, ZsciiConfig, ZsciiString,
    };

    #[test]
    fn shift_alphabet_up_works() {
        let mut alphabet = Alphabet::A0;

        assert_eq!(alphabet, Alphabet::A0);

        alphabet = shift_alphabet_up(alphabet);
        assert_eq!(alphabet, Alphabet::A1);

        alphabet = shift_alphabet_up(alphabet);
        assert_eq!(alphabet, Alphabet::A2);

        alphabet = shift_alphabet_up(alphabet);
        assert_eq!(alphabet, Alphabet::A0);
    }

    #[test]
    fn shift_alphabet_down_works() {
        let mut alphabet = Alphabet::A2;

        assert_eq!(alphabet, Alphabet::A2);

        alphabet = shift_alphabet_down(alphabet);
        assert_eq!(alphabet, Alphabet::A1);

        alphabet = shift_alphabet_down(alphabet);
        assert_eq!(alphabet, Alphabet::A0);

        alphabet = shift_alphabet_down(alphabet);
        assert_eq!(alphabet, Alphabet::A2);
    }

    #[test]
    fn shift_alphabet_works() {
        // Reset alphabet for shift up tests
        let mut alphabet = Alphabet::A0;

        assert_eq!(alphabet, Alphabet::A0);

        alphabet = shift_alphabet(alphabet, ShiftState::Disabled);
        assert_eq!(alphabet, Alphabet::A0);

        alphabet = shift_alphabet(alphabet, ShiftState::Up);
        assert_eq!(alphabet, Alphabet::A1);

        alphabet = shift_alphabet(alphabet, ShiftState::Up);
        assert_eq!(alphabet, Alphabet::A2);

        alphabet = shift_alphabet(alphabet, ShiftState::Up);
        assert_eq!(alphabet, Alphabet::A0);

        // Reset alphabet for shift down tests
        alphabet = Alphabet::A2;
        assert_eq!(alphabet, Alphabet::A2);

        alphabet = shift_alphabet(alphabet, ShiftState::Down);
        assert_eq!(alphabet, Alphabet::A1);

        alphabet = shift_alphabet(alphabet, ShiftState::Down);
        assert_eq!(alphabet, Alphabet::A0);

        alphabet = shift_alphabet(alphabet, ShiftState::Down);
        assert_eq!(alphabet, Alphabet::A2);
    }

    // Tests for state manipulation functions

    #[test]
    fn shift_up_works() {
        let mut state = State {
            version: 1,
            alphabet: Alphabet::A0,
            shifted: ShiftState::Disabled,
            ten_bit_zscii_mode: false,
            ten_bit_zscii_word: None,
        };

        assert_eq!(state.version, 1);
        assert_eq!(state.alphabet, Alphabet::A0);
        assert_eq!(state.shifted, ShiftState::Disabled);

        // Test permanent shift

        shift_up(&mut state, ShiftType::Permanent);
        assert_eq!(state.alphabet, Alphabet::A1);
        assert_eq!(state.shifted, ShiftState::Disabled);

        shift_up(&mut state, ShiftType::Permanent);
        assert_eq!(state.alphabet, Alphabet::A2);
        assert_eq!(state.shifted, ShiftState::Disabled);

        shift_up(&mut state, ShiftType::Permanent);
        assert_eq!(state.alphabet, Alphabet::A0);
        assert_eq!(state.shifted, ShiftState::Disabled);

        // Test temporary shift
        state = State {
            version: 1,
            alphabet: Alphabet::A0,
            shifted: ShiftState::Disabled,
            ten_bit_zscii_mode: false,
            ten_bit_zscii_word: None,
        };
        assert_eq!(state.version, 1);
        assert_eq!(state.alphabet, Alphabet::A0);
        assert_eq!(state.shifted, ShiftState::Disabled);

        shift_up(&mut state, ShiftType::Temporary);
        assert_eq!(state.alphabet, Alphabet::A0);
        assert_eq!(state.shifted, ShiftState::Up);

        state = State {
            version: 1,
            alphabet: Alphabet::A1,
            shifted: ShiftState::Disabled,
            ten_bit_zscii_mode: false,
            ten_bit_zscii_word: None,
        };
        assert_eq!(state.version, 1);
        assert_eq!(state.alphabet, Alphabet::A1);
        assert_eq!(state.shifted, ShiftState::Disabled);

        shift_up(&mut state, ShiftType::Temporary);
        assert_eq!(state.alphabet, Alphabet::A1);
        assert_eq!(state.shifted, ShiftState::Up);

        state = State {
            version: 1,
            alphabet: Alphabet::A2,
            shifted: ShiftState::Disabled,
            ten_bit_zscii_mode: false,
            ten_bit_zscii_word: None,
        };
        assert_eq!(state.version, 1);
        assert_eq!(state.alphabet, Alphabet::A2);
        assert_eq!(state.shifted, ShiftState::Disabled);

        shift_up(&mut state, ShiftType::Temporary);
        assert_eq!(state.alphabet, Alphabet::A2);
        assert_eq!(state.shifted, ShiftState::Up);
    }

    #[test]
    fn shift_down_works() {
        let mut state = State {
            version: 1,
            alphabet: Alphabet::A2,
            shifted: ShiftState::Disabled,
            ten_bit_zscii_mode: false,
            ten_bit_zscii_word: None,
        };

        assert_eq!(state.version, 1);
        assert_eq!(state.alphabet, Alphabet::A2);
        assert_eq!(state.shifted, ShiftState::Disabled);

        // Test permanent shift

        shift_down(&mut state, ShiftType::Permanent);
        assert_eq!(state.alphabet, Alphabet::A1);
        assert_eq!(state.shifted, ShiftState::Disabled);

        shift_down(&mut state, ShiftType::Permanent);
        assert_eq!(state.alphabet, Alphabet::A0);
        assert_eq!(state.shifted, ShiftState::Disabled);

        shift_down(&mut state, ShiftType::Permanent);
        assert_eq!(state.alphabet, Alphabet::A2);
        assert_eq!(state.shifted, ShiftState::Disabled);

        // Test temporary shift
        state = State {
            version: 1,
            alphabet: Alphabet::A2,
            shifted: ShiftState::Disabled,
            ten_bit_zscii_mode: false,
            ten_bit_zscii_word: None,
        };
        assert_eq!(state.version, 1);
        assert_eq!(state.alphabet, Alphabet::A2);
        assert_eq!(state.shifted, ShiftState::Disabled);

        shift_down(&mut state, ShiftType::Temporary);
        assert_eq!(state.alphabet, Alphabet::A2);
        assert_eq!(state.shifted, ShiftState::Down);

        state = State {
            version: 1,
            alphabet: Alphabet::A1,
            shifted: ShiftState::Disabled,
            ten_bit_zscii_mode: false,
            ten_bit_zscii_word: None,
        };
        assert_eq!(state.version, 1);
        assert_eq!(state.alphabet, Alphabet::A1);
        assert_eq!(state.shifted, ShiftState::Disabled);

        shift_down(&mut state, ShiftType::Temporary);
        assert_eq!(state.alphabet, Alphabet::A1);
        assert_eq!(state.shifted, ShiftState::Down);

        state = State {
            version: 1,
            alphabet: Alphabet::A0,
            shifted: ShiftState::Disabled,
            ten_bit_zscii_mode: false,
            ten_bit_zscii_word: None,
        };
        assert_eq!(state.version, 1);
        assert_eq!(state.alphabet, Alphabet::A0);
        assert_eq!(state.shifted, ShiftState::Disabled);

        shift_down(&mut state, ShiftType::Temporary);
        assert_eq!(state.alphabet, Alphabet::A0);
        assert_eq!(state.shifted, ShiftState::Down);
    }

    #[test]
    fn shift_works() {
        // Test shift up works
        let mut state = State {
            version: 1,
            alphabet: Alphabet::A0,
            shifted: ShiftState::Disabled,
            ten_bit_zscii_mode: false,
            ten_bit_zscii_word: None,
        };

        assert_eq!(state.version, 1);
        assert_eq!(state.alphabet, Alphabet::A0);
        assert_eq!(state.shifted, ShiftState::Disabled);

        // Test permanent shift

        shift(&mut state, ShiftState::Up, ShiftType::Permanent);
        assert_eq!(state.alphabet, Alphabet::A1);
        assert_eq!(state.shifted, ShiftState::Disabled);

        shift(&mut state, ShiftState::Up, ShiftType::Permanent);
        assert_eq!(state.alphabet, Alphabet::A2);
        assert_eq!(state.shifted, ShiftState::Disabled);

        shift(&mut state, ShiftState::Up, ShiftType::Permanent);
        assert_eq!(state.alphabet, Alphabet::A0);
        assert_eq!(state.shifted, ShiftState::Disabled);

        // Test temporary shift
        state = State {
            version: 1,
            alphabet: Alphabet::A0,
            shifted: ShiftState::Disabled,
            ten_bit_zscii_mode: false,
            ten_bit_zscii_word: None,
        };
        assert_eq!(state.version, 1);
        assert_eq!(state.alphabet, Alphabet::A0);
        assert_eq!(state.shifted, ShiftState::Disabled);

        shift(&mut state, ShiftState::Up, ShiftType::Temporary);
        assert_eq!(state.alphabet, Alphabet::A0);
        assert_eq!(state.shifted, ShiftState::Up);

        state = State {
            version: 1,
            alphabet: Alphabet::A1,
            shifted: ShiftState::Disabled,
            ten_bit_zscii_mode: false,
            ten_bit_zscii_word: None,
        };
        assert_eq!(state.version, 1);
        assert_eq!(state.alphabet, Alphabet::A1);
        assert_eq!(state.shifted, ShiftState::Disabled);

        shift(&mut state, ShiftState::Up, ShiftType::Temporary);
        assert_eq!(state.alphabet, Alphabet::A1);
        assert_eq!(state.shifted, ShiftState::Up);

        state = State {
            version: 1,
            alphabet: Alphabet::A2,
            shifted: ShiftState::Disabled,
            ten_bit_zscii_mode: false,
            ten_bit_zscii_word: None,
        };
        assert_eq!(state.version, 1);
        assert_eq!(state.alphabet, Alphabet::A2);
        assert_eq!(state.shifted, ShiftState::Disabled);

        shift(&mut state, ShiftState::Up, ShiftType::Temporary);
        assert_eq!(state.alphabet, Alphabet::A2);
        assert_eq!(state.shifted, ShiftState::Up);

        // Test shift down

        let mut state = State {
            version: 1,
            alphabet: Alphabet::A2,
            shifted: ShiftState::Disabled,
            ten_bit_zscii_mode: false,
            ten_bit_zscii_word: None,
        };

        assert_eq!(state.version, 1);
        assert_eq!(state.alphabet, Alphabet::A2);
        assert_eq!(state.shifted, ShiftState::Disabled);

        // Test permanent shift

        shift(&mut state, ShiftState::Down, ShiftType::Permanent);
        assert_eq!(state.alphabet, Alphabet::A1);
        assert_eq!(state.shifted, ShiftState::Disabled);

        shift(&mut state, ShiftState::Down, ShiftType::Permanent);
        assert_eq!(state.alphabet, Alphabet::A0);
        assert_eq!(state.shifted, ShiftState::Disabled);

        shift(&mut state, ShiftState::Down, ShiftType::Permanent);
        assert_eq!(state.alphabet, Alphabet::A2);
        assert_eq!(state.shifted, ShiftState::Disabled);

        // Test temporary shift
        state = State {
            version: 1,
            alphabet: Alphabet::A2,
            shifted: ShiftState::Disabled,
            ten_bit_zscii_mode: false,
            ten_bit_zscii_word: None,
        };
        assert_eq!(state.version, 1);
        assert_eq!(state.alphabet, Alphabet::A2);
        assert_eq!(state.shifted, ShiftState::Disabled);

        shift(&mut state, ShiftState::Down, ShiftType::Temporary);
        assert_eq!(state.alphabet, Alphabet::A2);
        assert_eq!(state.shifted, ShiftState::Down);

        state = State {
            version: 1,
            alphabet: Alphabet::A1,
            shifted: ShiftState::Disabled,
            ten_bit_zscii_mode: false,
            ten_bit_zscii_word: None,
        };
        assert_eq!(state.version, 1);
        assert_eq!(state.alphabet, Alphabet::A1);
        assert_eq!(state.shifted, ShiftState::Disabled);

        shift(&mut state, ShiftState::Down, ShiftType::Temporary);
        assert_eq!(state.alphabet, Alphabet::A1);
        assert_eq!(state.shifted, ShiftState::Down);

        state = State {
            version: 1,
            alphabet: Alphabet::A0,
            shifted: ShiftState::Disabled,
            ten_bit_zscii_mode: false,
            ten_bit_zscii_word: None,
        };
        assert_eq!(state.version, 1);
        assert_eq!(state.alphabet, Alphabet::A0);
        assert_eq!(state.shifted, ShiftState::Disabled);

        shift(&mut state, ShiftState::Down, ShiftType::Temporary);
        assert_eq!(state.alphabet, Alphabet::A0);
        assert_eq!(state.shifted, ShiftState::Down);
    }

    // TODO: Add tests for decode_char

    #[test]
    fn test_get_character_works() {
        let alphabet = Alphabet::A0;

        let a0: [u8; 26] = [
            0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6a, 0x6b, 0x6c, 0x6d, 0x6e,
            0x6f, 0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78, 0x79, 0x7a,
        ];

        for i in 0x06..=0x1f {
            let c = get_character(&alphabet, i);
            assert_eq!(c, Ok(a0[i as usize - 6]));
        }

        let a1: [u8; 26] = [
            0x41, 0x42, 0x43, 0x44, 0x45, 0x46, 0x47, 0x48, 0x49, 0x4a, 0x4b, 0x4c, 0x4d, 0x4e,
            0x4f, 0x50, 0x51, 0x52, 0x53, 0x54, 0x55, 0x56, 0x57, 0x58, 0x59, 0x5a,
        ];

        let alphabet = Alphabet::A1;

        for i in 0x06..=0x1f {
            let c = get_character(&alphabet, i);
            assert_eq!(c, Ok(a1[i as usize - 6]));
        }

        let alphabet = Alphabet::A2;

        let a2: [u8; 25] = [
            0x0d, 0x30, 0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39, 0x2e, 0x2c, 0x21,
            0x3f, 0x5f, 0x23, 0x60, 0x22, 0x2f, 0x5c, 0x2d, 0x3a, 0x28, 0x29,
        ];

        for i in 0x07..=0x1f {
            let c = get_character(&alphabet, i);
            assert_eq!(c, Ok(a2[i as usize - 7]));
        }
    }

    #[test]
    fn zscii_with_config_works() {
        let mut zscii_config = ZsciiConfig::load().expect("Error loading config file");
        zscii_config.zmachine_version = Some(3);
        let ps = ZsciiString::new_with_config(&zscii_config, 4, [0x65, 0xae, 0xa9, 0x65]);

        let mut s: String = String::from(ps);

        assert_eq!(s.len(), 5);
        assert_eq!(s.pop().unwrap(), 'f');
        assert_eq!(s.pop().unwrap(), 'e');
        assert_eq!(s.pop().unwrap(), 'i');
        assert_eq!(s.pop().unwrap(), 'h');
        assert_eq!(s.pop().unwrap(), 't');
    }

    // Tests for encoding from Unicode to ZSCII and Z-characters

    // #[test]
    // fn zscii_from_unicode_lowercase_works()  {
    //     let lowercase = "abcdefghijklmnopqrstuvwxyz";
    //     let s: String = String::from(lowercase);

    //     let expected: [u8; 26] = [ 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b,
    // 				   0x0c, 0x0d, 0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14,
    // 				   0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d,
    // 				   0x1e, 0x1f ];

    //     let zscii_config = ZsciiConfig::load().expect("Error loading config");

    //     let zs = ZsciiString::<26>::from_str_with_config(&s, &zscii_config);

    //     assert_eq!(zs.len(), 26);
    //     assert_eq!(zs.data, expected);

    //     let s: String = String::from(ps);

    //     assert_eq!(s, lowercase);
    // }
}
