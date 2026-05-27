use crate::{Error, core::Core, token::Token};
use lean_string::LeanString;
use std::convert::TryFrom;

/// The words upon which our stack machine works.
#[derive(Debug, PartialEq, Eq, Clone, strum::Display)]
#[cfg_attr(test, derive(chaos_theory::Arbitrary))]
pub enum Word {
    /// A core word,
    #[strum(serialize = "{0}")]
    Core(Core),
    /// An integer.
    #[strum(serialize = "{0}")]
    Num(i64),
    /// A custom word.
    #[strum(serialize = "{0}")]
    Custom(#[cfg_attr(test, chaos_theory(generator = custom()))] LeanString),
}

#[cfg(test)]
fn custom() -> impl chaos_theory::Generator<Item = LeanString> {
    chaos_theory::make::from_fn(|src| {
        let s = src.any_of(
            "custom word",
            chaos_theory::make::string_matching(r"custom_[a-zA-Z]+", true),
        );
        LeanString::from(&s)
    })
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
    /// Unsafely grab the inner `LeanString` of this custom word.
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
    use super::*;
    use chaos_theory::check;
    use logos::Logos;

    #[test]
    fn from_token() {
        check(|src| {
            let t = src.any::<Token>("token");
            let w = Word::try_from(t.clone());
            assert_eq!(w.is_ok(), t != Token::Def);
        });
    }

    #[test]
    fn self_eq() {
        check(|src| {
            let w = src.any::<Word>("word");
            assert_eq!(w.clone(), w);
        });
    }

    #[test]
    #[allow(clippy::cmp_owned)]
    fn str_eq() {
        check(|src| {
            let w = src.any::<Word>("word");
            let s = src.any_of("string", chaos_theory::make::string_matching(r"\S+", true));
            assert_eq!(w == s, w.to_string() == s);
        });
    }

    #[test]
    fn roundtrip() {
        check(|src| {
            let w = src.any::<Word>("word");
            let s = w.to_string();
            let ts = Token::lexer(&s).collect::<Result<Vec<Token>, _>>();
            assert!(ts.is_ok());
            let mut ts = ts.expect("is_ok");
            assert_eq!(ts.len(), 1);
            let w2 = Word::try_from(ts.pop().expect("len == 1"));
            assert!(w2.is_ok());
            assert_eq!(w2.expect("is_ok"), w);
        });
    }

    #[test]
    fn into_name() {
        check(|src| {
            let w = src.any::<Word>("word");
            let n = w.clone().into_name();
            assert_eq!(n.is_ok(), w == Word::Custom(n.unwrap_or_default()));
        });
    }
}
