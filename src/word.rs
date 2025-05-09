use crate::{Error, token::Token};
use std::{convert::TryFrom, fmt};

/// The words upon which our stack machine works.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Word {
    /// Pop an item off the stack.
    Pop,
    /// Swap the top two elements of the stack.
    Swap,
    /// Duplicate the first element of the stack.
    Dup,
    /// Add the first two elements of the stack.
    Add,
    /// Subtract the second from the first element of the stack.
    Sub,
    /// Multiply the first two elements of the stack.
    Mul,
    /// Divide the second into the first element of the stack.
    Div,
    /// Take the remainder of the second in the first element of the stack.
    Mod,
    /// Pop 3 elements. If the first is zero, push the second back on; otherwise, push the third.
    Zero,
    /// An integer.
    Num(i64),
    /// A custom word.
    Custom(String),
}

impl fmt::Display for Word {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Pop => f.write_str("pop"),
            Self::Swap => f.write_str("swap"),
            Self::Dup => f.write_str("dup"),
            Self::Add => f.write_str("add"),
            Self::Sub => f.write_str("sub"),
            Self::Mul => f.write_str("mul"),
            Self::Div => f.write_str("div"),
            Self::Mod => f.write_str("mod"),
            Self::Zero => f.write_str("zero?"),
            Self::Num(n) => write!(f, "{n}"),
            Self::Custom(w) => write!(f, "{w}"),
        }
    }
}

impl TryFrom<Token<'_>> for Word {
    type Error = Error;
    fn try_from(t: Token<'_>) -> Result<Self, Self::Error> {
        match t {
            Token::Def => Err(Error::Reserved),
            Token::Core(w) => match w {
                // A verbose pattern rather than strings to ensure this matches the Display impl.
                s if s == Self::Pop.to_string() => Ok(Self::Pop),
                s if s == Self::Swap.to_string() => Ok(Self::Swap),
                s if s == Self::Dup.to_string() => Ok(Self::Dup),
                s if s == Self::Add.to_string() => Ok(Self::Add),
                s if s == Self::Sub.to_string() => Ok(Self::Sub),
                s if s == Self::Mul.to_string() => Ok(Self::Mul),
                s if s == Self::Div.to_string() => Ok(Self::Div),
                s if s == Self::Mod.to_string() => Ok(Self::Mod),
                s if s == Self::Zero.to_string() => Ok(Self::Zero),
                // Should be unreachable, but let's be careful.
                _ => Err(Error::Unknown(w.to_string())),
            },
            Token::Num(n) | Token::Hex(n) => Ok(Self::Num(n)),
            Token::Custom(w) => Ok(Self::Custom(w.into())),
        }
    }
}

impl TryFrom<Word> for i64 {
    type Error = Error;
    fn try_from(w: Word) -> Result<Self, Self::Error> {
        match w {
            Word::Num(n) => Ok(n),
            _ => Err(Error::NaN(w.to_string())),
        }
    }
}

impl PartialEq<String> for Word {
    fn eq(&self, s: &String) -> bool {
        match self {
            Self::Custom(w) => w == s,
            _ => false,
        }
    }
}

impl Word {
    pub fn into_name(self) -> Result<String, Error> {
        match self {
            Self::Custom(w) => Ok(w),
            Self::Num(n) => Err(Error::NumNotName(n)),
            _ => Err(Error::CoreNotName(self.to_string())),
        }
    }
}
