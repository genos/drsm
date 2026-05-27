use crate::core::Core;
use logos::Logos;

/// Tokens are lexed from input strings.
#[derive(Logos, Debug, PartialEq, Eq, Clone, strum::Display)]
#[cfg_attr(test, derive(chaos_theory::Arbitrary))]
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
    Hex(#[cfg_attr(test, chaos_theory(generator = chaos_theory::make::int_in_range(0..)))] i64),
    /// A (possibly unknown) custom token.
    #[regex(r"\S+", priority = 0)]
    #[strum(serialize = "{0}")]
    Custom(
        #[cfg_attr(test, chaos_theory(generator = chaos_theory::make::just("custom_token")))]
        &'source str,
    ),
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use chaos_theory::check;
    use logos::Logos;

    #[test]
    fn roundtrip() {
        check(|src| {
            let t = src.any::<Token>("token");
            let s = t.to_string();
            let ts = Token::lexer(&s).collect::<Result<Vec<_>, _>>();
            assert!(ts.is_ok());
            let mut ts = ts.expect("is_ok");
            assert_eq!(ts.len(), 1);
            let t2 = ts.pop().expect("len == 1");
            assert_eq!(t2, t);
        });
    }
}
