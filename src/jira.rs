/// Contains all code related to interfacing with JIRA.
/// This includes functionality for getting projects and breaking them down into initiatives.
use std::{
    collections::HashMap,
    fmt::{self, Display},
};

use base64::{
    self,
    engine::{GeneralPurpose, GeneralPurposeConfig},
    Engine,
};
use reqwest::{
    self,
    blocking::{Client, ClientBuilder, RequestBuilder},
    header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE},
};
use serde::{ser::SerializeMap, Deserialize, Serialize, Serializer};
use serde_json::value::Value as JSONValue;

use crate::util;

use self::jql::{JQLClause, JQLStatement, JQLValue};

pub mod jql;

#[derive(Debug, Deserialize)]
pub struct Field {
    pub id: String,
    pub name: String,
}

/// Represents an issue in JIRA, as returned by a search request [1].
///
/// [1]: https://docs.atlassian.com/software/jira/docs/api/REST/9.6.0/#api/2/search-searchUsingSearchRequest
#[derive(Debug, Deserialize)]
pub struct Issue {
    pub id: String,
    pub key: String,
    pub fields: HashMap<String, JSONValue>,
}

impl Issue {
    pub fn status_category(&self) -> Option<String> {
        if let Some(status_obj) = self.fields.get("status") {
            let path = vec!["statusCategory", "name"];

            return util::get_string_in_json(status_obj, &path);
        }

        None
    }

    pub fn story_points(&self, field_ids: &Vec<String>) -> Option<f64> {
        for field_id in field_ids {
            if let Some(JSONValue::Number(points)) = self.fields.get(field_id) {
                if let Some(points_f64) = points.as_f64() {
                    return Some(points_f64);
                }
            }
        }

        None
    }
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    issues: Vec<Issue>,
}

#[derive(Debug, Serialize)]
struct SearchRequest {
    fields: Vec<String>,

    jql: JQLStatement,

    #[serde(rename(serialize = "maxResults"))]
    max_results: u64,

    #[serde(rename(serialize = "startAt"))]
    start_at: u64,
}

#[derive(Clone, Debug)]
pub enum IssueEditUpdateLabel {
    Add(String),
    /* Remove would go here */
}

impl Serialize for IssueEditUpdateLabel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            IssueEditUpdateLabel::Add(label) => {
                let mut m = serializer.serialize_map(Some(1))?;
                m.serialize_entry("add", label)?;
                m.end()
            }
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct IssueEditUpdate {
    pub labels: Vec<IssueEditUpdateLabel>,
}

#[derive(Clone, Debug, Serialize)]
pub struct IssueEditRequest {
    pub update: IssueEditUpdate,
}

#[derive(Debug)]
pub struct RestClientInitializationError(pub reqwest::Error);

impl Display for RestClientInitializationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "There was a problem initializing a connection to JIRA to make a reusable client.  It's worth checking that you have the right JIRA URL specified.  You can run the command with --debug to see the full error.")
    }
}

pub struct RestClient {
    base_url: String,
    client: Client,
}

impl RestClient {
    /// Initialize a RestClient for the URL, with the given username and token.
    pub fn new(
        url: &str,
        username: &str,
        token: &str,
    ) -> Result<Self, RestClientInitializationError> {
        let base64_engine =
            GeneralPurpose::new(&base64::alphabet::URL_SAFE, GeneralPurposeConfig::new());

        let mut default_headers = HeaderMap::new();
        default_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        default_headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
        Self::add_auth_header(&mut default_headers, &base64_engine, username, token);

        let client = ClientBuilder::new()
            .default_headers(default_headers)
            .build()
            .map_err(|e| RestClientInitializationError(e))?;

        Ok(RestClient {
            base_url: format!("{}/rest/api/3", url),
            client,
        })
    }

    fn add_auth_header(
        headers: &mut HeaderMap,
        base64_engine: &GeneralPurpose,
        username: &str,
        token: &str,
    ) {
        let encoded = base64_engine.encode(format!("{}:{}", username, token));
        // Unwrap here is considered safe since the method returns an error if the input is out of bounds, which would
        // have to be a bug in the base64 library.
        let mut auth_header_value =
            HeaderValue::from_str(format!("Basic {}", encoded).as_str()).unwrap();
        auth_header_value.set_sensitive(true);
        headers.insert(AUTHORIZATION, auth_header_value);
    }

    fn get(&self, path: &str) -> RequestBuilder {
        self.client.get(format!("{}/{}", self.base_url, path))
    }

    fn post(&self, path: &str) -> RequestBuilder {
        self.client.post(format!("{}/{}", self.base_url, path))
    }

    fn put(&self, path: &str) -> RequestBuilder {
        self.client.put(format!("{}/{}", self.base_url, path))
    }

    pub fn get_fields(&self) -> Result<Vec<Field>, reqwest::Error> {
        let response = self.get("/field").send()?;
        let fields: Vec<Field> = response.json()?;

        Ok(fields)
    }

    pub fn get_story_point_field_ids(
        &self,
        jira_story_point_field: &str,
    ) -> Result<Vec<String>, reqwest::Error> {
        let fields = self.get_fields()?;
        let field_ids = fields
            .iter()
            .filter_map(|field| {
                if jira_story_point_field == &field.name {
                    Some(field.id.to_owned())
                } else {
                    None
                }
            })
            .collect();

        Ok(field_ids)
    }

    fn single_page_search(
        &self,
        fields: &Vec<String>,
        jql: &JQLStatement,
        start_at: u64,
        max_results: u64,
    ) -> Result<SearchResponse, reqwest::Error> {
        let response = self
            .post("/search")
            .json(&SearchRequest {
                fields: fields.to_vec(),
                jql: jql.clone(),
                start_at,
                max_results,
            })
            .send()?;
        response.json()
    }

    pub fn search(
        &self,
        fields: &Vec<String>,
        jql: &JQLStatement,
    ) -> Result<Vec<Issue>, reqwest::Error> {
        let mut start_at = 0u64;
        let max_results = 100u64;
        let mut result = vec![];

        loop {
            let mut response = self.single_page_search(fields, jql, start_at, max_results)?;
            let num_responses = response.issues.len() as u64;
            result.append(&mut response.issues);

            if num_responses < max_results {
                break;
            }

            start_at = start_at + num_responses
        }

        Ok(result)
    }

    pub fn edit_issue(&self, key: &str, update: &IssueEditUpdate) -> Result<(), reqwest::Error> {
        let path = format!("/issue/{}", key);
        let response = self
            .put(&path)
            .json(&IssueEditRequest {
                update: update.clone(),
            })
            .send()?;
        response.json()
    }
}

pub fn build_issue_search_jql(
    projects: &Vec<String>,
    labels: &Vec<String>,
    included_issue_types: &Vec<String>,
) -> Result<jql::JQLStatement, String> {
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
