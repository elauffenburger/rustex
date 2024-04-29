use clap::{CommandFactory, Parser};
use termcolor::{self};

use std::{fs, io};

use rustex::{
    executor::{self},
    parser,
};

mod error;
use error::Error;

mod printer;
use printer::OutputPrinter;

mod matcher;
use matcher::Matcher;

#[derive(clap::Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The regex pattern to match.
    #[arg(index = 1, required = false)]
    pattern: Option<String>,

    /// The files to match against (or - for STDIN).
    #[arg(index = 2, required = false, name = "PATH")]
    filenames: Vec<String>,

    /// Patterns to search for.
    #[arg(short = 'e', long, conflicts_with = "pattern")]
    expressions: Vec<String>,
}

pub fn main() -> Result<(), u32> {
    tracing_subscriber::fmt::init();

    Ok(maine()?)
}

fn maine() -> Result<(), Error> {
    let args = Args::parse();

    let (filenames, read_stdin) = {
        let mut filenames = vec![];
        let mut read_stdin = false;

        for filename in &args.filenames {
            match filename.as_str() {
                "-" => {
                    if !read_stdin {
                        read_stdin = true
                    } else {
                        Args::command()
                            .error(
                                clap::error::ErrorKind::ArgumentConflict,
                                "cannot supply '-' as filename more than once",
                            )
                            .exit()
                    }
                }
                _ => filenames.push(filename),
            }
        }

        if filenames.is_empty() {
            read_stdin = true;
        }

        (filenames, read_stdin)
    };

    let expressions = {
        let parser = parser::Parser::new();

        args.pattern
            .map_or_else(|| args.expressions, |pattern| vec![pattern])
            .iter()
            .map(|expr| parser.parse_str(expr))
            .collect::<Result<Vec<_>, _>>()?
    };

    let mut files = {
        let mut files: Vec<FileInput> = vec![];

        // Add literal files.
        for filename in filenames {
            // The filename might actually be a dir, so we need to transform a single path into multiple file specs.
            for spec in get_filespecs_from_path(filename)? {
                files.push(spec);
            }
        }

        // Add stdin if requested.
        if read_stdin {
            files.push(FileInput::Stdin(io::stdin()));
        }

        files
    };

    let mut matcher = Matcher {
        printer: OutputPrinter::new(termcolor::Ansi::new(termcolor::StandardStream::stdout(
            termcolor::ColorChoice::AlwaysAnsi,
        ))),
    };

    matcher.run(&mut files, &expressions)
}

enum FileInput {
    File(String, fs::File),
    Stdin(io::Stdin),
}

fn get_filespecs_from_path(filename: &str) -> Result<Vec<FileInput>, Error> {
    fn rec(filename: &str, files: &mut Vec<FileInput>) -> Result<(), Error> {
        let metadata = fs::metadata(filename)?;

        if metadata.is_dir() {
            let dir = fs::read_dir(filename)?;
            for entry in dir {
                let entry = entry?;
                let path = entry.path();

                rec(path.to_str().unwrap(), files)?;
            }
        } else {
            let file = fs::File::open(filename)?;
            files.push(FileInput::File(filename.into(), file));
        }

        Ok(())
    }

    let mut files = vec![];
    rec(filename, &mut files)?;

    Ok(files)
}
