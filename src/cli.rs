use std::{fmt, io};

use argfile;
use clap::Parser;

pub mod csv;
pub mod estimate;
pub mod tag;

#[derive(Debug, Parser)]
#[command(name = "statustracker")]
#[command(author = "Jim Berlage <jamesberlage@gmail.com>")]
#[command(version = "1.0.0")]
#[command(about = "A suite of utilities to estimate time left to complete a project.  Based on team velocity and estimated story points.", long_about = None)]
pub enum StatusTracker {
    CSV(csv::CSV),
    Estimate(estimate::Estimate),
    Tag(tag::Tag),
}

#[derive(Debug)]
pub enum ParseError {
    ProblemUnwrappingArgfileError(io::Error),
    CLIParseError(clap::Error),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseError::ProblemUnwrappingArgfileError(_) => {
                write!(f, "There was a problem reading in the argfile (a file with command line arguments that starts with '@'.)  I would check to ensure that you have the right path to the argfile.  You can run the command with --debug to see the full error.")
            }
            ParseError::CLIParseError(inner) => {
                write!(f, "{}", inner)
            }
        }
    }
}

pub fn parse() -> Result<StatusTracker, ParseError> {
    let args = argfile::expand_args(argfile::parse_fromfile, argfile::PREFIX)
        .map_err(|e| ParseError::ProblemUnwrappingArgfileError(e))?;

    StatusTracker::try_parse_from(args).map_err(|e| ParseError::CLIParseError(e))
}
