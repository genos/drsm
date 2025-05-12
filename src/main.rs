//! Dylan's Rusty Stack Machine.
#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![deny(missing_docs)]
#![deny(unsafe_code)]
use clap::{Parser, Subcommand, ValueEnum};
use drsm::Machine;
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
        edit_mode: Mode,
    },
    #[command(about = "execute the commands in a file")]
    Run { file: PathBuf },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
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

fn main() -> Result<(), Error> {
    let args = Args::parse();
    match args.command {
        Command::Repl { edit_mode } => {
            let mut r =
                DefaultEditor::with_config(Config::builder().edit_mode(edit_mode.into()).build())?;
            println!(
                r"
    ____  ____  _____ __  ___
   / __ \/ __ \/ ___//  |/  /
  / / / / /_/ /\__ \/ /|_/ /
 / /_/ / _, _/___/ / /  / /
/_____/_/ |_|/____/_/  /_/
"
            );
            if r.load_history("history.txt").is_err() {
                eprintln!("No previous history.");
            }
            let mut m = Machine::default();
            loop {
                match r.readline(">  ") {
                    Ok(l) => {
                        r.add_history_entry(&l)?;
                        match m.read_eval(&l) {
                            Ok(()) => {}
                            Err(e) => eprintln!("{e}"),
                        }
                    }
                    Err(ReadlineError::Eof) => {
                        println!("Bye!");
                        break;
                    }
                    Err(ReadlineError::Interrupted) => println!("^C"),
                    Err(e) => eprintln!("Error: {e}"),
                }
                println!("{m}");
            }
            r.save_history("history.txt")?;
        }
        Command::Run { file } => {
            let f = File::open(file)?;
            let mut m = Machine::default();
            for line in BufReader::new(f).lines() {
                m.read_eval(&line?)?;
            }
            println!("{m}");
        }
    }
    Ok(())
}
