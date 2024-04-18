use std::io::{self, Write};

use crate::{executor, parser};

#[derive(Debug)]
pub enum Error {
    IOError { msg: String },
    ParseError { msg: String },
    ExecError { msg: String },
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Self::IOError {
            msg: format!("io error: {:?}", err),
        }
    }
}

impl From<&io::Error> for Error {
    fn from(err: &io::Error) -> Self {
        Self::IOError {
            msg: format!("io error: {:?}", err),
        }
    }
}

impl<'a> From<parser::ParseErrorWithContext<'a>> for Error {
    fn from(err: parser::ParseErrorWithContext<'a>) -> Self {
        Self::ParseError {
            msg: format!("parse error: {:?}", err),
        }
    }
}

impl From<executor::ExecError> for Error {
    fn from(err: executor::ExecError) -> Self {
        Self::ExecError {
            msg: format!("parse error: {:?}", err),
        }
    }
}

impl From<Error> for u32 {
    fn from(err: Error) -> Self {
        let _ = io::stderr().write_fmt(format_args!("error: {:?}", err));
        1 as u32
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IOError { msg } => f.write_str(msg),
            Self::ParseError { msg } => f.write_str(msg),
            Self::ExecError { msg } => f.write_str(msg),
        }
    }
}
