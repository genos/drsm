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
            Token::Custom(w) => Ok(Self::Custom(w.to_string())),
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

#[cfg(test)]
mod tests {
    use super::{super::token::tests::token, *};
    use logos::Logos;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn from_token(t in token()) {
            let w = Word::try_from(t.clone());
            prop_assert_eq!(w.is_ok(), t != Token::Def);
        }
        #[test]
        fn self_eq(w in word()) {
            prop_assert_eq!(w.clone(), w);
        }
        #[test]
        #[allow(clippy::cmp_owned)]
        fn str_eq(w in word(), s in r"\S+") {
            prop_assert_eq!(w == s, w.to_string() == s);
        }
        #[test]
        fn roundtrip(w in word()) {
            let s = w.to_string();
            let ts = Token::lexer(&s).collect::<Result<Vec<Token>, _>>();
            prop_assert!(ts.is_ok());
            let mut ts = ts.unwrap();
            prop_assert_eq!(ts.len(), 1);
            let w2 = Word::try_from(ts.pop().unwrap());
            prop_assert!(w2.is_ok());
            prop_assert_eq!(w, w2.unwrap());
        }
        #[test]
        fn into_name(w in word()) {
            let n = w.clone().into_name();
            prop_assert_eq!(n.is_ok(), w == Word::Custom(n.unwrap_or_default()));
        }
    }

    pub fn word() -> impl Strategy<Value = Word> {
        prop_oneof![
            Just(Word::Pop),
            Just(Word::Swap),
            Just(Word::Dup),
            Just(Word::Add),
            Just(Word::Sub),
            Just(Word::Mul),
            Just(Word::Div),
            Just(Word::Mod),
            Just(Word::Zero),
            any::<i64>().prop_map(Word::Num),
            r"[a-zA-Z]+".prop_map(Word::Custom)
        ]
    }
}
