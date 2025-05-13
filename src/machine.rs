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
        self.check(word)?;
        eval_inner(&self.env, &mut self.stack, word)
    }
    fn check(&self, word: &Word) -> Result<(), Error> {
        let s = self.stack.len();
        let r = match word {
            Word::Num(_) | Word::Custom(_) => 0,
            Word::Pop | Word::Dup => 1,
            Word::Swap | Word::Add | Word::Sub | Word::Mul | Word::Div | Word::Mod => 2,
            Word::Zero => 3,
        };
        if s < r {
            Err(Error::Small(word.to_string(), r, s))
        } else if (*word == Word::Div || *word == Word::Mod) && self.stack[s - 2] == 0 {
            Err(Error::NNZ(word.to_string()))
        } else if matches!(word, Word::Custom(_)) && !self.env.contains_key(&word.to_string()) {
            Err(Error::Unknown(word.to_string()))
        } else {
            Ok(())
        }
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
                if m.check(&w).is_ok() {
                    prop_assert!(m.eval(&w).is_ok(), "Machine with state {m:?} failed on {w}");
                }
            }
        }
    }

    prop_state_machine! {
        #[test]
        fn state_machine_testing(sequential 1..256 => Machine);
    }

    #[derive(Debug, Clone, Copy, proptest_derive::Arbitrary)]
    pub enum Op {
        Push(i64),
        Pop,
        Swap,
        Dup,
        Add,
        Sub,
        Mul,
        Div,
        Mod,
        Zero,
        // NOTE: no Custom(w)
    }

    impl From<Op> for Word {
        fn from(o: Op) -> Self {
            match o {
                Op::Push(n) => Self::Num(n),
                Op::Pop => Self::Pop,
                Op::Swap => Self::Swap,
                Op::Dup => Self::Dup,
                Op::Add => Self::Add,
                Op::Sub => Self::Sub,
                Op::Mul => Self::Mul,
                Op::Div => Self::Div,
                Op::Mod => Self::Mod,
                Op::Zero => Self::Zero,
            }
        }
    }

    pub struct SafeMachine;
    impl ReferenceStateMachine for SafeMachine {
        type State = Vec<i64>;
        type Transition = Op;
        fn init_state() -> BoxedStrategy<Self::State> {
            Just(Vec::new()).boxed()
        }
        fn preconditions(state: &Self::State, transition: &Self::Transition) -> bool {
            match transition {
                Op::Push(_) => true,
                Op::Pop | Op::Dup => !state.is_empty(),
                Op::Swap | Op::Add | Op::Sub | Op::Mul => state.len() > 1,
                Op::Div | Op::Mod => state.len() > 1 && state[state.len() - 2] > 0,
                Op::Zero => state.len() > 2,
            }
        }
        fn transitions(_: &Self::State) -> BoxedStrategy<Self::Transition> {
            any::<Self::Transition>().boxed()
        }
        fn apply(mut state: Self::State, transition: &Self::Transition) -> Self::State {
            match transition {
                Op::Push(n) => state.push(*n),
                Op::Pop => {
                    state.pop();
                }
                Op::Swap => {
                    let x = state.pop().expect("swap 1");
                    let y = state.pop().expect("swap 2");
                    state.push(x);
                    state.push(y);
                }
                Op::Dup => {
                    let x = state.last().copied().expect("dup");
                    state.push(x);
                }
                Op::Add => {
                    let x = state.pop().expect("add 1");
                    let y = state.pop().expect("add 2");
                    state.push(x.wrapping_add(y));
                }
                Op::Sub => {
                    let x = state.pop().expect("sub 1");
                    let y = state.pop().expect("sub 2");
                    state.push(x.wrapping_sub(y));
                }
                Op::Mul => {
                    let x = state.pop().expect("mul 1");
                    let y = state.pop().expect("mul 2");
                    state.push(x.wrapping_mul(y));
                }
                Op::Div => {
                    let x = state.pop().expect("div 1");
                    let y = state.pop().expect("div 2");
                    state.push(x.wrapping_div(y));
                }
                Op::Mod => {
                    let x = state.pop().expect("mod 1");
                    let y = state.pop().expect("mod 2");
                    state.push(x.wrapping_rem(y));
                }
                Op::Zero => {
                    let x = state.pop().expect("zero 1");
                    let y = state.pop().expect("zero 2");
                    let z = state.pop().expect("zero 3");
                    state.push(if x == 0 { y } else { z });
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
            let w = Word::from(transition);
            sut.eval(&w).unwrap_or_else(|e| panic!("{w} errored: {e}"));
            for (x, y) in sut.stack.iter().zip(r#ref.iter()) {
                assert_eq!(x, y, "Different values in stacks: sut={x}, ref={y}");
            }
            sut
        }
    }
}
