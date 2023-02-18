/// Contains all code related to interfacing with JIRA.
/// This includes functionality for getting projects and breaking them down into initiatives.
use std::fmt::{self, Display};

use jimberlage_jira_client::{
    jql::{JQLClause, JQLStatement, JQLValue},
    RestClient, SearchIssue,
};
use reqwest;

pub fn story_points(issue: &SearchIssue, field_ids: &Vec<String>) -> Option<f64> {
    for field_id in field_ids {
        if let Some(points) = issue.numeric_field(field_id) {
            return Some(points);
        }
    }

    None
}

pub fn get_story_point_field_ids(
    client: &RestClient,
    field_name: &str,
) -> Result<Vec<String>, reqwest::Error> {
    let fields = client.get_fields()?;
    let field_ids: Vec<String> = fields
        .iter()
        .filter_map(|field| {
            if field.name == field_name {
                Some(field.id.clone())
            } else {
                None
            }
        })
        .collect();

    Ok(field_ids)
}

#[derive(Debug)]
pub struct RestClientInitializationError(pub reqwest::Error);

impl Display for RestClientInitializationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "There was a problem initializing a connection to JIRA to make a reusable client.  It's worth checking that you have the right JIRA URL specified.  You can run the command with --debug to see the full error.")
    }
}

pub fn build_issue_search_jql(
    projects: &Vec<String>,
    labels: &Vec<String>,
    included_issue_types: &Vec<String>,
) -> Result<JQLStatement, String> {
    if projects.is_empty() && labels.is_empty() {
        return Err("This command will search all projects & labels.  To avoid crawling your entire JIRA instance, you must supply at least one project or a label to narrow the search.".to_owned());
    }

    let mut clauses: Vec<Box<JQLClause>> = vec![];

    if !projects.is_empty() {
        clauses.push(Box::new(JQLClause::In(
            "project".to_owned(),
            projects
                .iter()
                .map(|project| JQLValue::String(project.clone()))
                .collect(),
        )));
    }

    if !labels.is_empty() {
        clauses.push(Box::new(JQLClause::In(
            "labels".to_owned(),
            labels
                .iter()
                .map(|label| JQLValue::String(label.clone()))
                .collect(),
        )))
    }

    if !included_issue_types.is_empty() {
        clauses.push(Box::new(JQLClause::In(
            "issuetype".to_owned(),
            included_issue_types
                .iter()
                .map(|issue_type| JQLValue::String(issue_type.clone()))
                .collect(),
        )))
    }

    Ok(JQLStatement {
        clause: JQLClause::And(clauses),
    })
}
