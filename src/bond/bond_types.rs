use crate::common::{binary_rw::MyReader, errors::MadeleineError};

use byteorder::{ReadBytesExt, LE};
use num_enum::TryFromPrimitive;
use std::io::Read;

#[derive(Debug, PartialEq, PartialOrd)]
pub enum BondValue {
    Guid(Guid),
    Bool(bool),
    Uint8(u8),
    Uint16(u16),
    Uint32(u32),
    Uint64(u64),
    Float(f32),
    Double(f64),
    Stop,
    StopBase,
    List(Vec<BondValue>),
    Set(Vec<BondValue>),
    Map(Vec<(BondValue, BondValue)>),
    String(String),
    Wstring(String),
    Int8(i8),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    Struct {
        base: Option<Box<BondValue>>,
        fields: Vec<BondValue>,
    },
}

impl BondValue {
    pub fn new_struct(base: Option<Box<BondValue>>, fields: Vec<BondValue>) -> Self {
        Self::Struct { base, fields }
    }

    pub fn fields(&self) -> Option<&[BondValue]> {
        if let Self::Struct { fields, .. } = self {
            Some(fields)
        } else {
            None
        }
    }

    pub fn base(&self) -> Option<&BondValue> {
        if let Self::Struct { base, .. } = self {
            base.as_deref()
        } else {
            None
        }
    }
}

#[derive(Debug, TryFromPrimitive, PartialEq, Eq)]
#[repr(u8)]
pub enum BondType {
    Stop = 0,
    StopBase = 1,
    Bool = 2,
    Uint8 = 3,
    Uint16 = 4,
    Uint32 = 5,
    Uint64 = 6,
    Float = 7,
    Double = 8,
    String = 9,
    Struct = 10,
    List = 11,
    Set = 12,
    Map = 13,
    Int8 = 14,
    Int16 = 15,
    Int32 = 16,
    Int64 = 17,
    Wstring = 18,
    Unavailable = 127,
}

#[derive(Debug, PartialEq, PartialOrd)]
pub struct Guid(String);

impl Guid {
    fn from_parts(data1: u32, data2: u16, data3: u16, data4: u64) -> Self {
        let data_4b = data4.to_le_bytes();
        Self(format!(
            "{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}",
            data1,
            data2,
            data3,
            data_4b[0],
            data_4b[1],
            data_4b[2],
            data_4b[3],
            data_4b[4],
            data_4b[5],
            data_4b[6],
            data_4b[7]
        ))
    }
}

impl std::fmt::Display for Guid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn read_blob<R: Read + ReadBytesExt + MyReader>(
    reader: &mut R,
    count: u32,
) -> Result<Vec<BondValue>, MadeleineError> {
    let mut blob = vec![0u8; count as usize];
    reader.read_exact(&mut blob)?;
    Ok(vec![BondValue::List(
        blob.into_iter().map(|b| BondValue::Uint8(b)).collect(),
    )])
}

pub fn read_list<R: Read + ReadBytesExt + MyReader>(
    reader: &mut R,
    container_type: &BondType,
) -> Result<Vec<BondValue>, MadeleineError> {
    let (element_type, count) = reader.get_type_and_count()?;

    match (container_type, &element_type) {
        (BondType::List, BondType::Int8 | BondType::Uint8) => read_blob(reader, count),
        _ => (0..count)
            .map(|_| read_value(&element_type, reader))
            .collect(),
    }
}

fn process_values_for_guids(values: &mut Vec<BondValue>) {
    let mut i = 0;
    while i + 3 < values.len() {
        let window = &values[i..=i + 3];
        match window {
            [BondValue::Uint32(data1), BondValue::Uint16(data2), BondValue::Uint16(data3), BondValue::Uint64(data4)] =>
            {
                let guid = Guid::from_parts(*data1, *data2, *data3, *data4);
                values.splice(i..i + 4, std::iter::once(BondValue::Guid(guid)));
                i += 1;
            }
            _ => i += 1,
        }
    }
}

