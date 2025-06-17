//! Dylan's Rusty Stack Machine.
#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![deny(missing_docs)]
#![deny(unsafe_code)]
use clap::{Parser, Subcommand, ValueEnum};
use documented::DocumentedFields;
use drsm::{Core, Machine};
use rustyline::{Config, DefaultEditor, EditMode, error::ReadlineError};
use std::{
    fmt,
    fs::File,
    io::{self, BufRead, BufReader},
    path::PathBuf,
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(about = "run an interactive REPL")]
    Repl {
        #[arg(short, long, default_value_t = Mode::Vi)]
        mode: Mode,
    },
    #[command(about = "execute the commands in a file")]
    Run { file: PathBuf },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Mode {
    Vi,
    Emacs,
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Vi => f.write_str("vi"),
            Self::Emacs => f.write_str("emacs"),
        }
    }
}

impl From<Mode> for EditMode {
    fn from(m: Mode) -> Self {
        match m {
            Mode::Vi => Self::Vi,
            Mode::Emacs => Self::Emacs,
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("{0}")]
    Execution(#[from] drsm::Error),
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    Readline(#[from] ReadlineError),
}

static REPL_COMMANDS: &str = "
Commands:
    ?           =>  show these commands.
    ?show       =>  show machine's environment & stack.
    ?lookup <w> =>  look up word <w> in the environment.
    ?quit       =>  quit the REPL.
";

fn main() -> Result<(), Error> {
    let args = Args::parse();
    match args.command {
        Command::Repl { mode } => {
            let mut r =
                DefaultEditor::with_config(Config::builder().edit_mode(mode.into()).build())?;
            println!(
                r"
    ____  ____  _____ __  ___
   / __ \/ __ \/ ___//  |/  /
  / / / / /_/ /\__ \/ /|_/ /
 / /_/ / _, _/___/ / /  / /
/_____/_/ |_|/____/_/  /_/

Dylan's Rusty Stack Machine

{REPL_COMMANDS}

Line-editing is enabled, with {mode}-style key bindings (chosen at startup via the `-m/--mode` option).
"
            );
            if r.load_history("history.txt").is_err() {
                eprintln!("No previous history.");
            }
            let mut m = Machine::default();
            loop {
                match r.readline(">  ") {
                    Ok(l) if l == "?" => println!("{REPL_COMMANDS}"),
                    Ok(l) if l == "?show" => println!("{m}"),
                    Ok(l) if l == "?quit" => {
                        println!("Bye!");
                        break;
                    }
                    Ok(l) if l.starts_with("?lookup ") => {
                        if let Some(w) = l.split_ascii_whitespace().nth(1) {
                            match (Core::get_field_docs(w), m.lookup(w)) {
                                (Ok(d), _) => println!("`{w}` is a core word: {d}"),
                                (_, Some(d)) => println!("`{w}` is defind as {d}"),
                                (_, None) => eprintln!("`{w}` is not defined in the environment."),
                            }
                        } else {
                            eprintln!("?lookup requires a word to look up.");
                        }
                    }
                    Ok(l) => {
                        r.add_history_entry(&l)?;
                        match m.read_eval(&l) {
                            Ok(()) => {}
                            Err(e) => eprintln!("{e}"),
                        }
                    }
                    Err(ReadlineError::Eof) => {
                        println!("^D");
                        break;
                    }
                    Err(ReadlineError::Interrupted) => println!("^C"),
                    Err(e) => eprintln!("Error: {e}"),
                }
            }
            r.save_history("history.txt")?;
        }
        Command::Run { file } => {
            let mut m = Machine::default();
            for line in BufReader::new(File::open(file)?).lines() {
                m.read_eval(&line?)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_roundtrip() {
        for m in [Mode::Vi, Mode::Emacs] {
            assert_eq!(m, ValueEnum::from_str(&m.to_string(), false).unwrap());
        }
    }
}
