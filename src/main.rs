// [[file:~/Workspace/Programming/gosh/gosh.note::c5024615-a25b-4b40-9305-890be0fe004b][c5024615-a25b-4b40-9305-890be0fe004b]]
// `error_chain!` can recurse deeply
#![recursion_limit = "1024"]

extern crate linefeed;
extern crate gchemol;
#[macro_use]
extern crate error_chain;

use std::rc::Rc;
use std::io::Write;
use linefeed::{Reader, ReadResult};
use linefeed::terminal::Terminal;
use linefeed::complete::{Completer, Completion};
use error_chain::ChainedError; // trait which holds `display_chain`

use gchemol::{
    Molecule,
};

use cli::Commander;
mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain!{}
}
use errors::*;

mod cli;

use std::process::Command;
use std::path::{Path, PathBuf};
use std::env;

fn get_history_file() -> Result<PathBuf> {
    match env::home_dir() {
        Some(path) => {
            let filename = path.join(".gosh.history");
            return Ok(filename);
        },
        None => bail!("Impossible to get your home dir!"),
    }
}


fn main() {
    let mut reader = Reader::new("rusty gosh").unwrap();

    let version = env!("CARGO_PKG_VERSION");
    println!("This is the rusty gosh shell version {}.", version);
    println!("Enter \"help\" or \"?\" for a list of commands.");
    println!("Press Ctrl-D or enter \"quit\" or \"q\" to exit.");
    println!("");

    reader.set_completer(Rc::new(GOSHCompleter));
    reader.set_prompt("gosh> ");

    let mut commander = Commander::new();
    let history_file = get_history_file().unwrap();
    if let Err(e) = reader.load_history(&history_file) {
        if e.kind() == std::io::ErrorKind::NotFound {
            println!("History file {} doesn't exist, not loading history.", history_file.display());
        } else {
            eprintln!("Could not load history file {}: {}", history_file.display(), e);
        }
    }

    while let Ok(ReadResult::Input(line)) = reader.read_line() {
        if !line.trim().is_empty() {
            reader.add_history(line.clone());
        }

        let (cmd, args) = split_first_word(&line);

        match cmd {
            "help" | "?" => {
                println!("linefeed demo commands:");
                println!();
                for &(cmd, help) in GOSH_COMMANDS {
                    println!("  {:16} - {}", cmd, help);
                }
                println!();
            },
            "load" => {
                if args.len() == 0 {
                    println!("Please input path to a file containing molecule.");
                } else {
                    let filename = args;
                    if let Err(ref e) = &mut commander.load(filename) {
                        let stderr = &mut ::std::io::stderr();
                        let errmsg = "Error writing to stderr";

                        writeln!(stderr, "{}", e.display_chain()).expect(errmsg);
                    } else {
                        println!("{} molecules loaded from: {:?}.", commander.molecules.len(), filename);
                    }
                }
            },

            "rebond" => {
                if let Err(ref e) = &mut commander.rebond() {
                    let stderr = &mut ::std::io::stderr();
                    let errmsg = "Error writing to stderr";

                    writeln!(stderr, "{}", e.display_chain()).expect(errmsg);
                }
            },

            "clean" => {
                if let Err(ref e) = &mut commander.clean() {
                    let stderr = &mut ::std::io::stderr();
                    let errmsg = "Error writing to stderr";

                    writeln!(stderr, "{}", e.display_chain()).expect(errmsg);
                }
            },

            "write" => {
                if args.len() == 0 {
                    println!("Please input path to save the molecule.");
                } else {
                    let filename = args;
                    if let Err(ref e) = &commander.write(filename) {
                        let stderr = &mut ::std::io::stderr();
                        let errmsg = "Error writing to stderr";

                        writeln!(stderr, "{}", e.display_chain()).expect(errmsg);
                    }
                }
            },

            "save" => {
                if let Err(ref e) = &commander.save() {
                    let stderr = &mut ::std::io::stderr();
                    let errmsg = "Error writing to stderr";

                    writeln!(stderr, "{}", e.display_chain()).expect(errmsg);
                } else {
                    println!("saved.");
                }
            },

            "ls" => {
                if let Err(ref e) = &commander.extern_cmdline("ls") {
                    let stderr = &mut ::std::io::stderr();
                    let errmsg = "Error writing to stderr";

                    writeln!(stderr, "{}", e.display_chain()).expect(errmsg);
                }
            },

            "pwd" => {
                if let Err(ref e) = &commander.extern_cmdline("pwd") {
                    let stderr = &mut ::std::io::stderr();
                    let errmsg = "Error writing to stderr";

                    writeln!(stderr, "{}", e.display_chain()).expect(errmsg);
                }
            },

            "quit" | "q" => {
                if let Err(e) = reader.save_history(&history_file) {
                    eprintln!("Could not save history file {}: {}", history_file.display(), e);
                }
                break;
            },

            "" => (),
            _ => println!("{:?}: not a command", line),
        }
    }
}

static GOSH_COMMANDS: &'static [(&'static str, &'static str)] = &[
    ("help",             "You're looking at it"),
    ("quit",             "Quit the demo"),
    ("load",             "Load molecule from disk"),
    ("write",            "Write molecules into file"),
    ("rebond",           "Rebuild bonds from atom distances."),
    ("clean",            "Clean up bad molecular geometry."),
];

fn split_first_word(s: &str) -> (&str, &str) {
    let s = s.trim();

    match s.find(|ch: char| ch.is_whitespace()) {
        Some(pos) => (&s[..pos], s[pos..].trim_left()),
        None => (s, "")
    }
}

struct GOSHCompleter;

impl<Term: Terminal> Completer<Term> for GOSHCompleter {
    fn complete(&self, word: &str, reader: &Reader<Term>,
            start: usize, _end: usize) -> Option<Vec<Completion>> {
        let line = reader.buffer();

        let mut words = line[..start].split_whitespace();

        match words.next() {
            // Complete command name
            None => {
                let mut compls = Vec::new();

                for &(cmd, _) in GOSH_COMMANDS {
                    if cmd.starts_with(word) {
                        compls.push(Completion::simple(cmd.to_owned()));
                    }
                }

                Some(compls)
            }
            // Complete command parameters
            Some("load") | Some("write") => {
                if words.count() == 0 {
                    let mut res = Vec::new();

                    for (name, _) in reader.variables() {
                        if name.starts_with(word) {
                            res.push(Completion::simple(name.to_owned()));
                        }
                    }

                    Some(res)
                } else {
                    None
                }
            }
            _ => None
        }
    }
}
// c5024615-a25b-4b40-9305-890be0fe004b ends here
