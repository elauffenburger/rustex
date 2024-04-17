use lazy_static::lazy_static;

use clap::{CommandFactory, Parser};
use termcolor::{self, WriteColor};

use std::{
    ffi, fs,
    io::{self, stderr, BufRead, Write},
};

use rustex::{executor, parser};

lazy_static! {
    static ref FILENAME_COLOR_SPEC: termcolor::ColorSpec = {
        let mut spec = termcolor::ColorSpec::new();

        spec.set_fg(Some(termcolor::Color::Magenta)).to_owned()
    };
    static ref LINE_NUMBER_COLOR_SPEC: termcolor::ColorSpec = {
        let mut spec = termcolor::ColorSpec::new();

        spec.set_fg(Some(termcolor::Color::Green)).to_owned()
    };
    static ref MATCH_COLOR_SPEC: termcolor::ColorSpec = {
        let mut spec = termcolor::ColorSpec::new();

        spec.set_bold(true).set_fg(Some(termcolor::Color::Red)).to_owned()
    };
}

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
                let _ = io::stderr().write_fmt(format_args!("error parsing expression: {:?}", err));
                1 as u32
            })?
    };

    let files = {
        let mut files: Vec<FileInput> = vec![];

        // Add literal files.
        for filename in filenames {
            // The filename might actually be a dir, so we need to transform a single path into multiple file specs.
            let specs = get_filespecs_from_path(filename).map_err(|err| {
                let _ = io::stderr().write_fmt(format_args!("error getting file specs: {:?}", err));
                1 as u32
            })?;

            for spec in specs {
                files.push(spec);
            }
        }

        // Add stdin if requested.
        if read_stdin {
            files.push(FileInput::Stdin(io::stdin()));
        }

        files
    };

    let mut printer = termcolor::Ansi::new(termcolor::StandardStream::stdout(termcolor::ColorChoice::AlwaysAnsi));

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
            match &reader.read_line(&mut line) {
                Err(err) => {
                    let _ = io::stderr().write_fmt(format_args!("error reading line: {}", err));
                    return Err(1);
                }
                Ok(0) => break,
                Ok(_) => {
                    line_num += 1;
                    let line_bytes = line.bytes().collect::<Vec<_>>();

                    for expr in &expressions {
                        let exec_res = executor.exec(expr.clone(), &line).map_err(|err| {
                            let _ = io::stderr().write_fmt(format_args!("error executing expression: {:?}", err));
                            return 1 as u32;
                        })?;

                        match exec_res {
                            None => {}
                            Some(res) => {
                                // Write filename info if applicable and if we haven't already done it.
                                if should_print_file_info && !has_printed_file_info {
                                    printer
                                        .set_color(&FILENAME_COLOR_SPEC)
                                        .and_then(|_| printer.write(&file_name.bytes().collect::<Vec<_>>()))
                                        .and_then(|_| printer.reset())
                                        .and_then(|_| printer.write(&[b'\n']))
                                        .map_err(|err| {
                                            let _ = io::stderr()
                                                .write_fmt(format_args!("error writing filename: {:?}", err));
                                            1 as u32
                                        })?;

                                    has_printed_file_info = true;
                                }

                                // Write line number info if applicable.
                                if searching_multiple_files || !is_stdin {
                                    printer
                                        .set_color(&LINE_NUMBER_COLOR_SPEC)
                                        .and_then(|_| printer.write_fmt(format_args!("{:?}", line_num)))
                                        .and_then(|_| printer.reset())
                                        .and_then(|_| printer.write(&[b':']))
                                        .map_err(|err| {
                                            let _ =
                                                stderr().write_fmt(format_args!("error writing line info: {:?}", err));
                                            1 as u32
                                        })?;
                                }

                                // Write result info.
                                printer
                                    .write(&line_bytes[0..res.start])
                                    .and_then(|_| printer.set_color(&MATCH_COLOR_SPEC))
                                    .and_then(|_| printer.write(&line_bytes[res.start..res.end + 1]))
                                    .and_then(|_| printer.reset())
                                    .and_then(|_| printer.write(&line_bytes[res.end + 1..]))
                                    .map_err(|err| {
                                        let _ = io::stderr()
                                            .write_fmt(format_args!("error writing results to stdout: {:?}", err));

                                        1 as u32
                                    })?;
                            }
                        }
                    }
                }
            }
        }

        // If we started printing file info and this isn't the last file, print a newline to finish off the file results block.
        if has_printed_file_info && !file_num == num_files - 1 {
            printer.write(&[b'\n']).map_err(|err| {
                let _ = io::stderr().write_fmt(format_args!("error writing end of line to stdout: {:?}", err));
                1 as u32
            })?;
        }
    }

    Ok(())
}

#[derive(Debug)]
enum GetFileSpecsError {
    IOError(io::Error),
}

impl From<io::Error> for GetFileSpecsError {
    fn from(err: io::Error) -> Self {
        GetFileSpecsError::IOError(err)
    }
}

fn get_filespecs_from_path(filename: &str) -> Result<Vec<FileInput>, GetFileSpecsError> {
    let mut files = vec![];
    get_filespecs_from_path_rec(filename, &mut files)?;

    Ok(files)
}

fn get_filespecs_from_path_rec(filename: &str, files: &mut Vec<FileInput>) -> Result<(), GetFileSpecsError> {
    let metadata = fs::metadata(filename)?;

    if metadata.is_file() {
        let file = fs::File::open(filename)?;
        files.push(FileInput::File(filename.into(), file));
    } else if metadata.is_dir() {
        let dir = fs::read_dir(filename)?;
        for entry in dir {
            let entry = entry?;
            let path = entry.path();

            get_filespecs_from_path_rec(path.to_str().expect("expected file path"), files)?;
        }
    } else {
        unimplemented!("file type not supported: {:?}", metadata)
    }

    Ok(())
}
