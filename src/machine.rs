use crate::{error::Error, token::Token, word::Word};
use indexmap::IndexMap;
use logos::Logos;
use std::{convert::TryFrom, fmt};

/// The main data structure: a stack machine with an environment of local definitions.
#[derive(Default, Debug)]
pub struct Machine {
    env: IndexMap<String, Vec<Word>>,
    stack: Vec<i64>,
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
    fn eval(&mut self, w: &Word) -> Result<(), Error> {
        self.check(w)?;
        eval_inner(&self.env, &mut self.stack, w)
    }
    fn check(&self, w: &Word) -> Result<(), Error> {
        let s = self.stack.len();
        let r = match w {
            Word::Num(_) | Word::Custom(_) => 0,
            Word::Pop | Word::Dup => 1,
            Word::Swap | Word::Add | Word::Sub | Word::Mul | Word::Div | Word::Mod => 2,
            Word::Zero => 3,
        };
        if s < r {
            Err(Error::Small(w.to_string(), r, s))
        } else if (*w == Word::Div || *w == Word::Mod) && self.stack[s - 2] == 0 {
            Err(Error::NNZ(w.to_string()))
        } else if matches!(w, Word::Custom(_)) && !self.env.contains_key(&w.to_string()) {
            Err(Error::Unknown(w.to_string()))
        } else {
            Ok(())
        }
    }
}

/// Broken out to untangle mutability concerns.
/// Full of `stack.pop().expect(…)` because this should only be called from within `Machine::eval`.
fn eval_inner(
    env: &IndexMap<String, Vec<Word>>,
    stack: &mut Vec<i64>,
    w: &Word,
) -> Result<(), Error> {
    match w {
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
            let x = stack.pop().expect("Internal error at zero? 1");
            let y = stack.pop().expect("Internal error at zero? 2");
            let z = stack.pop().expect("Internal error at zero? 3");
            stack.push(if x == 0 { y } else { z });
        }
        Word::Num(n) => stack.push(*n),
        Word::Custom(w) => {
            for v in env.get(w).ok_or_else(|| Error::Unknown(w.to_string()))? {
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
    use std::collections::LinkedList;

    proptest! {
        #[test]
        fn check_implies_ok(ws in prop::collection::vec(word(), 1..100)) {
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
        fn state_machine_testing(sequential 1..100 => Machine);
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

    pub struct Dummy;
    impl ReferenceStateMachine for Dummy {
        type State = LinkedList<i64>;
        type Transition = Op;
        fn init_state() -> BoxedStrategy<Self::State> {
            Just(LinkedList::new()).boxed()
        }
        fn preconditions(state: &Self::State, transition: &Self::Transition) -> bool {
            match transition {
                Op::Push(_) => true,
                Op::Pop | Op::Dup => !state.is_empty(),
                Op::Swap | Op::Add | Op::Sub | Op::Mul => state.len() > 1,
                Op::Div | Op::Mod => {
                    state.len() > 1 && state.iter().nth_back(1).copied().unwrap_or_default() > 0
                }
                Op::Zero => state.len() > 2,
            }
        }
        fn transitions(_: &Self::State) -> BoxedStrategy<Self::Transition> {
            any::<Self::Transition>().boxed()
        }
        fn apply(mut state: Self::State, transition: &Self::Transition) -> Self::State {
            match transition {
                Op::Push(n) => state.push_back(*n),
                Op::Pop => {
                    state.pop_back();
                }
                Op::Swap => {
                    let x = state.pop_back().expect("swap 1");
                    let y = state.pop_back().expect("swap 2");
                    state.push_back(x);
                    state.push_back(y);
                }
                Op::Dup => {
                    let x = state.back().copied().expect("dup");
                    state.push_back(x);
                }
                Op::Add => {
                    let x = state.pop_back().expect("add 1");
                    let y = state.pop_back().expect("add 2");
                    state.push_back(x.wrapping_add(y));
                }
                Op::Sub => {
                    let x = state.pop_back().expect("sub 1");
                    let y = state.pop_back().expect("sub 2");
                    state.push_back(x.wrapping_sub(y));
                }
                Op::Mul => {
                    let x = state.pop_back().expect("mul 1");
                    let y = state.pop_back().expect("mul 2");
                    state.push_back(x.wrapping_mul(y));
                }
                Op::Div => {
                    let x = state.pop_back().expect("div 1");
                    let y = state.pop_back().expect("div 2");
                    state.push_back(x.wrapping_div(y));
                }
                Op::Mod => {
                    let x = state.pop_back().expect("mod 1");
                    let y = state.pop_back().expect("mod 2");
                    state.push_back(x.wrapping_rem(y));
                }
                Op::Zero => {
                    let x = state.pop_back().expect("zero 1");
                    let y = state.pop_back().expect("zero 2");
                    let z = state.pop_back().expect("zero 3");
                    state.push_back(if x == 0 { y } else { z });
                }
            }
            state
        }
    }

    impl StateMachineTest for Machine {
        type SystemUnderTest = Self;
        type Reference = Dummy;
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
