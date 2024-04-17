use clap::{CommandFactory, Parser};

use std::{
    fs,
    io::{self, stderr, BufRead, BufReader, Write},
};

use rustex::{executor, parser};

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
            .collect::<Result<Vec<_>, _>>()
            .map_err(|err| {
                let _ = stderr().write_fmt(format_args!("error parsing expression: {:?}", err));
                1 as u32
            })?
    };

    let files = {
        let mut files: Vec<Box<dyn io::Read>> = vec![];

        // Add literal files.
        for filename in filenames {
            let file = fs::File::open(filename).map_err(|err| {
                let _ = io::stderr().write_fmt(format_args!("error reading file: {:?}", err));
                1 as u32
            })?;

            files.push(Box::new(file));
        }

        // Add stdin if requested.
        if read_stdin {
            files.push(Box::new(io::stdin()));
        }

        files
    };

    let mut executor = executor::Executor::new();

    for file in files {
        let mut reader = BufReader::new(file);

        let mut buf = String::new();
        loop {
            match &reader.read_line(&mut buf) {
                Err(err) => {
                    let _ = stderr().write_fmt(format_args!("error reading line: {}", err));
                    return Err(1);
                }
                Ok(0) => break,
                Ok(_) => {
                    for expr in &expressions {
                        let exec_res = executor.exec(expr.clone(), &buf).map_err(|err| {
                            let _ = stderr()
                                .write_fmt(format_args!("error executing expression: {:?}", err));
                            return 1 as u32;
                        })?;

                        println!("{:?}", &exec_res);
                    }
                }
            }
        }
    }

    Ok(())
}
