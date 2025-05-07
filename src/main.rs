use indexmap::IndexMap;
use logos::Logos;
use rustyline::{Config, EditMode, Editor, error::ReadlineError};
use std::{cell::RefCell, fmt, num::ParseIntError};

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
    #[token("zero?")]
    Zero,
    #[regex(r"-?[0-9]+", |lex| lex.slice().parse().map_err(|e: ParseIntError| Error::Parsing(e.to_string())), priority = 3)]
    Num(i64),
    #[regex(r"#[0-9a-fA-F]+", |lex| i64::from_str_radix(&lex.slice()[1..], 16).map_err(|e: ParseIntError| Error::Parsing(e.to_string())))]
    Hex(i64),
    #[regex(r"\S+", |lex| lex.slice().to_owned())]
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
            Self::Mod => f.write_str("mod"),
            Self::Zero => f.write_str("zero?"),
            Self::Num(n) => write!(f, "{n}"),
            Self::Hex(n) => write!(f, "#{n:x}"),
            Self::Word(w) => write!(f, "{w}"),
        }
    }
}

impl Token {
    fn into_name(self) -> Result<String, Error> {
        match self {
            Self::Word(w) => Ok(w),
            Self::Def => Err(Error::Reserved),
            Self::Num(n) | Self::Hex(n) => Err(Error::DefNum(n)),
            _ => Ok(self.to_string()),
        }
    }
    fn into_num(self) -> Result<i64, Error> {
        match self {
            Self::Num(n) | Self::Hex(n) => Ok(n),
            _ => Err(Error::NaN(self)),
        }
    }
}

struct Machine {
    env: RefCell<IndexMap<String, Vec<Token>>>,
    stack: RefCell<Vec<Token>>,
}

impl Default for Machine {
    fn default() -> Self {
        Self {
            env: RefCell::new(IndexMap::from_iter([
                (Token::Pop.to_string(), vec![Token::Pop]),
                (Token::Swap.to_string(), vec![Token::Swap]),
                (Token::Dup.to_string(), vec![Token::Dup]),
                (Token::Add.to_string(), vec![Token::Add]),
                (Token::Sub.to_string(), vec![Token::Sub]),
                (Token::Mul.to_string(), vec![Token::Mul]),
                (Token::Div.to_string(), vec![Token::Div]),
                (Token::Mod.to_string(), vec![Token::Mod]),
                (Token::Zero.to_string(), vec![Token::Zero]),
            ])),
            stack: RefCell::default(),
        }
    }
}

impl fmt::Display for Machine {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("env:")?;
        for k in self.env.borrow().keys().rev() {
            write!(f, " {k}")?;
        }
        f.write_str("\nstack: [")?;
        for t in self.stack.borrow().iter().rev() {
            write!(f, " {t}")?;
        }
        f.write_str(" ]")
    }
}

impl Machine {
    fn pop(&self, t: &Token, required: usize, stack_len: usize) -> Result<Token, Error> {
        self.stack
            .borrow_mut()
            .pop()
            .ok_or(Error::Small(t.clone(), required, stack_len))
    }
    fn eval(&self, t: &Token) -> Result<(), Error> {
        match t {
            Token::Def => return Err(Error::Reserved),
            Token::Num(_) | Token::Hex(_) => self.stack.borrow_mut().push(t.clone()),
            Token::Pop => self.pop(t, 1, 0).map(|_| ())?,
            Token::Swap => {
                let x = self.pop(t, 2, 0)?;
                let y = self.pop(t, 2, 1)?;
                self.stack.borrow_mut().push(x);
                self.stack.borrow_mut().push(y);
            }
            Token::Dup => {
                let x = self.pop(t, 1, 0)?;
                self.stack.borrow_mut().push(x.clone());
                self.stack.borrow_mut().push(x);
            }
            Token::Add => {
                let x = self.pop(t, 2, 0).and_then(Token::into_num)?;
                let y = self.pop(t, 2, 1).and_then(Token::into_num)?;
                self.stack.borrow_mut().push(Token::Num(x + y));
            }
            Token::Sub => {
                let x = self.pop(t, 2, 0).and_then(Token::into_num)?;
                let y = self.pop(t, 2, 1).and_then(Token::into_num)?;
                self.stack.borrow_mut().push(Token::Num(x - y));
            }
            Token::Mul => {
                let x = self.pop(t, 2, 0).and_then(Token::into_num)?;
                let y = self.pop(t, 2, 1).and_then(Token::into_num)?;
                self.stack.borrow_mut().push(Token::Num(x * y));
            }
            Token::Div => {
                let x = self.pop(t, 2, 0).and_then(Token::into_num)?;
                let y = self.pop(t, 2, 1).and_then(Token::into_num)?;
                self.stack.borrow_mut().push(Token::Num(x / y));
            }
            Token::Mod => {
                let x = self.pop(t, 2, 0).and_then(Token::into_num)?;
                let y = self.pop(t, 2, 1).and_then(Token::into_num)?;
                self.stack.borrow_mut().push(Token::Num(x % y));
            }
            Token::Zero => {
                let x = self.pop(t, 3, 0).and_then(Token::into_num)?;
                let y = self.pop(t, 3, 1)?;
                let z = self.pop(t, 3, 2)?;
                self.stack.borrow_mut().push(if x == 0 { y } else { z });
            }
            Token::Word(w) => match self.env.borrow().get(w) {
                None => return Err(Error::Unknown(w.to_string())),
                Some(vs) => {
                    for v in vs {
                        self.eval(v)?;
                    }
                }
            },
        }
        Ok(())
    }
    fn read_eval_print(&self, s: &str) -> Result<(), Error> {
        let mut ts = Token::lexer(s).filter_map(Result::ok);
        while let Some(t) = ts.next() {
            match t {
                Token::Def => {
                    let k = ts.next().ok_or(Error::DefName).and_then(Token::into_name)?;
                    let us = ts.collect::<Vec<_>>();
                    if us.is_empty() {
                        return Err(Error::DefBody);
                    } else if us.iter().any(|u| u.to_string() == k) {
                        return Err(Error::SelfRef(k));
                    }
                    let _ = self.env.borrow_mut().insert(k, us);
                    break;
                }
                _ => self.eval(&t)?,
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, thiserror::Error)]
enum Error {
    #[default]
    #[error("Something bad happened.")]
    Bad,
    #[error("I expected a number, but I found `{0}`.")]
    NaN(Token),
    #[error("The stack is too small for `{0}`; it requires {1}, but the stack only has {2}.")]
    Small(Token, usize, usize),
    #[error("Parsing error: `{0}`.")]
    Parsing(String),
    #[error("Something happend when trying to read: {0}.")]
    Readline(String),
    #[error("Unknown op: `{0}`.")]
    Unknown(String),
    #[error("Self reference: `{0}` refers to itself.")]
    SelfRef(String),
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
    let m = Machine::default();
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
                match m.read_eval_print(&l) {
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
        println!("{m}");
    }
    rl.save_history("history.txt")
        .map_err(|e| Error::Readline(e.to_string()))?;
    Ok(())
}
