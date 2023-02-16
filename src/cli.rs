use argfile;
use clap::Parser;

pub mod csv;
pub mod estimate;

#[derive(Debug, Parser)]
#[command(name = "statustracker")]
#[command(author = "Jim Berlage <jamesberlage@gmail.com>")]
#[command(version = "1.0.0")]
#[command(about = "A suite of utilities to estimate time left to complete a project.  Based on team velocity and estimated story points.", long_about = None)]
pub enum StatusTracker {
    CSV(csv::CSV),
    Estimate(estimate::Estimate),
}

pub fn parse() -> StatusTracker {
    let args = argfile::expand_args(argfile::parse_fromfile, argfile::PREFIX).unwrap();

    StatusTracker::parse_from(args)
}
