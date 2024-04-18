use clap::{CommandFactory, Parser};
use termcolor::{self};

use std::{
    fs,
    io::{self, BufRead},
};

use rustex::{
    executor::{self},
    parser,
};

mod error;
use error::Error;

mod printer;
use printer::OutputPrinter;

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

enum FileInput {
    File(String, fs::File),
    Stdin(io::Stdin),
}

pub fn main() -> Result<(), u32> {
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

    let files = {
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

    let mut printer = OutputPrinter::new(termcolor::Ansi::new(termcolor::StandardStream::stdout(
        termcolor::ColorChoice::AlwaysAnsi,
    )));

    let num_files = (&files).len();
    let searching_multiple_files = num_files > 1;
    let should_print_file_info = searching_multiple_files;

    for (file_num, file_spec) in files.into_iter().enumerate() {
        let (file_handle, file_name, is_stdin): (Box<dyn io::Read>, String, bool) = match file_spec {
            FileInput::File(filename, file_handle) => (Box::new(file_handle), filename.into(), false),
            FileInput::Stdin(stdin) => (Box::new(stdin), "stdin".into(), true),
        };

        let mut executor = executor::Executor::new();
        let mut reader = io::BufReader::new(file_handle);

        let mut line_num = 0;
        let mut has_printed_file_info = false;
        loop {
            let mut line = String::new();

            match reader.read_line(&mut line) {
                Err(err) => return Err(Error::from(err)),
                Ok(0) => break,
                _ => {}
            };

            line_num += 1;
            let line_bytes = line.bytes().collect::<Vec<_>>();

            for expr in &expressions {
                let exec_res = executor.exec(expr, &line)?;
                if exec_res.is_none() {
                    continue;
                }

                let res = exec_res.unwrap();

                // Write filename info if applicable and if we haven't already done it.
                if should_print_file_info && !has_printed_file_info {
                    printer.print_file_start(&file_name.bytes().collect::<Vec<_>>())?;
                    has_printed_file_info = true;
                }

                // Write line number info if applicable.
                if searching_multiple_files || !is_stdin {
                    printer.print_line_num(line_num)?;
                }

                // Write result info.
                printer.print_match(&res, &line_bytes)?;
            }
        }

        // If we started printing file info and this isn't the last file, print a newline to finish off the file results block.
        if has_printed_file_info && !file_num == num_files - 1 {
            printer.print_file_end()?;
        }
    }

    Ok(())
}

fn get_filespecs_from_path(filename: &str) -> Result<Vec<FileInput>, Error> {
    let mut files = vec![];
    get_filespecs_from_path_rec(filename, &mut files)?;

    Ok(files)
}

fn get_filespecs_from_path_rec(filename: &str, files: &mut Vec<FileInput>) -> Result<(), Error> {
    let metadata = fs::metadata(filename)?;

    if metadata.is_file() {
        let file = fs::File::open(filename)?;
        files.push(FileInput::File(filename.into(), file));
    } else if metadata.is_dir() {
        let dir = fs::read_dir(filename)?;
        for entry in dir {
            let entry = entry?;
            let path = entry.path();

            get_filespecs_from_path_rec(path.to_str().unwrap(), files)?;
        }
    } else {
        unimplemented!("file type not supported: {:?}", metadata)
    }

    Ok(())
}
