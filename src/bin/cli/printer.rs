use lazy_static::lazy_static;

use termcolor::{self, WriteColor};

use std::io::Write;

use rustex::executor::ExecResult;

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

pub struct OutputPrinter<W: std::io::Write> {
    pr: termcolor::Ansi<W>,
}

impl<W: std::io::Write> OutputPrinter<W> {
    pub fn new(printer: termcolor::Ansi<W>) -> Self {
        Self { pr: printer }
    }

    pub fn print_file_start(&mut self, file_name_bytes: &[u8]) -> std::io::Result<usize> {
        self.pr
            .set_color(&FILENAME_COLOR_SPEC)
            .and_then(|_| self.pr.write(file_name_bytes))
            .and_then(|_| self.pr.reset())
            .and_then(|_| self.pr.write(&[b'\n']))
    }

    pub fn print_file_end(&mut self) -> std::io::Result<usize> {
        self.pr.write(&[b'\n'])
    }

    pub fn print_line_num(&mut self, line_num: usize) -> std::io::Result<usize> {
        self.pr
            .set_color(&LINE_NUMBER_COLOR_SPEC)
            .and_then(|_| self.pr.write_fmt(format_args!("{:?}", line_num)))
            .and_then(|_| self.pr.reset())
            .and_then(|_| self.pr.write(&[b':']))
    }

    pub fn print_match(&mut self, res: &ExecResult, line_bytes: &[u8]) -> std::io::Result<usize> {
        self.pr
            .write(&line_bytes[0..res.start])
            .and_then(|_| self.pr.set_color(&MATCH_COLOR_SPEC))
            .and_then(|_| self.pr.write(&line_bytes[res.start..res.end + 1]))
            .and_then(|_| self.pr.reset())
            .and_then(|_| self.pr.write(&line_bytes[res.end + 1..]))
    }

    pub fn print_replacement(&mut self, replacement: &[u8]) -> std::io::Result<usize> {
        self.pr
            .set_color(&MATCH_COLOR_SPEC)
            .and_then(|_| self.pr.write(replacement))
            .and_then(|_| self.pr.reset())
            .and_then(|_| self.pr.write(&[b'\n']))
    }
}
