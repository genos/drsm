//! Dylan's Rusty Stack Machine.
#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![deny(missing_docs)]
#![deny(unsafe_code)]
use indexmap::IndexMap;
use logos::Logos;
use std::{cell::RefCell, convert::TryFrom, fmt, num::ParseIntError};

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
}

/// Tokens are lexed from input strings.
#[derive(Logos, Debug, PartialEq, Eq, Clone)]
#[logos(error = Error)]
#[logos(skip r"\s")]
enum Token<'source> {
    /// Define a new word.
    #[token("def")]
    Def,
    /// A core word.
    #[regex(r"(pop|swap|dup|add|sub|mul|div|mod|zero)")]
    Core(&'source str),
    /// An integer in decimal notation.
    #[regex(r"-?[0-9]+", |lex| lex.slice().parse().map_err(Error::Parsing))]
    Num(i64),
    /// An integer in hexadecimal notation.
    #[regex(r"#[0-9a-fA-F]+", |lex| i64::from_str_radix(&lex.slice()[1..], 16).map_err(Error::Parsing))]
    Hex(i64),
    /// A (possibly unknown) custom token.
    #[regex(r"\S+", priority = 0)]
    Custom(&'source str),
}

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
            Token::Custom(w) => Ok(Self::Custom(w.to_string())),
        }
    }
}

impl Word {
    fn into_name(self) -> Result<String, Error> {
        match self {
            Self::Num(n) => Err(Error::NumNotName(n)),
            Self::Custom(w) => Ok(w),
            _ => Err(Error::CoreNotName(self.to_string())),
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

/// The main data structure: a stack machine with an environment of local definitions.
#[derive(Default)]
pub struct Machine {
    env: RefCell<IndexMap<String, Vec<Word>>>,
    stack: RefCell<Vec<Word>>,
}

impl fmt::Display for Machine {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("core:")?;
        for t in [
            Word::Pop,
            Word::Swap,
            Word::Dup,
            Word::Add,
            Word::Sub,
            Word::Mul,
            Word::Div,
            Word::Mod,
            Word::Zero,
        ] {
            write!(f, " {t}")?;
        }
        f.write_str("\nenv:")?;
        for k in self.env.borrow().keys().rev() {
            write!(f, " {k}")?;
        }
        f.write_str("\nstack: [")?;
        for t in self.stack.borrow().iter().rev() {
            write!(f, " {t}")?;
        }
        f.write_str(" ]")
    }
}

impl Machine {
    fn pop(&self, w: &Word, required: usize, stack_len: usize) -> Result<Word, Error> {
        self.stack
            .borrow_mut()
            .pop()
            .ok_or_else(|| Error::Small(w.to_string(), required, stack_len))
    }
    fn eval(&self, t: &Word) -> Result<(), Error> {
        match t {
            Word::Num(_) => self.stack.borrow_mut().push(t.clone()),
            Word::Pop => self.pop(t, 1, 0).map(|_| ())?,
            Word::Swap => {
                let x = self.pop(t, 2, 0)?;
                let y = self.pop(t, 2, 1)?;
                self.stack.borrow_mut().push(x);
                self.stack.borrow_mut().push(y);
            }
            Word::Dup => {
                let x = self.pop(t, 1, 0)?;
                self.stack.borrow_mut().push(x.clone());
                self.stack.borrow_mut().push(x);
            }
            Word::Add => {
                let x = self.pop(t, 2, 0).and_then(i64::try_from)?;
                let y = self.pop(t, 2, 1).and_then(i64::try_from)?;
                self.stack.borrow_mut().push(Word::Num(x + y));
            }
            Word::Sub => {
                let x = self.pop(t, 2, 0).and_then(i64::try_from)?;
                let y = self.pop(t, 2, 1).and_then(i64::try_from)?;
                self.stack.borrow_mut().push(Word::Num(x - y));
            }
            Word::Mul => {
                let x = self.pop(t, 2, 0).and_then(i64::try_from)?;
                let y = self.pop(t, 2, 1).and_then(i64::try_from)?;
                self.stack.borrow_mut().push(Word::Num(x * y));
            }
            Word::Div => {
                let x = self.pop(t, 2, 0).and_then(i64::try_from)?;
                let y = self.pop(t, 2, 1).and_then(i64::try_from)?;
                self.stack.borrow_mut().push(Word::Num(x / y));
            }
            Word::Mod => {
                let x = self.pop(t, 2, 0).and_then(i64::try_from)?;
                let y = self.pop(t, 2, 1).and_then(i64::try_from)?;
                self.stack.borrow_mut().push(Word::Num(x % y));
            }
            Word::Zero => {
                let x = self.pop(t, 3, 0).and_then(i64::try_from)?;
                let y = self.pop(t, 3, 1)?;
                let z = self.pop(t, 3, 2)?;
                self.stack.borrow_mut().push(if x == 0 { y } else { z });
            }
            Word::Custom(w) => match self.env.borrow().get(w) {
                None => return Err(Error::Unknown(w.to_string())),
                Some(vs) => {
                    for v in vs {
                        self.eval(v)?;
                    }
                }
            },
        }
        Ok(())
    }
    /// Read a string & evaluate it.
    ///
    /// # Errors
    /// If something goes wrong in lexing or evaluation.
    pub fn read_eval(&self, s: &str) -> Result<(), Error> {
        let mut ts = Token::lexer(s).collect::<Result<Vec<_>, _>>()?.into_iter();
        while let Some(t) = ts.next() {
            match t {
                Token::Def => {
                    let k = ts
                        .next()
                        .ok_or(Error::DefName)
                        .and_then(Word::try_from)
                        .and_then(Word::into_name)?;
                    let us = ts.map(Word::try_from).collect::<Result<Vec<_>, _>>()?;
                    if us.is_empty() {
                        return Err(Error::DefBody);
                    } else if us.iter().any(|u| u.to_string() == k) {
                        return Err(Error::SelfRef(k));
                    }
                    let _ = self.env.borrow_mut().insert(k, us);
                    break;
                }
                _ => self.eval(&Word::try_from(t)?)?,
            }
        }
        Ok(())
    }
}
