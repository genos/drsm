#![allow(unsafe_code)]
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
                    let _ = self.env.insert(k, us);
                    break;
                }
                _ => self.eval(&Word::try_from(t)?)?,
            }
        }
        Ok(())
    }
    fn eval(&mut self, word: &Word) -> Result<(), Error> {
        check(&self.env, &self.stack, word)?;
        eval_inner(&self.env, &mut self.stack, word)
    }
}

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
/// Full of **UNSAFE** code because this should only be called from within `Machine::eval`.
fn eval_inner(
    env: &IndexMap<String, Vec<Word>>,
    stack: &mut Vec<i64>,
    word: &Word,
) -> Result<(), Error> {
    let n = stack.len();
    match word {
        Word::Pop => unsafe { stack.set_len(n - 1) },
        Word::Swap => stack.swap(n - 1, n - 2),
        Word::Dup => stack.push(unsafe { *stack.get_unchecked(n - 1) }),
        Word::Add => unsafe {
            *stack.get_unchecked_mut(n - 2) = stack
                .get_unchecked(n - 1)
                .wrapping_add(*stack.get_unchecked(n - 2));
            stack.set_len(n - 1);
        },
        Word::Sub => unsafe {
            *stack.get_unchecked_mut(n - 2) = stack
                .get_unchecked(n - 1)
                .wrapping_sub(*stack.get_unchecked(n - 2));
            stack.set_len(n - 1);
        },
        Word::Mul => unsafe {
            *stack.get_unchecked_mut(n - 2) = stack
                .get_unchecked(n - 1)
                .wrapping_mul(*stack.get_unchecked(n - 2));
            stack.set_len(n - 1);
        },
        Word::Div => unsafe {
            *stack.get_unchecked_mut(n - 2) = stack
                .get_unchecked(n - 1)
                .wrapping_div(*stack.get_unchecked(n - 2));
            stack.set_len(n - 1);
        },
        Word::Mod => unsafe {
            *stack.get_unchecked_mut(n - 2) = stack
                .get_unchecked(n - 1)
                .wrapping_rem(*stack.get_unchecked(n - 2));
            stack.set_len(n - 1);
        },
        Word::Zero => unsafe {
            if *stack.get_unchecked(n - 1) == 0 {
                stack.swap(n - 2, n - 3);
            }
            stack.set_len(n - 2);
        },
        Word::Num(x) => {
            stack.push(*x);
        }
        Word::Custom(c) => {
            for v in &env[c] {
                eval_inner(env, stack, v)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{super::word::tests::word, *};
    use proptest::prelude::*;
    use proptest_state_machine::{ReferenceStateMachine, StateMachineTest, prop_state_machine};

    proptest! {
        #[test]
        fn check_implies_ok(ws in prop::collection::vec(word(), 1..512)) {
            let mut m = Machine::default();
            for w in ws {
                if check(&m.env, &m.stack, &w).is_ok() {
                    prop_assert!(m.eval(&w).is_ok(), "Machine with state {m:?} failed on {w}");
                }
            }
        }
    }

    prop_state_machine! {
        #[test]
        fn state_machine_testing(sequential 1..128 => Machine);
    }

    #[derive(Debug, Default, Clone)]
    pub struct SafeMachine {
        env: IndexMap<String, Vec<Word>>,
        stack: Vec<i64>,
    }

    impl ReferenceStateMachine for SafeMachine {
        type State = Self;
        type Transition = Word;
        fn init_state() -> BoxedStrategy<Self::State> {
            Just(Self::default()).boxed()
        }
        fn preconditions(state: &Self::State, transition: &Self::Transition) -> bool {
            check(&state.env, &state.stack, transition).is_ok()
        }
        fn transitions(_: &Self::State) -> BoxedStrategy<Self::Transition> {
            word().boxed()
        }
        fn apply(mut state: Self::State, transition: &Self::Transition) -> Self::State {
            match transition {
                Word::Pop => {
                    state.stack.pop();
                }
                Word::Swap => {
                    let x = state.stack.pop().expect("swap 1");
                    let y = state.stack.pop().expect("swap 2");
                    state.stack.push(x);
                    state.stack.push(y);
                }
                Word::Dup => {
                    let x = state.stack.last().copied().expect("dup");
                    state.stack.push(x);
                }
                Word::Add => {
                    let x = state.stack.pop().expect("add 1");
                    let y = state.stack.pop().expect("add 2");
                    state.stack.push(x.wrapping_add(y));
                }
                Word::Sub => {
                    let x = state.stack.pop().expect("sub 1");
                    let y = state.stack.pop().expect("sub 2");
                    state.stack.push(x.wrapping_sub(y));
                }
                Word::Mul => {
                    let x = state.stack.pop().expect("mul 1");
                    let y = state.stack.pop().expect("mul 2");
                    state.stack.push(x.wrapping_mul(y));
                }
                Word::Div => {
                    let x = state.stack.pop().expect("div 1");
                    let y = state.stack.pop().expect("div 2");
                    state.stack.push(x.wrapping_div(y));
                }
                Word::Mod => {
                    let x = state.stack.pop().expect("mod 1");
                    let y = state.stack.pop().expect("mod 2");
                    state.stack.push(x.wrapping_rem(y));
                }
                Word::Zero => {
                    let x = state.stack.pop().expect("zero 1");
                    let y = state.stack.pop().expect("zero 2");
                    let z = state.stack.pop().expect("zero 3");
                    state.stack.push(if x == 0 { y } else { z });
                }
                Word::Num(n) => state.stack.push(*n),
                Word::Custom(c) => {
                    for v in state.env.get(c).cloned().unwrap_or_default() {
                        state = Self::apply(state, &v);
                    }
                }
            }
            state
        }
    }

    impl StateMachineTest for Machine {
        type SystemUnderTest = Self;
        type Reference = SafeMachine;
        fn init_test(_r: &<Self::Reference as ReferenceStateMachine>::State) -> Self {
            Self::default()
        }
        fn apply(
            mut sut: Self::SystemUnderTest,
            r#ref: &<Self::Reference as ReferenceStateMachine>::State,
            transition: <Self::Reference as ReferenceStateMachine>::Transition,
        ) -> Self::SystemUnderTest {
            sut.eval(&transition).unwrap_or_else(|e| panic!("{transition} errored: {e}"));
            for (x, y) in sut.stack.iter().zip(r#ref.stack.iter()) {
                assert_eq!(x, y, "Different values in stacks: sut={x}, ref={y}");
            }
            sut
        }
    }
}
