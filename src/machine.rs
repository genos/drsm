use crate::{error::Error, token::Token, word::Word};
use indexmap::IndexMap;
use logos::Logos;
use std::{convert::TryFrom, fmt};

/// The main data structure: a stack machine with an environment of local definitions.
#[derive(Debug)]
pub struct Machine {
    env: IndexMap<String, Vec<Word>>,
    stack: Vec<i64>,
}

impl Default for Machine {
    fn default() -> Self {
        Self {
            env: IndexMap::with_capacity(1_024),
            stack: Vec::with_capacity(1024),
        }
    }
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
        for k in self.env.keys() {
            write!(f, " {k}")?;
        }
        f.write_str("\nstack: [")?;
        for t in self.stack.iter().rev() {
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
    pub fn read_eval(&mut self, s: &str) -> Result<(), Error> {
        let mut ts = Token::lexer(s).collect::<Result<Vec<_>, _>>()?.into_iter();
        while let Some(t) = ts.next() {
            if t == Token::Def {
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
                let _ = self.env.insert(k, us);
                break; // no need for `else` here
            }
            self.eval(&Word::try_from(t)?)?;
        }
        Ok(())
    }
    fn eval(&mut self, word: &Word) -> Result<(), Error> {
        check(&self.env, &self.stack, word)?;
        eval_inner(&self.env, &mut self.stack, word)
    }
}

/// Broken out because `eval_inner` is separate, too.
fn check(env: &IndexMap<String, Vec<Word>>, stack: &[i64], word: &Word) -> Result<(), Error> {
    let s = stack.len();
    let r = match word {
        Word::Num(_) | Word::Custom(_) => 0,
        Word::Pop | Word::Dup => 1,
        Word::Swap | Word::Add | Word::Sub | Word::Mul | Word::Div | Word::Mod => 2,
        Word::Zero => 3,
    };
    if s < r {
        Err(Error::Small(word.to_string(), r, s))
    } else if (*word == Word::Div || *word == Word::Mod) && stack[s - 2] == 0 {
        Err(Error::NNZ(word.to_string()))
    } else if matches!(word, Word::Custom(_)) && !env.contains_key(&word.to_string()) {
        Err(Error::Unknown(word.to_string()))
    } else {
        Ok(())
    }
}

/// Broken out to untangle mutability concerns.
/// Full of `stack.pop().expect(…)` because this should only be called from within `Machine::eval`.
fn eval_inner(
    env: &IndexMap<String, Vec<Word>>,
    stack: &mut Vec<i64>,
    word: &Word,
) -> Result<(), Error> {
    match word {
        Word::Pop => {
            stack.pop().expect("Internal error @ pop");
        }
        Word::Swap => {
            let x = stack.pop().expect("Internal error @ swap 1");
            let y = stack.pop().expect("Internal error @ swap 2");
            stack.push(x);
            stack.push(y);
        }
        Word::Dup => {
            let x = stack.pop().expect("Internal error @ dup");
            stack.push(x);
            stack.push(x);
        }
        Word::Add => {
            let x = stack.pop().expect("Internal error @ add 1");
            let y = stack.pop().expect("Internal error @ add 2");
            stack.push(x.wrapping_add(y));
        }
        Word::Sub => {
            let x = stack.pop().expect("Internal error @ sub 1");
            let y = stack.pop().expect("Internal error @ sub 2");
            stack.push(x.wrapping_sub(y));
        }
        Word::Mul => {
            let x = stack.pop().expect("Internal error @ mul 1");
            let y = stack.pop().expect("Internal error @ mul 2");
            stack.push(x.wrapping_mul(y));
        }
        Word::Div => {
            let x = stack.pop().expect("Internal error @ div 1");
            let y = stack.pop().expect("Internal error @ div 2");
            stack.push(x.wrapping_div(y));
        }
        Word::Mod => {
            let x = stack.pop().expect("Internal error @ mod 1");
            let y = stack.pop().expect("Internal error @ mod 2");
            stack.push(x.wrapping_rem(y));
        }
        Word::Zero => {
            let x = stack.pop().expect("Internal error @ zero? 1");
            let y = stack.pop().expect("Internal error @ zero? 2");
            let z = stack.pop().expect("Internal error @ zero? 3");
            stack.push(if x == 0 { y } else { z });
        }
        Word::Num(n) => stack.push(*n),
        Word::Custom(c) => {
            for w in &env[c] {
                check(env, stack, w)?;
                eval_inner(env, stack, w)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{super::word::tests::word, *};
    use proptest::prelude::*;

    #[test]
    fn def_errs() {
        for s in ["def", "def name", "def def pop", "def pop body", "def name name"] {
            assert!(Machine::default().read_eval(&s).is_err());
        }
    }

    proptest! {
        #[test]
        fn pushing_extends_stack(ns in prop::collection::vec(any::<i64>(), 1..64)) {
            let mut m = Machine::default();
            let mut old = m.to_string().len();
            for n in ns {
                prop_assert!(m.eval(&Word::Num(n)).is_ok());
                let new = m.to_string().len();
                prop_assert_eq!(new - old, format!(" {n}").len());
                old = new;
            }
        }
        #[test]
        fn check_implies_eval(ws in prop::collection::vec(word(), 0..64)) {
            let mut m = Machine::default();
            for w in ws {
                if check(&m.env, &m.stack, &w).is_ok() {
                    prop_assert!(m.eval(&w).is_ok(), "Machine with state {m:?} failed on {w}");
                }
            }
        }
        #[test]
        fn check_implies_read_eval(ws in prop::collection::vec(word(), 0..64)) {
            let mut m = Machine::default();
            for w in ws {
                if check(&m.env, &m.stack, &w).is_ok() {
                    prop_assert!(m.read_eval(&w.to_string()).is_ok(), "Machine with state {m:?} failed on {w}");
                }
            }
        }
        #[test]
        fn def_adds_to_env(ws in prop::collection::vec(r"\S+", 0..64), n in r"\S+") {
            let mut m = Machine::default();
            let s = format!("def {n} {}", ws.join(" "));
            let r = m.read_eval(&s);
            prop_assert!(
                (ws.is_empty()
                    || ws.contains(&n)
                    || n.parse::<i64>().is_ok()
                    || [
                        "def", "pop", "swap", "dup", "add", "sub", "mul", "div", "mod", "zero?"
                    ]
                    .contains(&&*n))
                    || (r.is_ok() && m.env.contains_key(&n) && m.to_string().contains(&n))
            );
            prop_assert!(m.stack.is_empty());
        }
        #[test]
        fn custom_ok(ws in prop::collection::vec(word(), 1..64), n in r"custom_word_\S+") {
            let mut m1 = Machine::default();
            let r1 = ws.iter().map(|w| m1.eval(w)).collect::<Result<Vec<()>, _>>();
            let s = format!("def {n} {}", ws.iter().map(|w| w.to_string()).collect::<Vec<_>>().join(" "));
            let mut m2 = Machine::default();
            prop_assert!(m2.read_eval(&s).is_ok());
            prop_assert_eq!(m2.eval(&Word::Custom(n)).is_ok(), r1.is_ok());
        }
    }
}
