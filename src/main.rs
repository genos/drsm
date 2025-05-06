use indexmap::IndexMap;
use logos::Logos;
use rustyline::{error::ReadlineError, Config, EditMode, Editor};
use std::{fmt, num::ParseIntError};

#[derive(Logos, Debug, PartialEq, Eq, Clone)]
#[logos(error = Error)]
#[logos(skip r"\s")]
enum Token {
    #[token("def")]
    Def,
    #[token("pop")]
    Pop,
    #[token("swap")]
    Swap,
    #[token("dup")]
    Dup,
    #[token("add")]
    Add,
    #[token("sub")]
    Sub,
    #[token("mul")]
    Mul,
    #[token("div")]
    Div,
    #[token("mod")]
    Mod,
    #[regex(r"[0-9]+", |lex| lex.slice().parse().map_err(|e: ParseIntError| Error::Parsing(e.to_string())), priority = 3)]
    Num(i64),
    #[regex(r"\w+", |lex| lex.slice().to_owned())]
    Word(String),
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Def => f.write_str("def"),
            Self::Pop => f.write_str("pop"),
            Self::Swap => f.write_str("swap"),
            Self::Dup => f.write_str("dup"),
            Self::Add => f.write_str("add"),
            Self::Sub => f.write_str("sub"),
            Self::Mul => f.write_str("mul"),
            Self::Div => f.write_str("div"),
            Self::Mod => f.write_str("modf"),
            Self::Num(n) => write!(f, "{n}"),
            Self::Word(w) => write!(f, "{w}"),
        }
    }
}

impl Token {
    fn into_name(self) -> Result<String, Error> {
        match self {
            Self::Word(w) => Ok(w),
            Self::Def => Err(Error::Reserved),
            Self::Num(n) => Err(Error::DefNum(n)),
            _ => Err(Error::NaName(self)),
        }
    }
    fn into_num(self) -> Result<i64, Error> {
        match self {
            Self::Num(n) => Ok(n),
            _ => Err(Error::NaN(self)),
        }
    }
}

type Env = IndexMap<String, Vec<Token>>;

fn default_env() -> Env {
    Env::from_iter([
        ("pop".to_string(), vec![Token::Pop]),
        ("swap".to_string(), vec![Token::Swap]),
        ("dup".to_string(), vec![Token::Dup]),
        ("add".to_string(), vec![Token::Add]),
        ("sub".to_string(), vec![Token::Sub]),
        ("mul".to_string(), vec![Token::Mul]),
        ("div".to_string(), vec![Token::Div]),
        ("mod".to_string(), vec![Token::Mod]),
    ])
}

type Stack = Vec<Token>;

fn pop(stack: &mut Stack, t: &Token, required: usize, stack_len: usize) -> Result<Token, Error> {
    stack
        .pop()
        .ok_or(Error::Small(t.clone(), required, stack_len))
}

fn eval(stack: &mut Vec<Token>, env: &Env, t: &Token) -> Result<(), Error> {
    match t {
        Token::Def => return Err(Error::Reserved),
        Token::Num(_) => stack.push(t.clone()),
        Token::Pop => pop(stack, t, 1, 0).map(|_| ())?,
        Token::Swap => {
            let x = pop(stack, t, 2, 0)?;
            let y = pop(stack, t, 2, 1)?;
            stack.push(x);
            stack.push(y);
        }
        Token::Dup => {
            let x = pop(stack, t, 1, 0)?;
            stack.push(x.clone());
            stack.push(x);
        }
        Token::Add => {
            let x = pop(stack, t, 2, 0).and_then(Token::into_num)?;
            let y = pop(stack, t, 2, 1).and_then(Token::into_num)?;
            stack.push(Token::Num(x + y));
        }
        Token::Sub => {
            let x = pop(stack, t, 2, 0).and_then(Token::into_num)?;
            let y = pop(stack, t, 2, 1).and_then(Token::into_num)?;
            stack.push(Token::Num(x - y));
        }
        Token::Mul => {
            let x = pop(stack, t, 2, 0).and_then(Token::into_num)?;
            let y = pop(stack, t, 2, 1).and_then(Token::into_num)?;
            stack.push(Token::Num(x * y));
        }
        Token::Div => {
            let x = pop(stack, t, 2, 0).and_then(Token::into_num)?;
            let y = pop(stack, t, 2, 1).and_then(Token::into_num)?;
            stack.push(Token::Num(x / y));
        }
        Token::Mod => {
            let x = pop(stack, t, 2, 0).and_then(Token::into_num)?;
            let y = pop(stack, t, 2, 1).and_then(Token::into_num)?;
            stack.push(Token::Num(x % y));
        }
        Token::Word(w) => match env.get(w) {
            None => return Err(Error::Unknown(w.to_string())),
            Some(vs) => {
                for v in vs {
                    eval(stack, env, v)?;
                }
            }
        },
    }
    Ok(())
}

