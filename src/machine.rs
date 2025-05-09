use crate::{error::Error, token::Token, word::Word};
use indexmap::IndexMap;
use logos::Logos;
use std::{cell::RefCell, convert::TryFrom, fmt};

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
        for k in self.env.borrow().keys() {
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
                    } else if us.iter().any(|u| u == &k) {
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
    fn pop(&self, w: &Word, required: usize, stack_len: usize) -> Result<Word, Error> {
        self.stack
            .borrow_mut()
            .pop()
            .ok_or_else(|| Error::Small(w.to_string(), required, stack_len))
    }
    fn eval(&self, w: &Word) -> Result<(), Error> {
        match w {
            Word::Pop => self.pop(w, 1, 0).map(|_| ())?,
            Word::Swap => {
                let x = self.pop(w, 2, 0)?;
                let y = self.pop(w, 2, 1)?;
                self.stack.borrow_mut().push(x);
                self.stack.borrow_mut().push(y);
            }
            Word::Dup => {
                let x = self.pop(w, 1, 0)?;
                self.stack.borrow_mut().push(x.clone());
                self.stack.borrow_mut().push(x);
            }
            Word::Add => {
                let x = self.pop(w, 2, 0).and_then(i64::try_from)?;
                let y = self.pop(w, 2, 1).and_then(i64::try_from)?;
                self.stack.borrow_mut().push(Word::Num(x + y));
            }
            Word::Sub => {
                let x = self.pop(w, 2, 0).and_then(i64::try_from)?;
                let y = self.pop(w, 2, 1).and_then(i64::try_from)?;
                self.stack.borrow_mut().push(Word::Num(x - y));
            }
            Word::Mul => {
                let x = self.pop(w, 2, 0).and_then(i64::try_from)?;
                let y = self.pop(w, 2, 1).and_then(i64::try_from)?;
                self.stack.borrow_mut().push(Word::Num(x * y));
            }
            Word::Div => {
                let x = self.pop(w, 2, 0).and_then(i64::try_from)?;
                let y = self.pop(w, 2, 1).and_then(i64::try_from)?;
                self.stack.borrow_mut().push(Word::Num(x / y));
            }
            Word::Mod => {
                let x = self.pop(w, 2, 0).and_then(i64::try_from)?;
                let y = self.pop(w, 2, 1).and_then(i64::try_from)?;
                self.stack.borrow_mut().push(Word::Num(x % y));
            }
            Word::Zero => {
                let x = self.pop(w, 3, 0).and_then(i64::try_from)?;
                let y = self.pop(w, 3, 1)?;
                let z = self.pop(w, 3, 2)?;
                self.stack.borrow_mut().push(if x == 0 { y } else { z });
            }
            Word::Num(_) => self.stack.borrow_mut().push(w.clone()),
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
}
