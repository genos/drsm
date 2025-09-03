use crate::core::Core;
use logos::Logos;

/// Tokens are lexed from input strings.
#[derive(Logos, Debug, PartialEq, Eq, Clone, strum::Display)]
#[logos(skip r"\s", error = crate::Error)]
pub enum Token<'source> {
    /// Define a new word.
    #[token("def")]
    #[strum(serialize = "def")]
    Def,
    /// A core word.
    #[regex(r"(drop|swap|dup|add|sub|mul|div|mod|zero[?]|print)", |lex| lex.slice().parse::<Core>().unwrap())]
    #[strum(serialize = "{0}")]
    Core(Core),
    /// An integer in decimal notation.
    #[regex(r"-?[[:digit:]]+", |lex| lex.slice().parse())]
    #[strum(serialize = "{0}")]
    Num(i64),
    /// An integer in hexadecimal notation.
    #[regex(r"#[[:xdigit:]]+", |lex| i64::from_str_radix(&lex.slice()[1..], 16))]
    #[strum(serialize = "#{0:x}")]
    Hex(i64),
    /// A (possibly unknown) custom token.
    #[regex(r"\S+", priority = 0)]
    #[strum(serialize = "{0}")]
    Custom(&'source str),
}

#[cfg(test)]
pub mod tests {
    use super::{super::core::tests::core, *};
    use logos::Logos;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn roundtrip(t in token()) {
            let s = t.to_string();
            let ts = Token::lexer(&s).collect::<Result<Vec<_>, _>>();
            prop_assert!(ts.is_ok());
            let mut ts = ts.expect("is_ok");
            prop_assert_eq!(ts.len(), 1);
            let t2 = ts.pop().expect("len == 1");
            prop_assert_eq!(t2, t);
        }
        #[test]
        fn custom_roundtrip(s in r"custom_token_\S+") {
            let t = Token::Custom(&s);
            prop_assert_eq!(&t.to_string(), &s);
            let ts = Token::lexer(&s).collect::<Result<Vec<_>, _>>();
            prop_assert!(ts.is_ok());
            let mut ts = ts.expect("is_ok");
            prop_assert_eq!(ts.len(), 1);
            let t2 = ts.pop().expect("len == 1");
            prop_assert_eq!(t2, t);
        }
    }

    // NOTE: I can't get Token::Custom to generate due to lifetime issues, and
    // proptest_derive::Arbitrary doesn't allow generic lifetimes.
    pub fn token() -> impl Strategy<Value = Token<'static>> {
        prop_oneof![
            Just(Token::Def),
            core().prop_map(Token::Core),
            any::<i64>().prop_map(Token::Num),
            (0..i64::MAX).prop_map(Token::Hex),
        ]
    }
}
