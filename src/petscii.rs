//!
//! PETSCII string library
//!
//! PETSCII is a character set used in Commodore Business Machines'
//! 8-bit computers.  It's based on the 1963 version of ASCII, not the
//! 1967 version.  In addition, it has some custom block graphics
//! characters, geometric shapes and playing card suits.
//!
#![warn(missing_docs)]
#![warn(unsafe_code)]

use std::fmt::{Debug, Display, Formatter, Result};

use crate::SystemConfig;

/// A PETSCII string
///
/// A fixed-length PETSCII string Later versions may support
/// variable-length strings.  This library was created to help debug
/// C64 file systems, which use fixed-length strings for some of the
/// data structures.
#[derive(Clone, Copy)]
pub struct PetsciiString<'a, const L: usize> {
    /// The length of the string
    pub len: u32,
    /// The string data
    pub data: [u8; L],

    /// The character map for this string
    pub character_map: Option<&'a SystemConfig>,
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
        let iter = (*self).into_iter().map(|b| b as char);
        let s = String::from_iter(iter);
        write!(f, "{}", s)
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

impl<'a, const L: usize> From<PetsciiString<'a, L>> for String {
    /// Create a String from a PetsciiString
    ///
    /// # Examples
    ///
    /// ```
    /// use std::process::exit;
    /// use forbidden_bands::{load_config, petscii::PetsciiString, Config};
    ///
    /// let config_result = load_config();
    /// let config: Config = match config_result {
    ///     Ok(c) => c,
    ///     Err(e) => {
    ///         println!("Error loading config: {:?}", e);
    ///         exit(-1);
    ///     }
    /// };
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
    // TODO: Fix this, from_utf8 needs a lot of additional work with
    // lookup tables and error handling.  See below for more.
    // TODO: Unicode 13 now has "Legacy Computing Sources"
    // (Unicode 13 was released around March 10, 2020).
    // Add those mappings to the config
    fn from(s: PetsciiString<L>) -> String {
        s.into_iter()
            .map(|c| {
                let key = c.to_string();
                let d = match &s.character_map {
                    Some(cm) => {
                        if cm.character_set_map.contains_key(&key) {
                            match cm.character_set_map.get(&key).unwrap() {
                                serde_json::Value::Number(v) => v.as_u64().unwrap() as u32,
                                _ => 0,
                            }
                        } else {
                            c as u32
                        }
                    }
                    None => c as u32,
                };
                char::from_u32(d).unwrap()
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
        }
    }

    /// Create a new PETSCII string with a given character map
    ///
    /// # Examples
    ///
    /// ```
    /// use std::process::exit;
    /// use forbidden_bands::{load_config, petscii::PetsciiString, Config};
    ///
    /// let config_result = load_config();
    /// let config: Config = match config_result {
    ///     Ok(c) => c,
    ///     Err(e) => {
    ///         println!("Error loading config: {:?}", e);
    ///         exit(-1);
    ///     }
    /// };
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
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{load_config, petscii::PetsciiString, Config, CONFIG};
    use std::process::exit;

    #[cfg(feature = "external-json")]
    use crate::load_config_from_file;

    #[test]
    fn petscii_struct_works() {
        let ps = PetsciiString::new(3, [0x41, 0x42, 0x43]);
        assert_eq!(ps.len, 3);
        assert_eq!(ps.data, [0x41, 0x42, 0x43]);
    }

    #[test]
    fn petscii_with_config_works() {
        let config_result = load_config();
        let config: Config = match config_result {
            Ok(c) => c,
            Err(e) => {
                println!("Error loading config: {:?}", e);
                exit(-1);
            }
        };
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
        let config_result = load_config();
        let config: Config = match config_result {
            Ok(c) => c,
            Err(e) => {
                println!("Error loading config: {:?}", e);
                exit(-1);
            }
        };
        let ps = PetsciiString::new_with_config(2, [0x41, 0xb2], &config.petscii);
        let mut s: String = String::from(ps);
        assert_eq!(s.pop().unwrap(), '²');
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
    fn into_iter_works() {
        #[cfg(not(feature = "external-json"))]
        let config_result = load_config();
        #[cfg(feature = "external-json")]
        let config_result = {
            let config_fn = String::from("data/config.json");
            load_config_from_file(&config_fn)
        };
        let config: Config = match config_result {
            Ok(c) => c,
            Err(e) => {
                println!("Error loading config: {:?}", e);
                exit(-1);
            }
        };

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

        let ps = PetsciiString::new_with_config(3, [0x41, 0x42, 0x43], &config.petscii);

        let mut iter = ps.into_iter();

        assert_eq!(iter.next(), Some(0x41));
        assert_eq!(iter.next(), Some(0x42));
        assert_eq!(iter.next(), Some(0x43));
        assert_eq!(iter.next(), None);
    }
}
