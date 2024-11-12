use super::errors::MadeleineError;
use crate::bond::bond_types::BondType;

use byteorder::{ReadBytesExt, LE};
use std::{
    fs::File,
    io::{BufReader, Read, Seek},
};

pub trait MyReader: Read
where
    Self: Seek + ReadBytesExt,
{
    fn read_string(&mut self) -> Result<String, MadeleineError> {
        let length = usize::try_from(self.read_uleb128()?)?;
        let mut buffer = vec![0; length];
        self.read_exact(&mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }

    fn read_wstring(&mut self) -> Result<String, MadeleineError> {
        let length = usize::try_from(self.read_uleb128()?)?;
        let mut buffer = vec![0; length * 2];
        self.read_exact(&mut buffer)?;

        let utf16_words = buffer
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes(chunk.try_into().unwrap()))
            .collect::<Vec<u16>>();

        Ok(String::from_utf16(&utf16_words)?)
    }

    fn read_uleb128(&mut self) -> Result<u64, MadeleineError> {
        let mut result: u64 = 0;
        let mut shift = 0;
        let mut b;
        loop {
            b = u64::from(self.read_u8()?);
            result |= (b & 0x7f) << shift;
            shift += 7;
            if (b & 0x80) == 0 {
                break;
            }
        }
        Ok(result)
    }

    fn read_sleb128(&mut self) -> Result<i64, MadeleineError> {
        let unsigned = self.read_uleb128()?;
        Ok(((unsigned >> 1) as i64) ^ -((unsigned & 1) as i64))
    }

    fn get_type_and_id(&mut self) -> Result<(BondType, u16), MadeleineError> {
        let id_and_type = self.read_u8()?;
        let bond_type = BondType::try_from(id_and_type & 0x1f)?;
        let mut id: u16 = u16::from(id_and_type >> 5);

        if id == 6 {
            id = u16::from(self.read_u8()?);
        } else if id == 7 {
            id = self.read_u16::<LE>()?;
        }
        Ok((bond_type, id))
    }

    fn get_type_and_count(&mut self) -> Result<(BondType, u32), MadeleineError> {
        let byte = self.read_u8()?;
        let bond_type = BondType::try_from(byte & 0x1f)?;

        let count = match u32::from(byte >> 5) {
            0 => u32::try_from(self.read_uleb128()?)?,
            n => n - 1,
        };

        Ok((bond_type, count))
    }
}

impl MyReader for BufReader<File> {}
