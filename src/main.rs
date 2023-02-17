mod cli;
mod jira;
mod util;

use std::process;

use cli::{csv, estimate, tag, StatusTracker};

fn main() {
    let args = cli::parse();
    if args.is_err() {
        println!("{}", args.unwrap_err());
        process::exit(1)
    }

    match args.unwrap() {
        StatusTracker::CSV(csv_args) => csv::run(&csv_args),
        StatusTracker::Estimate(estimate_args) => estimate::run(&estimate_args),
        StatusTracker::Tag(tag_args) => tag::run(&tag_args),
    };
}
