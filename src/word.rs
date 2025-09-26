use crate::{Error, core::Core, token::Token};
use lean_string::LeanString;
use std::convert::TryFrom;

/// The words upon which our stack machine works.
#[derive(Debug, PartialEq, Eq, Clone, strum::Display)]
pub enum Word {
    /// A core word,
    #[strum(serialize = "{0}")]
    Core(Core),
    /// An integer.
    #[strum(serialize = "{0}")]
    Num(i64),
    /// A custom word.
    #[strum(serialize = "{0}")]
    Custom(LeanString),
}

impl TryFrom<Token<'_>> for Word {
    type Error = Error;
    fn try_from(t: Token<'_>) -> Result<Self, Self::Error> {
        match t {
            Token::Def => Err(Error::DefReserved),
            Token::Core(c) => Ok(Self::Core(c)),
            Token::Num(n) | Token::Hex(n) => Ok(Self::Num(n)),
            Token::Custom(w) => Ok(Self::Custom(LeanString::from(w))),
        }
    }
}

impl PartialEq<String> for Word {
    fn eq(&self, s: &String) -> bool {
        matches!(self, Self::Custom(w) if w == s)
    }
}

impl PartialEq<LeanString> for Word {
    fn eq(&self, s: &LeanString) -> bool {
        matches!(self, Self::Custom(w) if w == s)
    }
}

impl Word {
    /// Transform this word into a name, if possible.
    ///
    /// # Errors
    /// If the word is a number or a core word.
    pub fn into_name(self) -> Result<LeanString, Error> {
        match self {
            Self::Custom(w) => Ok(w),
            Self::Num(n) => Err(Error::NumNotName(n)),
            Self::Core(_) => Err(Error::CoreNotName(self.to_string())),
        }
    }
    /// Unsafely grab the inner lean string of this custom word.
    ///
    /// # Panics
    /// If this isn't a custom word.
    pub(crate) fn unsafe_custom_inner(&self) -> &LeanString {
        match self {
            Self::Custom(w) => w,
            _ => panic!("Unsafe custom inner called on non-custom word {self}"),
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::{
        super::{core::tests::core, token::tests::token},
        *,
    };
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
            let mut ts = ts.expect("is_ok");
            prop_assert_eq!(ts.len(), 1);
            let w2 = Word::try_from(ts.pop().expect("len == 1"));
            prop_assert!(w2.is_ok());
            prop_assert_eq!(w2.expect("is_ok"), w);
        }
        #[test]
        fn into_name(w in word()) {
            let n = w.clone().into_name();
            prop_assert_eq!(n.is_ok(), w == Word::Custom(n.unwrap_or_default()));
        }
    }

    pub fn word() -> impl Strategy<Value = Word> {
        prop_oneof![
            core().prop_map(Word::Core),
            any::<i64>().prop_map(Word::Num),
            r"custom_[a-zA-Z]+".prop_map(|s| Word::Custom(s.into()))
        ]
    }
}
