//! Dylan's Rusty Stack Machine.
#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![deny(missing_docs)]
#![deny(unsafe_code)]
use indexmap::IndexMap;
use logos::Logos;
use std::{cell::RefCell, fmt, num::ParseIntError};

/// Our Error type.
#[derive(Clone, Debug, Default, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[default]
    /// Something bad happened.
    #[error("Something bad happened.")]
    Bad,
    /// I expected a number, but I found `{0}`.
    #[error("I expected a number, but I found `{0}`.")]
    NaN(Token),
    /// The stack is too small for `{0}`; it requires {1}, but the stack only has {2}.
    #[error("The stack is too small for `{0}`; it requires {1}, but the stack only has {2}.")]
    Small(Token, usize, usize),
    /// Parsing error: `{0}`.
    #[error("Parsing error: `{0}`.")]
    Parsing(String),
    /// Something happend when trying to read: {0}.
    #[error("Something happend when trying to read: {0}.")]
    Readline(String),
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
    /// `def` needs a name, but a number `{0}` was supplied.
    #[error("`def` needs a name, but a number `{0}` was supplied.")]
    DefNum(i64),
    /// `def` needs a body, but none was supplied.
    #[error("`def` needs a body, but none was supplied.")]
    DefBody,
}

/// Tokens are lexed from input strings.
#[derive(Logos, Debug, PartialEq, Eq, Clone)]
#[logos(error = Error)]
#[logos(skip r"\s")]
pub enum Token {
    /// Define a new word.
    #[token("def")]
    Def,
    #[token("pop")]
    /// Pop an item off the stack.
    Pop,
    #[token("swap")]
    /// Swap the top two elements of the stack.
    Swap,
    #[token("dup")]
    /// Duplicate the first element of the stack.
    Dup,
    #[token("add")]
    /// Add the first two elements of the stack.
    Add,
    #[token("sub")]
    /// Subtract the second from the first element of the stack.
    Sub,
    /// Multiply the first two elements of the stack.
    #[token("mul")]
    Mul,
    #[token("div")]
    /// Divide the second into the first element of the stack.
    Div,
    #[token("mod")]
    /// Take the remainder of the second in the first element of the stack.
    Mod,
    #[token("zero?")]
    /// Pop 3 elements. If the first is zero, push the second back on; otherwise, push the third.
    Zero,
    /// An integer in decimal notation.
    #[regex(r"-?[0-9]+", |lex| lex.slice().parse().map_err(|e: ParseIntError| Error::Parsing(e.to_string())), priority = 3)]
    Num(i64),
    /// An integer in hexadecimal notation.
    #[regex(r"#[0-9a-fA-F]+", |lex| i64::from_str_radix(&lex.slice()[1..], 16).map_err(|e: ParseIntError| Error::Parsing(e.to_string())))]
    Hex(i64),
    /// A (possibly unknown) word.
    #[regex(r"\S+", |lex| lex.slice().to_owned())]
    Word(String),
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Def => f.write_str("def"),
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
            Self::Hex(n) => write!(f, "#{n:x}"),
            Self::Word(w) => write!(f, "{w}"),
        }
    }
}

impl Token {
    fn into_name(self) -> Result<String, Error> {
        match self {
            Self::Word(w) => Ok(w),
            Self::Def => Err(Error::Reserved),
            Self::Num(n) | Self::Hex(n) => Err(Error::DefNum(n)),
            _ => Ok(self.to_string()),
        }
    }
    fn into_num(self) -> Result<i64, Error> {
        match self {
            Self::Num(n) | Self::Hex(n) => Ok(n),
            _ => Err(Error::NaN(self)),
        }
    }
}

/// The main data structure: a stack machine with an environment of local definitions.
#[derive(Default)]
pub struct Machine {
    env: RefCell<IndexMap<String, Vec<Token>>>,
    stack: RefCell<Vec<Token>>,
}

impl fmt::Display for Machine {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("core:")?;
        for t in [
            Token::Def,
            Token::Pop,
            Token::Swap,
            Token::Dup,
            Token::Add,
            Token::Sub,
            Token::Mul,
            Token::Div,
            Token::Mod,
            Token::Zero,
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
    fn pop(&self, t: &Token, required: usize, stack_len: usize) -> Result<Token, Error> {
        self.stack
            .borrow_mut()
            .pop()
            .ok_or_else(|| Error::Small(t.clone(), required, stack_len))
    }
    fn eval(&self, t: &Token) -> Result<(), Error> {
        match t {
            Token::Def => return Err(Error::Reserved),
            Token::Num(_) | Token::Hex(_) => self.stack.borrow_mut().push(t.clone()),
            Token::Pop => self.pop(t, 1, 0).map(|_| ())?,
            Token::Swap => {
                let x = self.pop(t, 2, 0)?;
                let y = self.pop(t, 2, 1)?;
                self.stack.borrow_mut().push(x);
                self.stack.borrow_mut().push(y);
            }
            Token::Dup => {
                let x = self.pop(t, 1, 0)?;
                self.stack.borrow_mut().push(x.clone());
                self.stack.borrow_mut().push(x);
            }
            Token::Add => {
                let x = self.pop(t, 2, 0).and_then(Token::into_num)?;
                let y = self.pop(t, 2, 1).and_then(Token::into_num)?;
                self.stack.borrow_mut().push(Token::Num(x + y));
            }
            Token::Sub => {
                let x = self.pop(t, 2, 0).and_then(Token::into_num)?;
                let y = self.pop(t, 2, 1).and_then(Token::into_num)?;
                self.stack.borrow_mut().push(Token::Num(x - y));
            }
            Token::Mul => {
                let x = self.pop(t, 2, 0).and_then(Token::into_num)?;
                let y = self.pop(t, 2, 1).and_then(Token::into_num)?;
                self.stack.borrow_mut().push(Token::Num(x * y));
            }
            Token::Div => {
                let x = self.pop(t, 2, 0).and_then(Token::into_num)?;
                let y = self.pop(t, 2, 1).and_then(Token::into_num)?;
                self.stack.borrow_mut().push(Token::Num(x / y));
            }
            Token::Mod => {
                let x = self.pop(t, 2, 0).and_then(Token::into_num)?;
                let y = self.pop(t, 2, 1).and_then(Token::into_num)?;
                self.stack.borrow_mut().push(Token::Num(x % y));
            }
            Token::Zero => {
                let x = self.pop(t, 3, 0).and_then(Token::into_num)?;
                let y = self.pop(t, 3, 1)?;
                let z = self.pop(t, 3, 2)?;
                self.stack.borrow_mut().push(if x == 0 { y } else { z });
            }
            Token::Word(w) => match self.env.borrow().get(w) {
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
                    let k = ts.next().ok_or(Error::DefName).and_then(Token::into_name)?;
                    let us = ts.collect::<Vec<_>>();
                    if us.is_empty() {
                        return Err(Error::DefBody);
                    } else if us.iter().any(|u| u.to_string() == k) {
                        return Err(Error::SelfRef(k));
                    }
                    let _ = self.env.borrow_mut().insert(k, us);
                    break;
                }
                _ => self.eval(&t)?,
            }
        }
        Ok(())
    }
}
