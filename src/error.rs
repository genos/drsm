use std::num::ParseIntError;

/// Our Error type.
#[derive(Clone, Debug, Default, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[default]
    /// Something bad happened.
    #[error("Something bad happened.")]
    Bad,
    /// I expected a number, but I found `{0}`.
    #[error("I expected a number, but I found `{0}`.")]
    NaN(String),
    /// The stack is too small for `{0}`; it requires {1}, but the stack only has {2}.
    #[error("The stack is too small for `{0}`; it requires {1}, but the stack only has {2}.")]
    Small(String, usize, usize),
    /// Error parsing an int: `{0}`.
    #[error("Error parsing an int: `{0}`.")]
    Parsing(#[from] ParseIntError),
    /// Unknown op: `{0}`.
    #[error("Unknown op: `{0}`.")]
    Unknown(String),
    /// Self reference: `{0}` refers to itself.
    #[error("Self reference: `{0}` refers to itself.")]
    SelfRef(String),
    /// `def` is a reserved keyword.
    #[error("`def` is a reserved keyword.")]
    Reserved,
    /// `def` needs a name, but none was supplied.
    #[error("`def` needs a name, but none was supplied.")]
    DefName,
    /// A name was expected, but a number `{0}` was supplied.
    #[error("A name was expected, but a number `{0}` was supplied.")]
    NumNotName(i64),
    /// A name was expected, but a core word `{0}` was supplied.
    #[error("A name was expected, but a core word `{0}` was supplied.")]
    CoreNotName(String),
    /// `def` needs a body, but none was supplied.
    #[error("`def` needs a body, but none was supplied.")]
    DefBody,
    /// `{0}` requires its second operand be nonzero
    #[error("`{0}` requires its second operand be nonzero")]
    NNZ(String),
}
