use crate::{core::Core, error::Error, token::Token, word::Word};
use indexmap::IndexMap;
use lean_string::LeanString;
use logos::Logos;
use std::{convert::TryFrom, fmt};
use strum::IntoEnumIterator;

/// The main data structure: a stack machine with an environment of local definitions.
#[derive(Debug)]
pub struct Machine {
    env: IndexMap<LeanString, Vec<Word>>,
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
                    return Err(Error::SelfRef(k.to_string()));
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
        check_word(&self.env, &self.stack, word)?;
        eval_inner(&self.env, &mut self.stack, word)
    }
}

/// Broken out because `eval_inner` is separate, too, and requires this.
fn check_word(
    env: &IndexMap<LeanString, Vec<Word>>,
    stack: &[i64],
    word: &Word,
) -> Result<(), Error> {
    let s = stack.len();
    let r = match word {
        Word::Num(_) | Word::Custom(_) => 0,
        Word::Core(c) => match c {
            Core::Drop | Core::Dup | Core::Print => 1,
            Core::Swap | Core::Add | Core::Sub | Core::Mul | Core::Div | Core::Mod => 2,
            Core::Zero | Core::Rot => 3,
        },
    };
    if s < r {
        Err(Error::Small(word.to_string(), r, s))
    } else if matches!(word, Word::Core(Core::Div | Core::Mod)) && stack[s - 2] == 0 {
        Err(Error::NotNonzero(word.to_string()))
    } else if *word == Word::Core(Core::Mod) && matches!(stack[s - 2..s], [-1, i64::MIN]) {
        Err(Error::ModEdge)
    } else if matches!(word, Word::Custom(_)) && !env.contains_key(word.unsafe_custom_inner()) {
        Err(Error::Unknown(word.to_string()))
    } else {
        Ok(())
    }
}

/// Broken out to untangle mutability concerns.
/// Full of `stack.pop().expect(…)` because this should _only_ be called from within `Machine::eval`.
fn eval_inner(
    env: &IndexMap<LeanString, Vec<Word>>,
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
        Word::Core(Core::Rot) => {
            let x = stack.pop().expect("Internal error @ rot 1");
            let y = stack.pop().expect("Internal error @ rot 2");
            let z = stack.pop().expect("Internal error @ rot 3");
            stack.push(z);
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
                check_word(env, stack, w)?;
                eval_inner(env, stack, w)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chaos_theory::{check, make};
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

    fn fib_machine(n: i64) -> Result<i64, Error> {
        if n >= 93 {
            Err(Error::Bad)
        } else {
            let mut m = Machine::default();
            (0..n - 1)
                .map(|i| m.read_eval(&format!("def fib_{} fib_{} fib_{i} add", i + 2, i + 1)))
                .collect::<Result<Vec<_>, _>>()?;
            m.read_eval("def fib_1 1")?;
            m.read_eval("def fib_0 1")?;
            m.read_eval(&format!("fib_{n}"))?;
            m.stack.pop().ok_or(Error::Bad)
        }
    }

    #[test]
    fn pushing_extends_stack() {
        check(|src| {
            let ns = src.any_of("ns", make::vec_with_size(make::arbitrary(), 1..64));
            let mut m = Machine::default();
            let mut old = m.to_string().len();
            for n in ns {
                assert!(m.eval(&Word::Num(n)).is_ok());
                let new = m.to_string().len();
                assert_eq!(new - old, format!(" {n}").len());
                old = new;
            }
        });
    }

    #[test]
    fn check_implies_eval() {
        check(|src| {
            let ws = src.any_of("ws", make::vec_with_size(make::arbitrary(), 0..64));
            let mut m = Machine::default();
            for w in ws {
                assert_eq!(check_word(&m.env, &m.stack, &w).is_ok(), m.eval(&w).is_ok());
            }
        });
    }

    #[test]
    fn check_implies_read_eval() {
        check(|src| {
            let ws = src.any_of("ws", make::vec_with_size(make::arbitrary(), 0..64));
            let mut m = Machine::default();
            for w in ws {
                assert_eq!(
                    check_word(&m.env, &m.stack, &w).is_ok(),
                    m.read_eval(&w.to_string()).is_ok()
                );
            }
        });
    }

    #[test]
    fn def_adds_to_env() {
        check(|src| {
            let ws = src.any_of(
                "ws",
                make::vec_with_size(make::string_matching(r"custom_name_\S+", true), 0..64),
            );
            let n = src.any_of("n", make::string_matching(r"custom_name_\S+", true));
            let mut m = Machine::default();
            let d = ws.join(" ");
            let r = m.read_eval(&format!("def {n} {d}"));
            assert!(
                (ws.is_empty()
                    || ws.contains(&n)
                    || n.parse::<i64>().is_ok()
                    || [
                        "def", "pop", "swap", "rot", "dup", "add", "sub", "mul", "div", "mod",
                        "zero?", "print",
                    ]
                    .contains(&&*n))
                    || (r.is_ok()
                        && m.lookup(&n).is_some()
                        && m.env.contains_key(&LeanString::from(n.clone()))
                        && m.to_string().contains(&n))
            );
            assert!(m.stack.is_empty());
        });
    }

    #[test]
    fn custom_ok() {
        check(|src| {
            let ws = src.any_of("ws", make::vec_with_size(make::arbitrary(), 1..64));
            let n = src.any_of("n", make::string_matching(r"custom_name_\S+", true));
            let mut m1 = Machine::default();
            let r1 = ws
                .iter()
                .map(|w| m1.eval(w))
                .collect::<Result<Vec<()>, _>>();
            let s = format!(
                "def {n} {}",
                ws.iter()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            let mut m2 = Machine::default();
            assert!(m2.read_eval(&s).is_ok());
            assert_eq!(m2.eval(&Word::Custom(n.into())).is_ok(), r1.is_ok());
        });
    }

    #[test]
    fn fib() {
        check(|src| {
            let n = src.any_of("n", make::int_in_range(0..16));
            let (mut a, mut b) = (1, 1);
            for _ in 1..n {
                let t = a + b;
                a = b;
                b = t;
            }
            let r = fib_machine(n);
            assert!(r.is_ok());
            assert_eq!(r.expect("is_ok"), b);
        });
    }
}
