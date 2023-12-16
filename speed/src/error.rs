use std::fmt::{Display, Formatter};

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
pub(crate) enum Error {
    Unknown,
    AlreadyDeclared,
    NotDeclared,
    AlreadyBeating,
    NotACamera,
    InvalidStream,
    // InvalidData also known as ParseError.
    InvalidData,
}
impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Unknown => "Unknown",
                Self::AlreadyDeclared => "AlreadyDeclared",
                Self::NotDeclared => "NotDeclared",
                Self::AlreadyBeating => "AlreadyBeating",
                Self::NotACamera => "NotACamera",
                Self::InvalidStream => "InvalidStream",
                Self::InvalidData => "InvalidData",
            }
        )
    }
}
