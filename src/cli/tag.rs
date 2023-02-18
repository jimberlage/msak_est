use clap::Args;

use jimberlage_jira_client::{IssueEditUpdate, IssueEditUpdateLabel, RestClient};

#[derive(Debug, Args)]
pub struct Tag {
    #[arg(long)]
    pub jira_key: Vec<String>,

    #[arg(long)]
    pub jira_label: String,

    #[arg(long)]
    pub jira_token: String,

    #[arg(long)]
    pub jira_url: String,

    #[arg(long)]
    pub jira_username: String,
}

pub fn run(args: &Tag) {
    let client = RestClient::new(&args.jira_url, &args.jira_username, &args.jira_token).unwrap();

    for key in &args.jira_key {
        let update = IssueEditUpdate {
            labels: vec![IssueEditUpdateLabel::Add(args.jira_label.clone())],
        };

        client.edit_issue(key, &update).unwrap();
    }

    println!("Done!");
}
