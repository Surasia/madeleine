use crate::bond::bond_types::read_struct;
use crate::common::errors::MadeleineError;

use std::{fs::File, io::BufReader};

use super::bond_types::BondValue;

/// A reader for Bond-formatted binary data.
///
/// This reader handles the parsing of Bond binary format files, providing
/// a high-level interface to read Bond structures.
pub struct BondReader {
    reader: BufReader<File>,
}

impl BondReader {
    pub fn new(filename: impl Into<String>) -> Result<Self, MadeleineError> {
        let file = File::open(filename.into())?;
        let reader = BufReader::new(file);
        Ok(Self { reader })
    }

    pub fn read(&mut self) -> Result<BondValue, MadeleineError> {
        read_struct(&mut self.reader, 2)
    }
}
