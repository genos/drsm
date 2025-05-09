//! Dylan's Rusty Stack Machine.
#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![deny(missing_docs)]
#![deny(unsafe_code)]

mod error;
mod machine;
mod token;
mod word;

pub use crate::{error::Error, machine::Machine};