fn check_struct_length<R: Read + ReadBytesExt + MyReader>(
    reader: &mut R,
    expected_length: Option<u64>,
    start_pos: Option<u64>,
) -> Result<(), MadeleineError> {
    if let (Some(expected), Some(start)) = (expected_length, start_pos) {
        let actual_length = reader.stream_position()? - start;
        if actual_length != expected {
            return Err(MadeleineError::IncorrectStructLength(
                expected,
                actual_length,
            ));
        }
    }
    Ok(())
}

pub fn read_map<R: Read + ReadBytesExt + MyReader>(
    reader: &mut R,
) -> Result<Vec<(BondValue, BondValue)>, MadeleineError> {
    let key_type = &BondType::try_from(reader.read_u8()? & 0x1f)?;
    let value_type = &BondType::try_from(reader.read_u8()? & 0x1f)?;
    let count = reader.read_uleb128()?;
    let mut map = Vec::new();
    for _ in 0..count {
        let key = read_value(key_type, reader)?;
        let value = read_value(value_type, reader)?;
        map.push((key, value));
    }
    Ok(map)
}

pub fn read_struct<R: Read + ReadBytesExt + MyReader>(
    reader: &mut R,
    version: u8,
) -> Result<BondValue, MadeleineError> {
    let (expected_length, start_pos) = match version {
        2 => {
            let length = reader.read_uleb128()?;
            let pos = reader.stream_position()?;
            (Some(length), Some(pos))
        }
        _ => (None, None),
    };

    let mut values = Vec::new();
    let mut base_struct = Vec::new();

    loop {
        match read_field(reader)? {
            BondValue::Stop => break,
            BondValue::StopBase => {
                base_struct = std::mem::take(&mut values);
            }
            field => values.push(field),
        }
    }

    process_values_for_guids(&mut values);
    check_struct_length(reader, expected_length, start_pos)?;

    let base = if !base_struct.is_empty() {
        Some(Box::new(BondValue::new_struct(None, base_struct)))
    } else {
        None
    };

    Ok(BondValue::new_struct(base, values))
}

pub fn read_field<R: Read + ReadBytesExt + MyReader>(
    reader: &mut R,
) -> Result<BondValue, MadeleineError> {
    let field_typeid = reader.get_type_and_id()?;
    read_value(&field_typeid.0, reader)
}

pub fn read_value<R: Read + ReadBytesExt + MyReader>(
    bond_type: &BondType,
    reader: &mut R,
) -> Result<BondValue, MadeleineError> {
    match bond_type {
        BondType::Bool => Ok(BondValue::Bool(reader.read_u8()? != 0)),
        BondType::Uint8 => Ok(BondValue::Uint8(reader.read_u8()?)),
        BondType::Uint16 => Ok(BondValue::Uint16(u16::try_from(reader.read_uleb128()?)?)),
        BondType::Uint32 => Ok(BondValue::Uint32(u32::try_from(reader.read_uleb128()?)?)),
        BondType::Uint64 => Ok(BondValue::Uint64(reader.read_uleb128()?)),
        BondType::Float => Ok(BondValue::Float(reader.read_f32::<LE>()?)),
        BondType::Double => Ok(BondValue::Double(reader.read_f64::<LE>()?)),
        BondType::List => Ok(BondValue::List(read_list(reader, bond_type)?)),
        BondType::Set => Ok(BondValue::Set(read_list(reader, bond_type)?)),
        BondType::Stop => Ok(BondValue::Stop),
        BondType::StopBase => Ok(BondValue::StopBase),
        BondType::Map => Ok(BondValue::Map(read_map(reader)?)),
        BondType::String => Ok(BondValue::String(reader.read_string()?)),
        BondType::Int8 => Ok(BondValue::Int8(reader.read_i8()?)),
        BondType::Int16 => Ok(BondValue::Int16(i16::try_from(reader.read_sleb128()?)?)),
        BondType::Int32 => Ok(BondValue::Int32(i32::try_from(reader.read_sleb128()?)?)),
        BondType::Int64 => Ok(BondValue::Int64(reader.read_sleb128()?)),
        BondType::Wstring => Ok(BondValue::Wstring(reader.read_wstring()?)),
        BondType::Struct => Ok(read_struct(reader, 2)?),
        BondType::Unavailable => Err(MadeleineError::UnavailableBondType),
    }
}
