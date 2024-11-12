use crate::bond::bond_types::BondType;

use num_enum::TryFromPrimitiveError;
use std::num::TryFromIntError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MadeleineError {
    #[error("Failed to read from buffer!")]
    ReadError(#[from] std::io::Error),
    #[error("Unknown Bond Type encountered!")]
    TryFromPrimitiveError(#[from] TryFromPrimitiveError<BondType>),
    #[error("Error casting integer!")]
    TryFromIntError(#[from] TryFromIntError),
    #[error("Unavailable bond type encountered!")]
    UnavailableBondType,
    #[error("Error converting from UTF-8!")]
    FromUtf8Error(#[from] std::string::FromUtf8Error),
    #[error("Error converting from UTF-16!")]
    FromUtf16Error(#[from] std::string::FromUtf16Error),
    #[error("Incorrect struct length; Expected: {0} Got: {1}!")]
    IncorrectStructLength(u64, u64),
}
