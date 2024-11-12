#![deny(clippy::pedantic)]
#![deny(clippy::all)]
use bond::bond_reader::BondReader;
use clap::Parser;
use common::errors::MadeleineError;

#[derive(Debug, Parser)]
pub struct Madeleine {
    #[arg(short, long)]
    /// Path to bond file to deserialize.
    path: String,
}

pub mod bond;
pub mod common;

fn main() -> Result<(), MadeleineError> {
    let args = Madeleine::parse();
    let mut reader = BondReader::new(args.path.to_string())?;
    let values = reader.read()?;
    println!("{:#?}", values);
    Ok(())
}
