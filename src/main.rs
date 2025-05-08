//! Dylan's Rusty Stack Machine.
#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![deny(missing_docs)]
#![deny(unsafe_code)]
use drsm::{Error, Machine};
use rustyline::{Config, DefaultEditor, EditMode, error::ReadlineError};

/// Errors that could occur while using the REPL.
#[derive(Debug, thiserror::Error)]
pub enum ReplError {
    /// An error that arose during execution.
    #[error("{0}")]
    Execution(#[from] Error),
    /// An error that occured in our use of `rustyline`.
    #[error("Something happened with readline: `{0}`.")]
    Readline(#[from] ReadlineError),
}

fn main() -> Result<(), ReplError> {
    let mut r = DefaultEditor::with_config(Config::builder().edit_mode(EditMode::Vi).build())?;
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
    if r.load_history("history.txt").is_err() {
        println!("No previous history.");
    }
    loop {
        match r.readline(">  ") {
            Ok(l) => {
                r.add_history_entry(&l)?;
                match m.read_eval(&l) {
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
            Err(e) => {
                eprintln!("Error: {e}");
            }
        }
        println!("{m}");
    }
    r.save_history("history.txt")?;
    Ok(())
}
