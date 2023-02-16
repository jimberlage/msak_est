use std::{io, process};

use clap::Args;
use csv;
use serde::Serialize;

use crate::jira::{self, RestClient};

#[derive(Debug, Args)]
pub struct CSV {
    #[arg(long)]
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
}

#[derive(Debug, Serialize)]
struct CSVIssue {
    #[serde(rename(serialize = "ID"))]
    key: String,

    #[serde(rename(serialize = "Story Points"))]
    story_points: Option<f64>,

    #[serde(rename(serialize = "Status"))]
    status: Option<String>,

    #[serde(rename(serialize = "Link"))]
    link: String,
}

pub fn run(args: &CSV) {
    let client = RestClient::new(&args.jira_url, &args.jira_username, &args.jira_token);

    let mut field_ids = client
        .get_story_point_field_ids(&args.jira_story_points_field)
        .unwrap();
    field_ids.push("status".to_owned());

    let jql =
        jira::build_issue_search_jql(&args.jira_project, &args.jira_label, &args.jira_issue_type);
    if jql.is_err() {
        eprintln!("{}", jql.unwrap_err());
        process::exit(1);
    }

    let issues = client.search(&field_ids, &jql.unwrap()).unwrap();
    let mut writer = csv::Writer::from_writer(io::stdout());

    for issue in issues {
        writer
            .serialize(CSVIssue {
                key: issue.key.clone(),
                story_points: issue.story_points(&field_ids),
                status: issue.status_category(),
                link: format!("{}/browse/{}", &args.jira_url, &issue.key),
            })
            .unwrap();
    }

    writer.flush().unwrap();
}
