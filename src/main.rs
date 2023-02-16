mod cli;
mod jira;
mod util;

use cli::{csv, estimate, StatusTracker};

fn main() {
    let args = cli::parse();
    match args {
        StatusTracker::CSV(csv_args) => csv::run(&csv_args),
        StatusTracker::Estimate(estimate_args) => estimate::run(&estimate_args),
    };
}
