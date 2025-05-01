use rustyline::{error::ReadlineError, Config, EditMode, Editor};
use std::{fmt, str::FromStr};

#[derive(Debug, Clone, Copy)]
enum Value {
    Num(i64),
    Op(Op),
}

impl Default for Value {
    fn default() -> Self {
        Self::Num(0)
    }
}

impl FromStr for Value {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse()
            .map(Self::Num)
            .or_else(|_| Op::from_str(s).map(Self::Op))
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Num(n) => write!(f, "{n}"),
            Self::Op(op) => write!(f, "{op}"),
        }
    }
}

impl Value {
    fn to_num(self) -> Result<i64, Error> {
        match self {
            Self::Num(n) => Ok(n),
            Self::Op(op) => Err(Error::NaN(op)),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
enum Op {
    #[default]
    Pop,
    Swap,
    Dup,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

impl FromStr for Op {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pop" => Ok(Self::Pop),
            "swap" => Ok(Self::Swap),
            "dup" => Ok(Self::Dup),
            "add" => Ok(Self::Add),
            "sub" => Ok(Self::Sub),
            "mul" => Ok(Self::Mul),
            "div" => Ok(Self::Div),
            "mod" => Ok(Self::Mod),
            _ => Err(Error::Token(s.to_string())),
        }
    }
}

impl fmt::Display for Op {
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
        }
    }
}

#[derive(Debug)]
struct Stack(Vec<Value>);

impl fmt::Display for Stack {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("[")?;
        for v in self.0.iter().rev() {
            write!(f, " {v}")?;
        }
        f.write_str(" ]")
    }
}

impl Stack {
    fn new() -> Self {
        Self(Vec::new())
    }
    fn pop(&mut self, op: Op, required: usize, stack_len: usize) -> Result<Value, Error> {
        self.0.pop().ok_or(Error::Small(op, required, stack_len))
    }
    fn eval(&mut self, vs: &[Value]) -> Result<(), Error> {
        for &v in vs {
            match v {
                Value::Num(_) => self.0.push(v),
                Value::Op(op) => match op {
                    Op::Pop => self.pop(op, 1, 0).map(|_| ())?,
                    Op::Swap => {
                        let x = self.pop(op, 2, 0)?;
                        let y = self.pop(op, 2, 1)?;
                        self.0.push(x);
                        self.0.push(y);
                    }
                    Op::Dup => {
                        let x = self.pop(op, 1, 0)?;
                        self.0.push(x);
                        self.0.push(x);
                    }
                    Op::Add => {
                        let x = self.pop(op, 2, 0).and_then(Value::to_num)?;
                        let y = self.pop(op, 2, 1).and_then(Value::to_num)?;
                        self.0.push(Value::Num(x + y));
                    }
                    Op::Sub => {
                        let x = self.pop(op, 2, 0).and_then(Value::to_num)?;
                        let y = self.pop(op, 2, 1).and_then(Value::to_num)?;
                        self.0.push(Value::Num(x - y));
                    }
                    Op::Mul => {
                        let x = self.pop(op, 2, 0).and_then(Value::to_num)?;
                        let y = self.pop(op, 2, 1).and_then(Value::to_num)?;
                        self.0.push(Value::Num(x * y));
                    }
                    Op::Div => {
                        let x = self.pop(op, 2, 0).and_then(Value::to_num)?;
                        let y = self.pop(op, 2, 1).and_then(Value::to_num)?;
                        self.0.push(Value::Num(x / y));
                    }
                    Op::Mod => {
                        let x = self.pop(op, 2, 0).and_then(Value::to_num)?;
                        let y = self.pop(op, 2, 1).and_then(Value::to_num)?;
                        self.0.push(Value::Num(x % y));
                    }
                },
            }
        }
        Ok(())
    }
    fn read_eval_print(&mut self, s: &str) -> Result<(), Error> {
        let vs = s
            .split_ascii_whitespace()
            .map(Value::from_str)
            .collect::<Result<Vec<_>, _>>()?;
        self.eval(&vs)?;
        println!("{self}");
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("I expected a number, but I found `{0}`.")]
    NaN(Op),
    #[error("The stack is too small for `{0}`; it requires {1}, but the stack only has {2}.")]
    Small(Op, usize, usize),
    #[error("Unknown token: `{0}`.")]
    Token(String),
    #[error("Something happend when trying to read: {0}.")]
    Readline(#[from] ReadlineError),
}

fn main() -> Result<(), Error> {
    let config = Config::builder().edit_mode(EditMode::Vi).build();
    let mut rl: Editor<(), _> = Editor::with_config(config)?;
    let mut stack = Stack::new();
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
            Ok(line) => {
                rl.add_history_entry(line.as_str())?;
                match stack.read_eval_print(&line) {
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
    }
    rl.save_history("history.txt")?;
    Ok(())
}
