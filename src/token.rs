use logos::Logos;
use std::fmt;

/// Tokens are lexed from input strings.
#[derive(Logos, Debug, PartialEq, Eq, Clone)]
#[logos(error = crate::Error)]
#[logos(skip r"\s")]
pub enum Token<'source> {
    /// Define a new word.
    #[token("def")]
    Def,
    /// A core word.
    #[regex(r"(drop|swap|dup|add|sub|mul|div|mod|zero[?]|print)")]
    Core(&'source str),
    /// An integer in decimal notation.
    #[regex(r"-?[[:digit:]]+", |lex| lex.slice().parse())]
    Num(i64),
    /// An integer in hexadecimal notation.
    #[regex(r"#[[:xdigit:]]+", |lex| i64::from_str_radix(&lex.slice()[1..], 16))]
    Hex(i64),
    /// A (possibly unknown) custom token.
    #[regex(r"\S+", priority = 0)]
    Custom(&'source str),
}

impl fmt::Display for Token<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Def => f.write_str("def"),
            Self::Core(s) => f.write_str(s),
            Self::Num(n) => write!(f, "{n}"),
            Self::Hex(n) => write!(f, "#{n:x}"),
            Self::Custom(w) => write!(f, "{w}"),
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use logos::Logos;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn roundtrip(t in token()) {
            let s = t.to_string();
            let ts = Token::lexer(&s).collect::<Result<Vec<_>, _>>();
            prop_assert!(ts.is_ok());
            let mut ts = ts.unwrap();
            prop_assert_eq!(ts.len(), 1);
            let t2 = ts.pop().unwrap();
            prop_assert_eq!(t2, t);
        }
        #[test]
        fn custom_roundtrip(s in r"custom_token_\S+") {
            let t = Token::Custom(&s);
            prop_assert_eq!(&t.to_string(), &s);
            let ts = Token::lexer(&s).collect::<Result<Vec<_>, _>>();
            prop_assert!(ts.is_ok());
            let mut ts = ts.unwrap();
            prop_assert_eq!(ts.len(), 1);
            let t2 = ts.pop().unwrap();
            prop_assert_eq!(t2, t);
        }
    }

    // NOTE: I can't get Token::Custom to generate due to lifetime issues, and
    // proptest_derive::Arbitrary doesn't allow generic lifetimes.
    pub fn token() -> impl Strategy<Value = Token<'static>> {
        prop_oneof![
            Just(Token::Def),
            Just(Token::Core("drop")),
            Just(Token::Core("swap")),
            Just(Token::Core("dup")),
            Just(Token::Core("add")),
            Just(Token::Core("sub")),
            Just(Token::Core("mul")),
            Just(Token::Core("div")),
            Just(Token::Core("mod")),
            Just(Token::Core("zero?")),
            Just(Token::Core("print")),
            any::<i64>().prop_map(Token::Num),
            (0..i64::MAX).prop_map(Token::Hex),
        ]
    }
}