fn read_eval_print(stack: &mut Stack, env: &mut Env, s: &str) -> Result<(), Error> {
    let mut ts = Token::lexer(s).filter_map(Result::ok);
    while let Some(t) = ts.next() {
        match t {
            Token::Def => {
                let k = ts.next().ok_or(Error::DefName).and_then(Token::into_name)?;
                let us = ts.collect::<Vec<_>>();
                if us.is_empty() {
                    return Err(Error::DefBody);
                }
                let _ = env.insert(k, us);
                break;
            }
            _ => eval(stack, env, &t)?,
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Default, PartialEq, thiserror::Error)]
enum Error {
    #[default]
    #[error("Something bad happened.")]
    Bad,
    #[error("I expected a number, but I found `{0}`.")]
    NaN(Token),
    #[error("I expected a name, but I found `{0}`.")]
    NaName(Token),
    #[error("The stack is too small for `{0}`; it requires {1}, but the stack only has {2}.")]
    Small(Token, usize, usize),
    #[error("Parsing error: `{0}`.")]
    Parsing(String),
    #[error("Something happend when trying to read: {0}.")]
    Readline(String),
    #[error("Unknown op: `{0}`.")]
    Unknown(String),
    #[error("`def` is a reserved keyword.")]
    Reserved,
    #[error("`def` needs a name, but none was supplied.")]
    DefName,
    #[error("`def` needs a name, but a number `{0}` was supplied.")]
    DefNum(i64),
    #[error("`def` needs a body, but none was supplied.")]
    DefBody,
}

fn main() -> Result<(), Error> {
    let config = Config::builder().edit_mode(EditMode::Vi).build();
    let mut rl: Editor<(), _> =
        Editor::with_config(config).map_err(|e| Error::Readline(e.to_string()))?;
    let mut e = default_env();
    let mut s = Stack::default();
    println!(
        r"
    ____  ____  _____ __  ___
   / __ \/ __ \/ ___//  |/  /
  / / / / /_/ /\__ \/ /|_/ /
 / /_/ / _, _/___/ / /  / /
/_____/_/ |_|/____/_/  /_/
"
    );
    if rl.load_history("history.txt").is_err() {
        println!("No previous history.");
    }
    loop {
        match rl.readline(">  ") {
            Ok(l) => {
                rl.add_history_entry(l.as_str())
                    .map_err(|e| Error::Readline(e.to_string()))?;
                match read_eval_print(&mut s, &mut e, &l) {
                    Ok(()) => {}
                    Err(e) => eprintln!("{e}"),
                }
            }
            Err(ReadlineError::Interrupted) => {
                eprintln!("^C");
            }
            Err(ReadlineError::Eof) => {
                eprintln!("Bye!");
                break;
            }
            Err(err) => {
                eprintln!("Error: {err:?}");
            }
        }
        print!("env:  ");
        for k in e.keys().rev() {
            print!(" {k}");
        }
        print!("\nstack: [");
        for t in s.iter().rev() {
            print!(" {t}");
        }
        println!(" ]");
    }
    rl.save_history("history.txt")
        .map_err(|e| Error::Readline(e.to_string()))?;
    Ok(())
}
