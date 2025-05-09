use logos::Logos;

/// Tokens are lexed from input strings.
#[derive(Logos, Debug, PartialEq, Eq, Clone)]
#[logos(error = crate::Error)]
#[logos(skip r"\s")]
pub enum Token<'source> {
    /// Define a new word.
    #[token("def")]
    Def,
    /// A core word.
    #[regex(r"(pop|swap|dup|add|sub|mul|div|mod|zero)")]
    Core(&'source str),
    /// An integer in decimal notation.
    #[regex(r"-?[0-9]+", |lex| lex.slice().parse())]
    Num(i64),
    /// An integer in hexadecimal notation.
    #[regex(r"#[0-9a-fA-F]+", |lex| i64::from_str_radix(&lex.slice()[1..], 16))]
    Hex(i64),
    /// A (possibly unknown) custom token.
    #[regex(r"\S+", priority = 0)]
    Custom(&'source str),
}
