use std::process;

use clap::Args;
use colored::Colorize;
use jimberlage_jira_client::{self, jql::SerializableToJQL, RestClient, SearchIssue};

use crate::jira;

#[derive(Debug, Args)]
pub struct Estimate {
    #[arg(long)]
    #[arg(default_value_t = 3.0)]
    pub default_story_points: f64,

    #[arg(long)]
    #[arg(default_value = "Story Points")]
    pub jira_story_points_field: String,

    #[arg(long)]
    pub jira_label: Vec<String>,

    #[arg(long)]
    pub jira_project: Vec<String>,

    #[arg(long)]
    pub jira_issue_type: Vec<String>,

    #[arg(long)]
    pub jira_token: String,

    #[arg(long)]
    pub jira_url: String,

    #[arg(long)]
    pub jira_username: String,

    #[arg(long)]
    pub velocity_in_story_points: f64,

    #[arg(long)]
    #[arg(default_value_t = false)]
    pub verbose: bool,
}

enum ClassifiedIssue {
    Complete,
    IncompleteAndPointed(f64),
    IncompleteAndUnpointed,
}

fn classify(issue: &SearchIssue, field_ids: &Vec<String>) -> ClassifiedIssue {
    if let Some(status) = &issue.status_category() {
        if status == "Done" {
            return ClassifiedIssue::Complete;
        }
    }

    if let Some(points) = jira::story_points(issue, field_ids) {
        if points == 0.0 {
            return ClassifiedIssue::IncompleteAndUnpointed;
        }

        return ClassifiedIssue::IncompleteAndPointed(points);
    }

    ClassifiedIssue::IncompleteAndUnpointed
}

struct Results {
    default_story_points: f64,
    num_complete: f64,
    num_incomplete_and_pointed: f64,
    num_incomplete_and_unpointed: f64,
    num_sprints_remaining: f64,
    unfinished_estimated_story_points: f64,
    unfinished_story_points: f64,
    unfinished_unestimated_story_points: f64,
    velocity_in_story_points: f64,
}

impl Results {
    fn explain(&self) {
        println!(
            "There are {} cards completed.",
            format!("{:.0}", self.num_complete).yellow()
        );
        println!(
            "There are {} cards remaining that are estimated, representing {} points left to go.",
            format!("{:.0}", self.num_incomplete_and_pointed).bright_blue(),
            format!("{:.0}", self.unfinished_estimated_story_points).bright_magenta()
        );
        println!(
            "There are {} cards remaining that are unestimated.  Using a default story point value of {}, there are {} × {} = {} points left to go.",
            format!("{:.0}", self.num_incomplete_and_unpointed).bright_red(),
            format!("{:.0}", self.default_story_points).cyan(),
            format!("{:.0}", self.num_incomplete_and_unpointed).bright_red(),
            format!("{:.0}", self.default_story_points).cyan(),
            format!("{:.0}", self.unfinished_unestimated_story_points).green()
        );
        println!(
            "That means there are {} + {} = {} total points left to go.",
            format!("{:.0}", self.unfinished_estimated_story_points).bright_magenta(),
            format!("{:.0}", self.unfinished_unestimated_story_points).green(),
            format!("{:.0}", self.unfinished_story_points).bright_yellow()
        );
        println!(
            "Given a velocity of {} points / sprint, there is at least {} / {} = {} sprints remaining.",
            format!("{:.0}", self.velocity_in_story_points).magenta(),
            format!("{:.0}", self.unfinished_story_points).bright_yellow(),
            format!("{:.0}", self.velocity_in_story_points).magenta(),
            format!("{:.1}", self.num_sprints_remaining).bright_green()
        );
    }

    fn tally(
        issues: &Vec<SearchIssue>,
        field_ids: &Vec<String>,
        default_story_points: f64,
        velocity_in_story_points: f64,
    ) -> Results {
        let mut results = Results {
            default_story_points,
            num_complete: 0.0,
            num_incomplete_and_pointed: 0.0,
            num_incomplete_and_unpointed: 0.0,
            num_sprints_remaining: 0.0,
            unfinished_estimated_story_points: 0.0,
            unfinished_story_points: 0.0,
            unfinished_unestimated_story_points: 0.0,
            velocity_in_story_points,
        };

        for issue in issues {
            match classify(issue, field_ids) {
                ClassifiedIssue::Complete => {
                    results.num_complete = results.num_complete + 1.0;
                }
                ClassifiedIssue::IncompleteAndPointed(points) => {
                    results.num_incomplete_and_pointed = results.num_incomplete_and_pointed + 1.0;
                    results.unfinished_estimated_story_points =
                        results.unfinished_estimated_story_points + points;
                }
                ClassifiedIssue::IncompleteAndUnpointed => {
                    results.num_incomplete_and_unpointed =
                        results.num_incomplete_and_unpointed + 1.0;
                }
            };
        }

        results.unfinished_unestimated_story_points =
            results.num_incomplete_and_unpointed * results.default_story_points;
        results.unfinished_story_points =
            results.unfinished_estimated_story_points + results.unfinished_unestimated_story_points;
        results.num_sprints_remaining =
            results.unfinished_story_points / results.velocity_in_story_points;

        results
    }
}

pub fn run(args: &Estimate) {
    let client = RestClient::new(&args.jira_url, &args.jira_username, &args.jira_token).unwrap();

    let mut field_ids =
        jira::get_story_point_field_ids(&client, &args.jira_story_points_field).unwrap();
    field_ids.push("status".to_owned());

    let maybe_jql =
        jira::build_issue_search_jql(&args.jira_project, &args.jira_label, &args.jira_issue_type);
    if maybe_jql.is_err() {
        eprintln!("{}", maybe_jql.unwrap_err());
        process::exit(1);
    }

    let jql = maybe_jql.unwrap();
    if args.verbose {
        println!("Searching for issues with the following JQL:");
        println!("{}", jql.serialize_to_jql());
    }

    let issues = client.search_all(&field_ids, &jql).unwrap();

    let results = Results::tally(
        &issues,
        &field_ids,
        args.default_story_points,
        args.velocity_in_story_points,
    );

    if args.verbose {
        results.explain();
    } else {
        println!("{:.1}", results.num_sprints_remaining)
    }
}
