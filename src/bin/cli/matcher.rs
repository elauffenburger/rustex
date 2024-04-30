use std::io::{self, BufRead};

use rustex::{executor, parser::ParseResult};

use crate::{error::Error, printer, FileInput};

pub(crate) struct Matcher<W: io::Write> {
    pub printer: printer::OutputPrinter<W>,
}

impl<W: io::Write> Matcher<W> {
    pub(crate) fn run(&mut self, files: &mut [FileInput], expressions: &[ParseResult]) -> Result<(), Error> {
        let num_files = files.len();
        let searching_multiple_files = num_files > 1;
        let should_print_file_info = searching_multiple_files;

        for (file_num, file_spec) in files.iter_mut().enumerate() {
            let (file_handle, file_name, is_stdin): (Box<dyn io::Read>, String, bool) = match file_spec {
                FileInput::File(filename, file_handle) => (Box::new(file_handle), filename.to_string(), false),
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

                for expr in expressions {
                    let exec_res = executor.exec(expr, &line)?;
                    if exec_res.is_none() {
                        continue;
                    }

                    let res = exec_res.unwrap();

                    // Write filename info if applicable and if we haven't already done it.
                    if should_print_file_info && !has_printed_file_info {
                        self.printer.print_file_start(&file_name.bytes().collect::<Vec<_>>())?;
                        has_printed_file_info = true;
                    }

                    // Write line number info if applicable.
                    if searching_multiple_files || !is_stdin {
                        self.printer.print_line_num(line_num)?;
                    }

                    // Write result info.
                    self.printer.print_match(&res, &line_bytes)?;
                }
            }

            // If we started printing file info and this isn't the last file, print a newline to finish off the file results block.
            if has_printed_file_info && !file_num == num_files - 1 {
                self.printer.print_file_end()?;
            }
        }

        Ok(())
    }
}
