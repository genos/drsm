use drsm::{Error, Machine};
use rustyline::{Config, EditMode, Editor, error::ReadlineError};

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
