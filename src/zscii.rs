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

use std::{
    ops::{Deref, DerefMut},
    sync::RwLock,
};

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
///
/// NOTE: The bytes in this string are NOT direct ZSCII characters.
/// They are sequences of three ZSCII characters encoded into two
/// bytes.
///
/// At various points in this crate we use the terms ZSCII and
/// Z-Character.  If there can be confusion about what the meaning of
/// a certain data structure or function operates on or returns,
/// additional documentation is included.
#[derive(Clone, Copy, Debug)]
pub struct ZsciiString<'config, const L: usize> {
    /// The configuration data for this ZSCII String
    /// This includes things like the Z-Machine version and Unicode
    /// table.
    pub config: &'config ZsciiConfig,

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

    /// There was no output vector to return
    NoOutput,
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

    /// Alphabet A2 characters to Unicode
    pub zscii_alphabet_a2_to_unicode: Map<String, Value>,

    /// Unicode to Alphabet A2 characters
    pub zscii_unicode_to_alphabet_a2: Map<String, Value>,
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
    /// Character 6 is used to switch to 10-bit mode, Character 7 is a
    /// carriage return / newline
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
#[derive(Clone, Debug, Eq, PartialEq)]
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
    /// The output vector to write data to
    pub output: Option<Vec<u8>>,
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
            output: None,
        }
    }

    /// Create a new State with a given Z-Machine version and set to a
    /// specific alphabet.
    #[allow(dead_code)] // Ignore because this is used in tests
    fn new_with_version_and_alphabet(version: u8, alphabet: Alphabet) -> State {
        State {
            version,
            alphabet,
            shifted: ShiftState::Disabled,
            ten_bit_zscii_mode: false,
            ten_bit_zscii_word: None,
            output: None,
        }
    }

    /// Set the output vector
    ///
    /// The output vector is written to if necessary when the state is
    /// changed.
    fn set_output(&mut self, output: Vec<u8>) {
        self.output = Some(output);
    }

    /// Write to the output vector directly.
    ///
    /// This can be used by functions outside the State implementation
    /// to write to the output vector.
    // TODO: Refactor this.
    // It really shouldn't be a function on State,
    // but it works for now and it's better than it was.  Now we're
    // driving shift character output by state transitions.
    fn write_output(&mut self, byte: u8) {
        if let Some(ref mut o) = self.output {
            o.push(byte);
        }
    }

    /// Return the output vector if there is one, otherwise return a
    /// NoOutput Error.
    ///
    /// This takes the vector from the output Option, replacing it
    /// with a None.  Any further attempts to write to the output will
    /// fail.
    ///
    /// It should be used when returning from a parser function.
    ///
    /// Maybe we can simplify this even more by not wrapping the Vec
    /// in a Some.
    fn return_output(&mut self) -> Result<Option<Vec<u8>>, Error> {
        self.output.take().map(Some).ok_or(Error::NoOutput)
    }

    /// Shift the alphabet in the current state up, either temporarily or
    /// permanently.
    /// Returns the new state with the changes applied.
    /// Changes the State
    fn shift_up(&mut self, shift_type: ShiftType) -> &mut Self {
        match shift_type {
            ShiftType::Temporary => {
                self.shifted = ShiftState::Up;
                if let Some(ref mut o) = self.output {
                    match self.version {
                        0 => {}
                        1..=2 => {
                            o.push(0x02);
                        }
                        3..=u8::MAX => {
                            o.push(0x04);
                        }
                    }
                }
            }
            ShiftType::Permanent => {
                self.alphabet = shift_alphabet(self.alphabet, ShiftState::Up);
                if let Some(ref mut o) = self.output {
                    match self.version {
                        0 => {}
                        1..=2 => {
                            o.push(0x04);
                        }
                        3..=u8::MAX => {}
                    }
                }
            }
        }

        self
    }

    /// Shift the alphabet in the current state down, either temporarily or
    /// permanently.
    /// Returns the new state with the changes applied
    /// Changes the State
    fn shift_down(&mut self, shift_type: ShiftType) -> &mut Self {
        match shift_type {
            ShiftType::Temporary => {
                self.shifted = ShiftState::Down;
                if let Some(ref mut o) = self.output {
                    match self.version {
                        0 => {}
                        1..=2 => {
                            o.push(0x03);
                        }
                        3..=u8::MAX => {
                            o.push(0x05);
                        }
                    }
                }
            }
            ShiftType::Permanent => {
                self.alphabet = shift_alphabet(self.alphabet, ShiftState::Down);
                if let Some(ref mut o) = self.output {
                    match self.version {
                        0 => {}
                        1..=2 => {
                            o.push(0x05);
                        }
                        3..=u8::MAX => {}
                    }
                }
            }
        }

        self
    }

    /// Shift the alphabet in the current state up or down, either
    /// temporarily or permanently.
    /// Returns the new state with the changes applied
    /// Changes the State
    fn shift(&mut self, shift_state: ShiftState, shift_type: ShiftType) -> &mut Self {
        match shift_state {
            ShiftState::Disabled => self,
            ShiftState::Up => self.shift_up(shift_type),
            ShiftState::Down => self.shift_down(shift_type),
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
fn decode_ten_bit_zchar(config: &ZsciiConfig, ten_bit_zchar: u16) -> Result<Option<u8>, Error> {
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
        155..=251 => decode_unicode_table_character(config, ten_bit_zchar),
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

// Decode a 10-bit ZSCII character that is in the Unicode table range
fn decode_unicode_table_character(
    config: &ZsciiConfig,
    ten_bit_zchar: u16,
) -> Result<Option<u8>, Error> {
    println!("Decoding ten-bit unicode table character");

    let extra_chars_map = &config.zscii_extra_characters_to_unicode_default_table;

    let key = ten_bit_zchar.to_string();
    if !extra_chars_map.contains_key(&key) {
        return Err(Error::InvalidCharacter);
    }
    let code_opt = extra_chars_map.get(&key);
    let code = match code_opt {
        Some(s) => s,
        None => {
            println!("get failed't find key: {:?}", key);
            return Err(Error::InvalidCharacter);
        }
    };
    let code_value_res = match code {
        serde_json::Value::Number(v) => v.as_u64().unwrap() as u8,
        _ => {
            println!("serde_json convert ugh: {:?}", code);
            return Err(Error::InvalidCharacter);
        }
    };

    Ok(Some(code_value_res))
}

// Encoding functions from Unicode to ZSCII

fn encode_alphabet_a2_char(
    state: &mut State,
    config: &ZsciiConfig,
    c: char,
    next_char: Option<char>,
) -> Result<Option<Vec<u8>>, Error> {
    let unicode_code_point = u32::from(c).to_string();

    // Unicode to Alphabet A2 characters
    let unicode_to_alphabet_a2 = &config.zscii_unicode_to_alphabet_a2;
    let output: Vec<u8> = Vec::new();

    state.set_output(output);

    let res = unicode_to_alphabet_a2
        .get(&unicode_code_point)
        .map(|value| value.as_u64().unwrap() as u8);

    match res {
        Some(r) => match state.version {
            0 => Err(Error::Unimplemented),
            1..=2 => {
                let next_char_in_a2: bool = if let Some(next) = next_char {
                    let next_unicode_code_point = u32::from(next).to_string();
                    unicode_to_alphabet_a2.contains_key(&next_unicode_code_point)
                } else {
                    false
                };

                match state.alphabet {
                    Alphabet::A0 => {
                        if next_char_in_a2 {
                            state.shift(ShiftState::Down, ShiftType::Permanent);
                        } else {
                            state.shift(ShiftState::Down, ShiftType::Temporary);
                        }
                    }
                    Alphabet::A1 => {
                        if next_char_in_a2 {
                            state.shift(ShiftState::Up, ShiftType::Permanent);
                        } else {
                            state.shift(ShiftState::Up, ShiftType::Temporary);
                        }
                    }
                    Alphabet::A2 => {}
                }
                state.write_output(r);
                state.return_output()
            }
            3..=u8::MAX => {
                // Version 3 of the Z-Machine only have temporary shifts
                // They are normally in A0
                state.shift(ShiftState::Down, ShiftType::Temporary);
                state.write_output(r);
                state.return_output()
            }
        },
        None => Ok(None),
    }
}

// TODO: Reduce the complexity of this function by splitting it up
// into smaller ones.  There's a lot of redundancy that can be reduced
// here.
fn encode_char(
    config: &ZsciiConfig,
    state: &mut State,
    c: char,
    next_char: Option<char>,
) -> Result<Option<Vec<u8>>, Error> {
    let unicode_code_point = u32::from(c);

    let mut output: Vec<u8> = Vec::new();

    let uc_map = &config.zscii_unicode_to_extra_characters_default_table;

    // We should probably explicitly list these in the match cases
    // Mixing hash lookups and matches works but seems weird
    let a2_option = encode_alphabet_a2_char(state, config, c, next_char)?;
    if let Some(a2) = a2_option {
        output = a2;
        return Ok(Some(output));
    }

    state.set_output(output);

    let d = match unicode_code_point {
        0x00..=0x40 => Err(Error::Unimplemented),

        // Uppercase characters
        0x41..=0x5a => match state.version {
            0 => Err(Error::Unimplemented),
            // Z-Machine Versions 1 and 2 have a permanent shift
            // character
            1..=2 => {
                // We are already in the correct state
                if state.alphabet == Alphabet::A1 {
                    state.write_output((unicode_code_point - 0x3b) as u8);
                    return state.return_output();
                }

                if let Some(next) = next_char {
                    let next_unicode_code_point = u32::from(next);
                    if (0x41..=0x5a).contains(&next_unicode_code_point) {
                        // Next character is also uppercase
                        // Use a permanent shift
                        match state.alphabet {
                            Alphabet::A0 => {
                                state.shift(ShiftState::Up, ShiftType::Permanent);
                            }
                            Alphabet::A1 => {}
                            Alphabet::A2 => {
                                state.shift(ShiftState::Down, ShiftType::Permanent);
                            }
                        }
                        state.write_output((unicode_code_point - 0x3b) as u8);
                        return state.return_output();
                    }
                }

                // Next character is not uppercase or doesn't exist, use a
                // temporary shift
                match state.alphabet {
                    Alphabet::A0 => {
                        state.shift(ShiftState::Up, ShiftType::Temporary);
                    }
                    Alphabet::A1 => {}
                    Alphabet::A2 => {
                        state.shift(ShiftState::Down, ShiftType::Temporary);
                    }
                }
                state.write_output((unicode_code_point - 0x3b) as u8);
                state.return_output()
            }

            // Z-Machine Versions 3 and above don't have a permanent
            // shift character
            3..=u8::MAX => {
                // Shift to A1
                state.shift(ShiftState::Up, ShiftType::Temporary);
                state.write_output((unicode_code_point - 0x3b) as u8);
                state.return_output()
            }
        },
        0x5b..=0x60 => Err(Error::Unimplemented),
        // lowercase characters
        0x61..=0x7a => match state.version {
            0 => Err(Error::Unimplemented),
            // Z-Machine Versions 1 and 2 have a permanent shift
            // character
            1..=2 => {
                // We are already in the correct state
                if state.alphabet == Alphabet::A0 {
                    state.write_output((unicode_code_point - 0x5b) as u8);
                    return state.return_output();
                }
                if let Some(next) = next_char {
                    let next_unicode_code_point = u32::from(next);
                    if (next_unicode_code_point >= 0x61) || (next_unicode_code_point <= 0x7a) {
                        // Next character is also lowercase
                        // Use a permanent shift
                        match state.alphabet {
                            Alphabet::A0 => {}
                            Alphabet::A1 => {
                                state.shift(ShiftState::Down, ShiftType::Permanent);
                            }
                            Alphabet::A2 => {
                                state.shift(ShiftState::Up, ShiftType::Permanent);
                            }
                        }
                        state.write_output((unicode_code_point - 0x5b) as u8);
                        return state.return_output();
                    }

                    // Next character is not lowercase or doesn't exist, use a
                    // temporary shift
                    match state.alphabet {
                        Alphabet::A0 => {}
                        Alphabet::A1 => {
                            state.shift(ShiftState::Down, ShiftType::Temporary);
                        }
                        Alphabet::A2 => {
                            state.shift(ShiftState::Up, ShiftType::Temporary);
                        }
                    }
                }
                state.write_output((unicode_code_point - 0x3b) as u8);
                state.return_output()
            }
            3..=u8::MAX => {
                // normal state is A0
                state.write_output((unicode_code_point - 0x5b) as u8);
                state.return_output()
            }
        },
        0x7b..=0x9f => Err(Error::Unimplemented),
        0xa0..=0xff => {
            // Shift first
            if !state.ten_bit_zscii_mode {
                // We need to be in alphabet A2 to enable ten bit ZSCII mode
                if state.alphabet == Alphabet::A2 {
                    // Just switch to ten bit ZSCII mode
                    state.write_output(0x06);
                } else {
                    // I know this is a state machine.  I'm reducing
                    // the complexity and refactoring input/output
                    // into state transitions gradually.
                    //
                    // We now output shift state transitions, and
                    // we'll do ten-bit transitions next.
                    match state.version {
                        0 => return Err(Error::Unimplemented),
                        // Z-Machine Versions 1 and 2 have a permanent shift
                        // character
                        1..=2 => {
                            let next_char_in_a2: bool = if let Some(next) = next_char {
                                let next_unicode_code_point = u32::from(next).to_string();
                                uc_map.contains_key(&next_unicode_code_point)
                            } else {
                                false
                            };
                            println!("Next character is in a2");
                            // Next character is not uppercase or doesn't exist, use a
                            // temporary shift
                            match state.alphabet {
                                Alphabet::A0 => {
                                    if next_char_in_a2 {
                                        state.shift(ShiftState::Down, ShiftType::Permanent);
                                    } else {
                                        state.shift(ShiftState::Down, ShiftType::Temporary);
                                    }
                                }
                                Alphabet::A1 => {
                                    if next_char_in_a2 {
                                        state.shift(ShiftState::Up, ShiftType::Permanent);
                                    } else {
                                        state.shift(ShiftState::Up, ShiftType::Temporary);
                                    }
                                }
                                Alphabet::A2 => {
                                    println!("Already in A2");
                                }
                            }
                        }
                        // Z-Machine Versions 3 and above don't have a permanent
                        // shift character
                        3..=u8::MAX => {
                            // Shift to A2
                            state.shift(ShiftState::Down, ShiftType::Temporary);
                        }
                    }
                    // Maybe add specific method for changing ten-byte
                    // state
                    state.write_output(0x06);
                }
            }
            println!("test");
            let key = unicode_code_point.to_string();
            println!("test2");
            if !uc_map.contains_key(&key) {
                println!("Couldn't find key: {:?}", key);
                return Err(Error::InvalidCharacter);
            }
            let code_opt = uc_map.get(&key);
            let code_value = match code_opt {
                Some(s) => s,
                None => {
                    println!("get failed't find key: {:?}", key);
                    return Err(Error::InvalidCharacter);
                }
            };

            let code = match code_value {
                serde_json::Value::Number(v) => v.as_u64().unwrap() as u32,
                _ => {
                    return Err(Error::InvalidCharacter);
                }
            };

            // Top 5 bits flag:   0b1111100000
            let top_bits = (code & 0x3e0) >> 5;
            state.write_output(top_bits.try_into().unwrap());
            // lower 5 bits flag: 0b0000011111
            let lower_bits = code & 0x1f;
            state.write_output(lower_bits.try_into().unwrap());

            state.return_output()
        }
        0x100..=u32::MAX => Err(Error::Unimplemented),
    };

    d
}

fn unicode_to_zscii_bytes(config: &ZsciiConfig, state: &mut State, s: &str) -> ZCharacterString {
    let bytes: Vec<u8> = s
        .chars()
        .enumerate()
        .filter_map(|(idx, c)| {
            let chars_opt: Option<Vec<u8>> =
                encode_char(config, state, c, s.chars().nth(idx + 1)).expect("Should be Ok result");
            chars_opt
        })
        .flatten()
        .collect();
    ZCharacterString(bytes)
}

impl<'config, const L: usize> ZsciiString<'config, L> {
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
    pub fn new_with_config(config: &'config ZsciiConfig, len: u32, data: [u8; L]) -> Self {
        ZsciiString { config, len, data }
    }

    /// Create a ZSCII String from a string slice
    ///
    /// # Examples
    ///
    /// ```
    /// use forbidden_bands::zscii::{ZsciiConfig, ZsciiString};
    ///
    /// let lowercase = "if";
    /// let s: String = String::from(lowercase);
    ///
    /// let expected: [u8; 2] = [0xb9, 0x65];
    ///
    /// let mut zscii_config = ZsciiConfig::load().expect("Error loading config");
    /// zscii_config.zmachine_version = Some(1);
    ///
    /// let zs = ZsciiString::<2>::from_str_with_config(&zscii_config, &s);
    ///
    /// assert_eq!(zs.len, 2);
    /// assert_eq!(zs.data, expected);
    ///
    /// let s: String = String::from(zs);
    ///
    /// assert_eq!(s, lowercase);
    /// ```
    pub fn from_str_with_config(config: &'config ZsciiConfig, s: &str) -> ZsciiString<'config, L> {
        let mut final_bytes: [u8; L] = [0; L];

        let mut state = State::new(config.zmachine_version.unwrap());

        let bytes: ZCharacterString = unicode_to_zscii_bytes(config, &mut state, s);

        let mut zcharacter_bytes: Vec<u8> = Vec::new();

        bytes.into_iter().for_each(|b| {
            zcharacter_bytes.push(b[0]);
            zcharacter_bytes.push(b[1]);
        });

        if zcharacter_bytes.len() > L {
            panic!("u8 vector is too large");
        }
        let b = zcharacter_bytes.as_slice();

        final_bytes[..b.len()].copy_from_slice(&b[..b.len()]);

        ZsciiString {
            config,
            len: b.len() as u32,
            data: final_bytes,
        }
    }
}

/// An IntoIter structure for ZsciiStrings
/// We need to keep track of the index of the current element, along
/// with the data.
///
/// TODO: We may need to keep track of the end-of-text marker which is
/// the most-significant bit in each 2-byte word
pub struct IntoIter<'config, const L: usize> {
    index: usize,
    data: ZsciiString<'config, L>,
    /// The MSB marker used to indicate end-of-text
    end_of_text_marker: bool,
    chars_to_send: Vec<u8>,
}

// Two normal bytes (16-bits) contain three Z-characters
// Each ZSCII character code is a 5-bit value.
//
// This iterator will feed 5-bits at a time to the user.
// It is then up to the other logic to decode those 5-bits
impl<'config, const L: usize> IntoIterator for ZsciiString<'config, L> {
    type Item = u8;
    type IntoIter = IntoIter<'config, L>;
    fn into_iter(self) -> IntoIter<'config, L> {
        IntoIter {
            index: 0,
            data: self,
            end_of_text_marker: false,
            chars_to_send: Vec::new(),
        }
    }
}

impl<const L: usize> IntoIter<'_, L> {
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
impl<const L: usize> Iterator for IntoIter<'_, L> {
    type Item = u8;
    fn next(&mut self) -> Option<Self::Item> {
        // Check first_byte, because we want to return the whole three
        // characters and only finish on the next 2-byte sequence
        if self.chars_to_send.is_empty() {
            // if self.end_of_text_marker && self.first_byte() {
            // 	return None;
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

struct ZCharacterString(Vec<u8>);

impl Deref for ZCharacterString {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ZCharacterString {
    fn deref_mut(&mut self) -> &mut Vec<u8> {
        &mut self.0
    }
}

/// An iterator structure for Z-characters
/// We need to keep track of the index of the current element, along
/// with the data.
///
/// TODO: We may need to keep track of the end-of-text marker which is
/// the most-significant bit in each 2-byte word
pub struct IntoZCharactersIter {
    index: usize,
    data: ZCharacterString,
}

// Two normal bytes (16-bits) contain three Z-characters
// Each ZSCII character code is a 5-bit value.
//
// This iterator will feed 5-bits at a time to the user.
// It is then up to the other logic to decode those 5-bits
impl IntoIterator for ZCharacterString {
    type Item = [u8; 2];
    type IntoIter = IntoZCharactersIter;
    fn into_iter(self) -> IntoZCharactersIter {
        IntoZCharactersIter {
            index: 0,
            data: self,
        }
    }
}

impl IntoZCharactersIter {
    fn get_next_bytes(&mut self) -> [u8; 2] {
        let char1 = if self.index < self.data.len() {
            self.data[{
                let tmp = self.index;
                self.index += 1;
                tmp
            }]
        } else {
            0x05
        };
        let char2 = if self.index < self.data.len() {
            self.data[{
                let tmp = self.index;
                self.index += 1;
                tmp
            }]
        } else {
            0x05
        };
        let char3 = if self.index < self.data.len() {
            self.data[{
                let tmp = self.index;
                self.index += 1;
                tmp
            }]
        } else {
            0x05
        };

        let end_of_list = self.index >= self.data.len();

        let first_bit = if end_of_list { 0b10000000 } else { 0b00000000 };

        let first_byte = first_bit | ((char1 & 0b00011111) << 2) | ((char2 & 0b00011000) >> 3);
        let second_byte = ((char2 & 0b00000111) << 5) | (char3 & 0b00011111);

        // TODO: Figure if there is a way to feed these characters
        // individually without state

        // The characters are added in backwards order here so we can
        // dequeue them using a standard Vec and pop()
        [first_byte, second_byte]
    }
}

impl Iterator for IntoZCharactersIter {
    type Item = [u8; 2];
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.data.len() {
            None
        } else {
            Some(self.get_next_bytes())
        }
    }
}

/// This function decodes a 5-bit z-character
fn decode_char(state: &mut State, c: u8) -> Option<char> {
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
                    state.shift(ShiftState::Up, ShiftType::Temporary);
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
                    state.shift(ShiftState::Down, ShiftType::Temporary);
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
                    state.shift(ShiftState::Up, ShiftType::Permanent);
                    None
                }
                // Versions 3 and 4 use character four as a temporary
                // shift up for one character only
                3..=4 => {
                    state.shift(ShiftState::Up, ShiftType::Temporary);
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
                    state.shift(ShiftState::Down, ShiftType::Permanent);
                    None
                }
                // Versions 3 and 4 use character four as a temporary
                // shift down for one character only
                3..=4 => {
                    state.shift(ShiftState::Down, ShiftType::Temporary);
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
                        .unwrap_or_else(|_| panic!("Should decode character: {:?} {}", alphabet, c))
                        .into(),
                ),
                Alphabet::A1 => {
                    char::from_u32(
                        get_character(&alphabet, c)
                            .unwrap_or_else(|_| {
                                panic!("Should decode character: {:?} {}", alphabet, c)
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
                    .unwrap_or_else(|_| panic!("Should decode character: {:?} {}", alphabet, c))
                    .into(),
            )
        }
        _ => None,
    };

    d
}

impl<const L: usize> From<&ZsciiString<'_, L>> for String {
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
        let mut state = State::new(
            s.config
                .zmachine_version
                .expect("The Z-Machine config should have a version"),
        );

        s.into_iter()
            .filter_map(|c| {
                // the ZsciiString iterator feeds us 5-bit words, we
                // don't need to worry about the encoding of those
                // words into bytes.
                if state.ten_bit_zscii_mode {
                    // Start decoding a ten-bit ZSCII character
                    if let Some(high_bits) = state.ten_bit_zscii_word {
                        let ten_bit_zchar: u16 = ((high_bits << 5) | c).into();
                        let decode_ten_bit_zchar_result =
                            decode_ten_bit_zchar(s.config, ten_bit_zchar);
                        // Exit out of ten-bit ZSCII mode after
                        // reading one ten-bit ZSCII character
                        state.ten_bit_zscii_mode = false;

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

impl<const L: usize> From<ZsciiString<'_, L>> for String {
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
        get_character, shift_alphabet, shift_alphabet_down, shift_alphabet_up, Alphabet,
        ShiftState, ShiftType, State, ZsciiConfig, ZsciiString,
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
        let mut state = State::new(1);

        assert_eq!(state.version, 1);
        assert_eq!(state.alphabet, Alphabet::A0);
        assert_eq!(state.shifted, ShiftState::Disabled);

        // Test permanent shift

        state.shift_up(ShiftType::Permanent);
        assert_eq!(state.alphabet, Alphabet::A1);
        assert_eq!(state.shifted, ShiftState::Disabled);

        state.shift_up(ShiftType::Permanent);
        assert_eq!(state.alphabet, Alphabet::A2);
        assert_eq!(state.shifted, ShiftState::Disabled);

        state.shift_up(ShiftType::Permanent);
        assert_eq!(state.alphabet, Alphabet::A0);
        assert_eq!(state.shifted, ShiftState::Disabled);

        // Test temporary shift
        let mut state = State::new(1);
        assert_eq!(state.version, 1);
        assert_eq!(state.alphabet, Alphabet::A0);
        assert_eq!(state.shifted, ShiftState::Disabled);

        state.shift_up(ShiftType::Temporary);
        assert_eq!(state.alphabet, Alphabet::A0);
        assert_eq!(state.shifted, ShiftState::Up);

        state = State::new_with_version_and_alphabet(1, Alphabet::A1);
        assert_eq!(state.version, 1);
        assert_eq!(state.alphabet, Alphabet::A1);
        assert_eq!(state.shifted, ShiftState::Disabled);

        state.shift_up(ShiftType::Temporary);
        assert_eq!(state.alphabet, Alphabet::A1);
        assert_eq!(state.shifted, ShiftState::Up);

        state = State::new_with_version_and_alphabet(1, Alphabet::A2);
        assert_eq!(state.version, 1);
        assert_eq!(state.alphabet, Alphabet::A2);
        assert_eq!(state.shifted, ShiftState::Disabled);

        state.shift_up(ShiftType::Temporary);
        assert_eq!(state.alphabet, Alphabet::A2);
        assert_eq!(state.shifted, ShiftState::Up);
    }

    #[test]
    fn shift_down_works() {
        let mut state = State::new_with_version_and_alphabet(1, Alphabet::A2);

        assert_eq!(state.version, 1);
        assert_eq!(state.alphabet, Alphabet::A2);
        assert_eq!(state.shifted, ShiftState::Disabled);

        // Test permanent shift

        state.shift_down(ShiftType::Permanent);
        assert_eq!(state.alphabet, Alphabet::A1);
        assert_eq!(state.shifted, ShiftState::Disabled);

        state.shift_down(ShiftType::Permanent);
        assert_eq!(state.alphabet, Alphabet::A0);
        assert_eq!(state.shifted, ShiftState::Disabled);

        state.shift_down(ShiftType::Permanent);
        assert_eq!(state.alphabet, Alphabet::A2);
        assert_eq!(state.shifted, ShiftState::Disabled);

        // Test temporary shift
        state = State::new_with_version_and_alphabet(1, Alphabet::A2);
        assert_eq!(state.version, 1);
        assert_eq!(state.alphabet, Alphabet::A2);
        assert_eq!(state.shifted, ShiftState::Disabled);

        state.shift_down(ShiftType::Temporary);
        assert_eq!(state.alphabet, Alphabet::A2);
        assert_eq!(state.shifted, ShiftState::Down);

        state = State::new_with_version_and_alphabet(1, Alphabet::A1);
        assert_eq!(state.version, 1);
        assert_eq!(state.alphabet, Alphabet::A1);
        assert_eq!(state.shifted, ShiftState::Disabled);

        state.shift_down(ShiftType::Temporary);
        assert_eq!(state.alphabet, Alphabet::A1);
        assert_eq!(state.shifted, ShiftState::Down);

        state = State::new(1);
        assert_eq!(state.version, 1);
        assert_eq!(state.alphabet, Alphabet::A0);
        assert_eq!(state.shifted, ShiftState::Disabled);

        state.shift_down(ShiftType::Temporary);
        assert_eq!(state.alphabet, Alphabet::A0);
        assert_eq!(state.shifted, ShiftState::Down);
    }

    #[test]
    fn shift_works() {
        // Test shift up works
        let mut state = State::new(1);

        assert_eq!(state.version, 1);
        assert_eq!(state.alphabet, Alphabet::A0);
        assert_eq!(state.shifted, ShiftState::Disabled);

        // Test permanent shift

        state.shift(ShiftState::Up, ShiftType::Permanent);
        assert_eq!(state.alphabet, Alphabet::A1);
        assert_eq!(state.shifted, ShiftState::Disabled);

        state.shift(ShiftState::Up, ShiftType::Permanent);
        assert_eq!(state.alphabet, Alphabet::A2);
        assert_eq!(state.shifted, ShiftState::Disabled);

        state.shift(ShiftState::Up, ShiftType::Permanent);
        assert_eq!(state.alphabet, Alphabet::A0);
        assert_eq!(state.shifted, ShiftState::Disabled);

        // Test temporary shift
        state = State::new(1);
        assert_eq!(state.version, 1);
        assert_eq!(state.alphabet, Alphabet::A0);
        assert_eq!(state.shifted, ShiftState::Disabled);

        state.shift(ShiftState::Up, ShiftType::Temporary);
        assert_eq!(state.alphabet, Alphabet::A0);
        assert_eq!(state.shifted, ShiftState::Up);

        state = State::new_with_version_and_alphabet(1, Alphabet::A1);
        assert_eq!(state.version, 1);
        assert_eq!(state.alphabet, Alphabet::A1);
        assert_eq!(state.shifted, ShiftState::Disabled);

        state.shift(ShiftState::Up, ShiftType::Temporary);
        assert_eq!(state.alphabet, Alphabet::A1);
        assert_eq!(state.shifted, ShiftState::Up);

        state = State::new_with_version_and_alphabet(1, Alphabet::A2);
        assert_eq!(state.version, 1);
        assert_eq!(state.alphabet, Alphabet::A2);
        assert_eq!(state.shifted, ShiftState::Disabled);

        state.shift(ShiftState::Up, ShiftType::Temporary);
        assert_eq!(state.alphabet, Alphabet::A2);
        assert_eq!(state.shifted, ShiftState::Up);

        // Test shift down

        let mut state = State::new_with_version_and_alphabet(1, Alphabet::A2);

        assert_eq!(state.version, 1);
        assert_eq!(state.alphabet, Alphabet::A2);
        assert_eq!(state.shifted, ShiftState::Disabled);

        // Test permanent shift

        state.shift(ShiftState::Down, ShiftType::Permanent);
        assert_eq!(state.alphabet, Alphabet::A1);
        assert_eq!(state.shifted, ShiftState::Disabled);

        state.shift(ShiftState::Down, ShiftType::Permanent);
        assert_eq!(state.alphabet, Alphabet::A0);
        assert_eq!(state.shifted, ShiftState::Disabled);

        state.shift(ShiftState::Down, ShiftType::Permanent);
        assert_eq!(state.alphabet, Alphabet::A2);
        assert_eq!(state.shifted, ShiftState::Disabled);

        // Test temporary shift
        state = State::new_with_version_and_alphabet(1, Alphabet::A2);
        assert_eq!(state.version, 1);
        assert_eq!(state.alphabet, Alphabet::A2);
        assert_eq!(state.shifted, ShiftState::Disabled);

        state.shift(ShiftState::Down, ShiftType::Temporary);
        assert_eq!(state.alphabet, Alphabet::A2);
        assert_eq!(state.shifted, ShiftState::Down);

        state = State::new_with_version_and_alphabet(1, Alphabet::A1);
        assert_eq!(state.version, 1);
        assert_eq!(state.alphabet, Alphabet::A1);
        assert_eq!(state.shifted, ShiftState::Disabled);

        state.shift(ShiftState::Down, ShiftType::Temporary);
        assert_eq!(state.alphabet, Alphabet::A1);
        assert_eq!(state.shifted, ShiftState::Down);

        state = State::new(1);
        assert_eq!(state.version, 1);
        assert_eq!(state.alphabet, Alphabet::A0);
        assert_eq!(state.shifted, ShiftState::Disabled);

        state.shift(ShiftState::Down, ShiftType::Temporary);
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

    #[test]
    fn zscii_from_unicode_works() {
        let lowercase = "thief";
        let s: String = String::from(lowercase);

        let expected: [u8; 4] = [0x65, 0xae, 0xa9, 0x65];

        let mut zscii_config = ZsciiConfig::load().expect("Error loading config");
        zscii_config.zmachine_version = Some(3);

        let zs = ZsciiString::<4>::from_str_with_config(&zscii_config, &s);

        assert_eq!(zs.len, 4);
        assert_eq!(zs.data, expected);

        let s: String = String::from(zs);

        assert_eq!(s, lowercase);
    }

    #[test]
    fn zscii_version_1_from_unicode_lowercase_a_uppercase_a_works() {
        // "aA" is ASCII hex [0x61, 0x41], binary [0b01100001, 0b01000001]
        // ZSCII: [0x06, 0x02, 0x06], binary: [0b00110, 0b00010, 0b00110]
        // Z-Characters: [0b10011000, 0b01000110], hex: [0x98, 0x46]

        let lowercase = "aA";
        let s: String = String::from(lowercase);

        let expected: [u8; 2] = [0x98, 0x46];

        let mut zscii_config = ZsciiConfig::load().expect("Error loading config");
        zscii_config.zmachine_version = Some(1);

        let zs = ZsciiString::<2>::from_str_with_config(&zscii_config, &s);

        assert_eq!(zs.len, 2);
        assert_eq!(zs.data, expected);

        let s: String = String::from(zs);

        assert_eq!(s, lowercase);
    }

    #[test]
    fn zscii_version_1_from_unicode_uppercase_a_lowercase_a_works() {
        // "Aa" is ASCII hex [0x41, 0x61], binary [0b01000001, 0b01100001]
        // ZSCII: [0x02, 0x06, 0x06], binary: [0b00010, 0b00110, 0b00110]
        // Z-Characters: [0b10001000, 0b11000110], hex: [0x88, 0xc6]

        let lowercase = "Aa";
        let s: String = String::from(lowercase);

        let expected: [u8; 2] = [0x88, 0xc6];

        let mut zscii_config = ZsciiConfig::load().expect("Error loading config");
        zscii_config.zmachine_version = Some(1);

        let zs = ZsciiString::<2>::from_str_with_config(&zscii_config, &s);

        assert_eq!(zs.len, 2);
        assert_eq!(zs.data, expected);

        let s: String = String::from(zs);

        assert_eq!(s, lowercase);
    }

    #[test]
    fn zscii_version_1_from_unicode_uppercase_a_uppercase_a_works() {
        // "AA" is ASCII hex [0x41, 0x41], binary [0b01000001, 0b01000001]
        // ZSCII: [0x04, 0x06, 0x06], binary: [0b00100, 0b00110, 0b00110]
        // Z-Characters: [0b10010000, 0b11000110], hex: [0x90, 0xc6]

        let lowercase = "AA";
        let s: String = String::from(lowercase);

        let expected: [u8; 2] = [0x90, 0xc6];

        let mut zscii_config = ZsciiConfig::load().expect("Error loading config");
        zscii_config.zmachine_version = Some(1);

        let zs = ZsciiString::<2>::from_str_with_config(&zscii_config, &s);

        assert_eq!(zs.len, 2);
        assert_eq!(zs.data, expected);

        let s: String = String::from(zs);

        assert_eq!(s, lowercase);
    }

    #[test]
    fn zscii_version_3_from_unicode_lowercase_a_uppercase_a_works() {
        // "aA" is ASCII hex [0x61, 0x41], binary [0b01100001, 0b01000001]

        // Z-Machine versions 3 and above only have temporary shift
        // (0x04 and 0x05)
        // ZSCII: [0x06, 0x04, 0x06], binary: [0b00110, 0b00100, 0b00110]

        // Z-Characters: [0b10011000, 0b10000110], hex: [0x98, 0x86]

        let lowercase = "aA";
        let s: String = String::from(lowercase);

        let expected: [u8; 2] = [0x98, 0x86];

        let mut zscii_config = ZsciiConfig::load().expect("Error loading config");
        zscii_config.zmachine_version = Some(3);

        let zs = ZsciiString::<2>::from_str_with_config(&zscii_config, &s);

        assert_eq!(zs.len, 2);
        assert_eq!(zs.data, expected);

        let s: String = String::from(zs);

        assert_eq!(s, lowercase);
    }

    #[test]
    fn zscii_version_3_from_unicode_uppercase_a_lowercase_a_works() {
        // "Aa" is ASCII hex [0x41, 0x61], binary [0b01000001, 0b01100001]

        // Z-Machine versions 3 and above only have temporary shift
        // (0x04 and 0x05)
        // ZSCII: [0x04, 0x06, 0x06], binary: [0b00100, 0b00110, 0b00110]

        // Z-Characters: [0b10010000, 0b11000110], hex: [0x90, 0xc6]

        let lowercase = "Aa";
        let s: String = String::from(lowercase);

        let expected: [u8; 2] = [0x90, 0xc6];

        let mut zscii_config = ZsciiConfig::load().expect("Error loading config");
        zscii_config.zmachine_version = Some(3);

        let zs = ZsciiString::<2>::from_str_with_config(&zscii_config, &s);

        assert_eq!(zs.len, 2);
        assert_eq!(zs.data, expected);

        let s: String = String::from(zs);

        assert_eq!(s, lowercase);
    }

    #[test]
    fn zscii_version_3_from_unicode_uppercase_a_uppercase_a_works() {
        // "AA" is ASCII hex [0x41, 0x41], binary [0b01000001, 0b01000001]
        // Z-Machine versions 3 and above only have temporary shift
        // (0x04 and 0x05)
        // So we need two shift characters, extending it out to 4 Z-Characters
        // ZSCII: [0x04, 0x06, 0x04, 0x06], binary: [0b00100, 0b00110, 0b00100, 0b00110]

        // Z-Characters: binary: [0b00010000, 0b11000100, 0b10011000, 0b10100101],
        //               hex: [0x10, 0xc4, 0x98, 0xa5]

        let lowercase = "AA";
        let s: String = String::from(lowercase);

        let expected: [u8; 4] = [0x10, 0xc4, 0x98, 0xa5];

        let mut zscii_config = ZsciiConfig::load().expect("Error loading config");
        zscii_config.zmachine_version = Some(3);

        let zs = ZsciiString::<4>::from_str_with_config(&zscii_config, &s);

        assert_eq!(zs.len, 4);
        assert_eq!(zs.data, expected);

        let s: String = String::from(zs);

        assert_eq!(s, lowercase);
    }

    #[test]
    fn zscii_from_unicode_lowercase_works() {
        let lowercase = "abcdefghijklmnopqrstuvwxyz";
        let s: String = String::from(lowercase);

        let expected: [u8; 18] = [
            0x18, 0xe8, 0x25, 0x4b, 0x31, 0xae, 0x3e, 0x11, 0x4a, 0x74, 0x56, 0xd7, 0x63, 0x3a,
            0x6f, 0x9d, 0xfb, 0xe5,
        ];

        let mut zscii_config = ZsciiConfig::load().expect("Error loading config");
        zscii_config.zmachine_version = Some(3);

        let zs = ZsciiString::<18>::from_str_with_config(&zscii_config, &s);

        assert_eq!(zs.len, 18);
        assert_eq!(zs.data, expected);

        let s: String = String::from(zs);

        assert_eq!(s, lowercase);
    }

    /// The Z-Machine Versions 1 and 2 support permanent shift
    /// characters, so if there is a chain of more than one character
    /// in alphabet A1 or A2, we should use a permanent shift.
    #[test]
    fn zscii_version_1_from_unicode_uppercase_works() {
        let lowercase = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let s: String = String::from(lowercase);

        let expected: [u8; 18] = [
            0x10, 0xc7, 0x21, 0x2a, 0x2d, 0x8d, 0x39, 0xf0, 0x46, 0x53, 0x52, 0xb6, 0x5f, 0x19,
            0x6b, 0x7c, 0xf7, 0xdf,
        ];

        let mut zscii_config = ZsciiConfig::load().expect("Error loading config");
        zscii_config.zmachine_version = Some(1);

        let zs = ZsciiString::<18>::from_str_with_config(&zscii_config, &s);

        assert_eq!(zs.len, 18);
        assert_eq!(zs.data, expected);

        let s: String = String::from(zs);

        assert_eq!(s, lowercase);
    }

    /// The Z-Machine Versions 3 and above don't have a permanent shift
    /// character, so you need a separate shift before each shifted
    /// character.
    #[test]
    fn zscii_version_3_from_unicode_uppercase_works() {
        let lowercase = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let s: String = String::from(lowercase);

        let expected: [u8; 36] = [
            0x10, 0xc4, 0x1c, 0x88, 0x11, 0x24, 0x28, 0x8b, 0x11, 0x84, 0x34, 0x8e, 0x11, 0xe4,
            0x40, 0x91, 0x12, 0x44, 0x4c, 0x94, 0x12, 0xa4, 0x58, 0x97, 0x13, 0x4, 0x64, 0x9a,
            0x13, 0x64, 0x70, 0x9d, 0x13, 0xc4, 0xfc, 0xa5,
        ];

        let mut zscii_config = ZsciiConfig::load().expect("Error loading config");
        zscii_config.zmachine_version = Some(3);

        let zs = ZsciiString::<36>::from_str_with_config(&zscii_config, &s);

        assert_eq!(zs.len, 36);
        assert_eq!(zs.data, expected);

        let s: String = String::from(zs);

        assert_eq!(s, lowercase);
    }

    #[test]
    fn zscii_version_1_short_unicode_to_alphabet_a2_works() {
        // "\r)" is ASCII hex [0x0d, 0x29], binary [0b00001101, 0b00101001]
        // Z-Machine versions 1 and above have permanent and temporary shift
        // ZSCII: [0x05, 0x07, 0x1f], binary: [0b00101, 0b00111, 0b11111]
        // Z-Characters: [0b10010100, 0b11111111], hex: [0x94, 0xff]

        let chars = "\r)";
        let s: String = String::from(chars);

        let expected: [u8; 2] = [0x94, 0xff];

        let mut zscii_config = ZsciiConfig::load().expect("Error loading config");
        zscii_config.zmachine_version = Some(1);

        let zs = ZsciiString::<2>::from_str_with_config(&zscii_config, &s);

        assert_eq!(zs.len, 2);
        assert_eq!(zs.data, expected);

        let s: String = String::from(zs);

        assert_eq!(s, chars);
    }

    #[test]
    fn zscii_version_1_short_unicode_three_characters_to_alphabet_a2_works() {
        // "\r,)" is ASCII hex [0x0d, 0x2c, 0x29], binary [0b00001101, 0b00101100, 0b00101001]
        // Z-Machine versions 1 and above have permanent and temporary shift
        // ZSCII: [0x05, 0x07, 0x13, 0x1f], binary: [0b00010, 0b00111, 0b10011, 0b11111]
        // Z-Characters: [0b00010100, 0b11110011, 0b11111100, 0b10100101]
        // Z-Characters: hex: [0x14, 0xf3, 0xfc, 0xa5]

        let chars = "\r,)";
        let s: String = String::from(chars);

        let expected: [u8; 4] = [0x14, 0xf3, 0xfc, 0xa5];

        let mut zscii_config = ZsciiConfig::load().expect("Error loading config");
        zscii_config.zmachine_version = Some(1);

        let zs = ZsciiString::<4>::from_str_with_config(&zscii_config, &s);

        assert_eq!(zs.len, 4);
        assert_eq!(zs.data, expected);

        let s: String = String::from(zs);

        assert_eq!(s, chars);
    }

    #[test]
    fn zscii_version_3_short_unicode_to_alphabet_a2_works() {
        // "\r)" is ASCII hex [0x0d, 0x29], binary [0b00001101, 0b00101001]
        // Z-Machine versions 3 and above only have temporary shift
        // ZSCII: [0x05, 0x07, 0x05, 0x1f], binary: [0b00101, 0b00111, 0b00101, 0b11111]
        // Z-Characters: [0b00010100, 0b11100101, 0b11111100, 0b10100101]
        // Z-Characters: [0x14, 0xe5, 0xfc, 0xa5]

        let chars = "\r)";
        let s: String = String::from(chars);

        let expected: [u8; 4] = [0x14, 0xe5, 0xfc, 0xa5];

        let mut zscii_config = ZsciiConfig::load().expect("Error loading config");
        zscii_config.zmachine_version = Some(3);

        let zs = ZsciiString::<4>::from_str_with_config(&zscii_config, &s);

        assert_eq!(zs.len, 4);
        assert_eq!(zs.data, expected);

        let s: String = String::from(zs);

        assert_eq!(s, chars);
    }

    #[test]
    fn zscii_version_1_unicode_to_alphabet_a2_works() {
        let chars = "\r0123456789.,!?_#`\"/\\-:()";
        let s: String = String::from(chars);

        let expected: [u8; 18] = [
            0x14, 0xe8, 0x25, 0x4b, 0x31, 0xae, 0x3e, 0x11, 0x4a, 0x74, 0x56, 0xd7, 0x63, 0x3a,
            0x6f, 0x9d, 0xfb, 0xe5,
        ];

        let mut zscii_config = ZsciiConfig::load().expect("Error loading config");
        zscii_config.zmachine_version = Some(1);

        let zs = ZsciiString::<18>::from_str_with_config(&zscii_config, &s);

        assert_eq!(zs.len, 18);
        assert_eq!(zs.data, expected);

        let s: String = String::from(zs);

        assert_eq!(s, chars);
    }

    #[test]
    fn zscii_version_3_unicode_to_alphabet_a2_works() {
        let chars = "\r0123456789.,!?_#`\"/\\-:()";
        let s: String = String::from(chars);

        let expected: [u8; 34] = [
            0x14, 0xe5, 0x20, 0xa9, 0x15, 0x45, 0x2c, 0xac, 0x15, 0xa5, 0x38, 0xaf, 0x16, 0x5,
            0x44, 0xb2, 0x16, 0x65, 0x50, 0xb5, 0x16, 0xc5, 0x5c, 0xb8, 0x17, 0x25, 0x68, 0xbb,
            0x17, 0x85, 0x74, 0xbe, 0x97, 0xe5,
        ];

        let mut zscii_config = ZsciiConfig::load().expect("Error loading config");
        zscii_config.zmachine_version = Some(3);

        let zs = ZsciiString::<34>::from_str_with_config(&zscii_config, &s);

        assert_eq!(zs.len, 34);
        assert_eq!(zs.data, expected);

        let s: String = String::from(zs);

        assert_eq!(s, chars);
    }

    #[test]
    fn zscii_version_1_unicode_to_zcharacters_default_translation_table_works() {
        let chars = "ò";
        let s: String = String::from(chars);

        let expected: [u8; 4] = [0x0c, 0xc5, 0xe0, 0xa5];

        let mut zscii_config = ZsciiConfig::load().expect("Error loading config");
        zscii_config.zmachine_version = Some(1);

        let zs = ZsciiString::<4>::from_str_with_config(&zscii_config, &s);

        assert_eq!(zs.len, 4);
        assert_eq!(zs.data, expected);
    }

    #[test]
    fn zscii_version_1_zcharacters_to_unicode_default_translation_table_works() {
        let z_characters: [u8; 4] = [0x0c, 0xc5, 0xe0, 0xa5];

        // let expected = "ò";

        let mut zscii_config = ZsciiConfig::load().expect("Error loading config");
        zscii_config.zmachine_version = Some(1);

        let ps = ZsciiString::new_with_config(&zscii_config, 4, z_characters);

        let s: String = String::from(ps);

        assert_eq!(s.chars().count(), 1);
        assert_eq!(s, 'ò'.to_string());
    }

    #[test]
    fn zscii_version_3_unicode_to_zcharacters_default_translation_table_works() {
        let chars = "ò";
        let s: String = String::from(chars);

        let expected: [u8; 4] = [0x14, 0xc5, 0xe0, 0xa5];

        let mut zscii_config = ZsciiConfig::load().expect("Error loading config");
        zscii_config.zmachine_version = Some(3);

        let zs = ZsciiString::<4>::from_str_with_config(&zscii_config, &s);

        assert_eq!(zs.len, 4);
        assert_eq!(zs.data, expected);
    }

    #[test]
    fn zscii_version_3_zcharacters_to_unicode_default_translation_table_works() {
        let z_characters: [u8; 4] = [0x14, 0xc5, 0xe0, 0xa5];

        let mut zscii_config = ZsciiConfig::load().expect("Error loading config");
        zscii_config.zmachine_version = Some(3);

        let ps = ZsciiString::new_with_config(&zscii_config, 4, z_characters);

        let s: String = String::from(ps);

        assert_eq!(s.chars().count(), 1);
        assert_eq!(s, 'ò'.to_string());
    }

    /// The Z-Machine Versions 1 and 2 support permanent shift
    /// characters, so if there is a chain of more than one unicode
    /// 10-bit character using the Unicode table we should use a
    /// permanent shift.
    /// We still need to include a ZSCII 6 character in A2 before
    /// every character though.
    #[test]
    fn zscii_version_1_two_unicode_characters_to_zcharacters_default_translation_table_works() {
        let chars = "òò";
        let s: String = String::from(chars);

        let expected: [u8; 6] = [0x14, 0xc5, 0x60, 0xc5, 0xe0, 0xa5];

        let mut zscii_config = ZsciiConfig::load().expect("Error loading config");
        zscii_config.zmachine_version = Some(1);

        let zs = ZsciiString::<6>::from_str_with_config(&zscii_config, &s);

        assert_eq!(zs.len, 6);
        assert_eq!(zs.data, expected);
    }

    /// The Z-Machine Versions 3 and above don't have a permanent
    /// shift character, so you need a separate shift before each
    /// shifted character.  This includes Unicode from the translation
    /// tables.
    #[test]
    fn zscii_version_3_two_unicode_characters_to_zcharacters_default_translation_table_works() {
        let chars = "òò";
        let s: String = String::from(chars);

        let expected: [u8; 6] = [0x14, 0xc5, 0x60, 0xa6, 0x97, 0x5];

        let mut zscii_config = ZsciiConfig::load().expect("Error loading config");
        zscii_config.zmachine_version = Some(3);

        let zs = ZsciiString::<6>::from_str_with_config(&zscii_config, &s);

        assert_eq!(zs.len, 6);
        assert_eq!(zs.data, expected);
    }
}
