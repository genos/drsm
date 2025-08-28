use crate::{core::Core, error::Error, token::Token, word::Word};
use indexmap::IndexMap;
use logos::Logos;
use std::{convert::TryFrom, fmt};
use strum::IntoEnumIterator;

/// The main data structure: a stack machine with an environment of local definitions.
#[derive(Debug)]
pub struct Machine {
    env: IndexMap<String, Vec<Word>>,
    stack: Vec<i64>,
}

impl Default for Machine {
    fn default() -> Self {
        Self {
            env: IndexMap::with_capacity(64),
            stack: Vec::with_capacity(64),
        }
    }
}

impl fmt::Display for Machine {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("core:")?;
        for c in Core::iter() {
            write!(f, " {c}")?;
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
    /// Look for a definition in the environment.
    #[must_use]
    pub fn lookup(&self, s: &str) -> Option<String> {
        self.env.get(s).map(|d| {
            d.iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ")
        })
    }
    /// `check` the input, then run it through `eval_inner`.
    fn eval(&mut self, word: &Word) -> Result<(), Error> {
        check(&self.env, &self.stack, word)?;
        eval_inner(&self.env, &mut self.stack, word)
    }
}

/// Broken out because `eval_inner` is separate, too, and requires this.
fn check(env: &IndexMap<String, Vec<Word>>, stack: &[i64], word: &Word) -> Result<(), Error> {
    let s = stack.len();
    let r = match word {
        Word::Num(_) | Word::Custom(_) => 0,
        Word::Core(c) => match c {
            Core::Drop | Core::Dup | Core::Print => 1,
            Core::Swap | Core::Add | Core::Sub | Core::Mul | Core::Div | Core::Mod => 2,
            Core::Zero => 3,
        },
    };
    if s < r {
        Err(Error::Small(word.to_string(), r, s))
    } else if matches!(word, Word::Core(Core::Div | Core::Mod)) && stack[s - 2] == 0 {
        Err(Error::NotNonzero(word.to_string()))
    } else if *word == Word::Core(Core::Mod) && matches!(stack[s - 2..s], [-1, i64::MIN]) {
        Err(Error::ModEdge)
    } else if matches!(word, Word::Custom(_)) && !env.contains_key(&word.to_string()) {
        Err(Error::Unknown(word.to_string()))
    } else {
        Ok(())
    }
}

/// Broken out to untangle mutability concerns.
/// Full of `stack.pop().expect(…)` because this should _only_ be called from within `Machine::eval`.
fn eval_inner(
    env: &IndexMap<String, Vec<Word>>,
    stack: &mut Vec<i64>,
    word: &Word,
) -> Result<(), Error> {
    match word {
        Word::Core(Core::Drop) => {
            stack.pop().expect("Internal error @ drop");
        }
        Word::Core(Core::Swap) => {
            let x = stack.pop().expect("Internal error @ swap 1");
            let y = stack.pop().expect("Internal error @ swap 2");
            stack.push(x);
            stack.push(y);
        }
        Word::Core(Core::Dup) => {
            let x = stack.pop().expect("Internal error @ dup");
            stack.push(x);
            stack.push(x);
        }
        Word::Core(Core::Add) => {
            let x = stack.pop().expect("Internal error @ add 1");
            let y = stack.pop().expect("Internal error @ add 2");
            stack.push(x.saturating_add(y));
        }
        Word::Core(Core::Sub) => {
            let x = stack.pop().expect("Internal error @ sub 1");
            let y = stack.pop().expect("Internal error @ sub 2");
            stack.push(x.saturating_sub(y));
        }
        Word::Core(Core::Mul) => {
            let x = stack.pop().expect("Internal error @ mul 1");
            let y = stack.pop().expect("Internal error @ mul 2");
            stack.push(x.saturating_mul(y));
        }
        Word::Core(Core::Div) => {
            let x = stack.pop().expect("Internal error @ div 1");
            let y = stack.pop().expect("Internal error @ div 2");
            stack.push(x.saturating_div(y));
        }
        Word::Core(Core::Mod) => {
            let x = stack.pop().expect("Internal error @ mod 1");
            let y = stack.pop().expect("Internal error @ mod 2");
            stack.push(x.rem_euclid(y));
        }
        Word::Core(Core::Zero) => {
            let x = stack.pop().expect("Internal error @ zero? 1");
            let y = stack.pop().expect("Internal error @ zero? 2");
            let z = stack.pop().expect("Internal error @ zero? 3");
            stack.push(if x == 0 { y } else { z });
        }
        Word::Core(Core::Print) => println!("{}", stack.pop().expect("Internal error @ print")),
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
    use std::string::ToString;

    #[test]
    fn def_errs() {
        for s in [
            "def",
            "def name",
            "def def drop",
            "def drop body",
            "def name name",
        ] {
            assert!(Machine::default().read_eval(s).is_err());
        }
    }
    #[test]
    fn num_errs() {
        for s in ["0 1 div", "0 1 mod", "-1 -9223372036854775808 mod"] {
            assert!(Machine::default().read_eval(s).is_err());
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
                prop_assert_eq!(check(&m.env, &m.stack, &w).is_ok(), m.eval(&w).is_ok());
            }
        }
        #[test]
        fn check_implies_read_eval(ws in prop::collection::vec(word(), 0..64)) {
            let mut m = Machine::default();
            for w in ws {
                prop_assert_eq!(check(&m.env, &m.stack, &w).is_ok(), m.read_eval(&w.to_string()).is_ok());
            }
        }
        #[test]
        fn def_adds_to_env(ws in prop::collection::vec(r"\S+", 0..64), n in r"custom_name_\S+") {
            let mut m = Machine::default();
            let d = ws.join(" ");
            let s = format!("def {n} {d}");
            let r = m.read_eval(&s);
            prop_assert!(
                (ws.is_empty()
                    || ws.contains(&n)
                    || n.parse::<i64>().is_ok()
                    || [
                        "def", "pop", "swap", "dup", "add", "sub", "mul", "div", "mod", "zero?", "print"
                    ]
                    .contains(&&*n))
                    || (r.is_ok() && m.lookup(&n).is_some() && m.env.contains_key(&n) && m.to_string().contains(&n))
            );
            prop_assert!(m.stack.is_empty());
        }
        #[test]
        fn custom_ok(ws in prop::collection::vec(word(), 1..64), n in r"custom_word_\S+") {
            let mut m1 = Machine::default();
            let r1 = ws.iter().map(|w| m1.eval(w)).collect::<Result<Vec<()>, _>>();
            let s = format!("def {n} {}", ws.iter().map(std::string::ToString::to_string).collect::<Vec<_>>().join(" "));
            let mut m2 = Machine::default();
            prop_assert!(m2.read_eval(&s).is_ok());
            prop_assert_eq!(m2.eval(&Word::Custom(n)).is_ok(), r1.is_ok());
        }
    }
}
