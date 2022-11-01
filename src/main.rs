use chrono::prelude::*;
use clap::Parser;
use reqwest::header::USER_AGENT;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

mod macros;
mod opts;

use opts::Opts;

pub static DEBUG: bool = true;

#[derive(Deserialize, Debug)]
struct Entry {
    user: User,
    id: u32,
    submitted_at: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Author {
    user: User,
    created_at: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct User {
    login: String,
    id: u32,
}

#[derive(Serialize, Debug, Clone)]
pub struct Data {
    path_segments: (String, String, String),
    api_url: Option<String>,
    first_review_date: Option<String>,
    author: Option<Author>,
}

#[derive(Deserialize, Debug)]
struct RateLimit {
    resources: RateLimitResources,
}
#[derive(Deserialize, Debug)]
struct RateLimitResources {
    core: RateLimitCore,
}
#[derive(Deserialize, Debug)]
struct RateLimitCore {
    remaining: u32,
    reset: i64,
}

// TODO: show how many requests left; UTC, ISO 8601
// TODO: github api token for personal private repos
// TODO: print when was PR created
// TODO: Change dates to chrono stuff
// TODO: time is not local
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Opts::parse();
    let client = reqwest::Client::new();
    let mut entries: HashMap<String, Data> = HashMap::new();

    for url in &args.links {
        let path_segments = url
            .path_segments()
            .map(|s| s.map(|c| c.to_string()).collect::<Vec<_>>())
            .unwrap();

        let scheme = url.scheme();
        let host = url.host_str().unwrap();
        let path = url.path();

        let url = [scheme, "://", host, path].join("");

        if path_segments.len() != 4 {
            usage!(
                "Invalid link - ",
                url,
                "\nLink should look like: https://github.com/{OWNER}/{REPO}/pull/{PULL_NUMBER}"
            );
        }

        let (owner, repo, pull_number) = (
            path_segments[0].clone(),
            path_segments[1].clone(),
            path_segments[3].clone(),
        );

        let request_url = format!(
            "https://api.github.com/repos/{}/{}/pulls/{}/reviews",
            owner, repo, pull_number
        );

        let data = Data {
            path_segments: (owner, repo, pull_number),
            api_url: Some(request_url),
            author: None,
            first_review_date: None,
        };

        entries.insert(url, data);
    }

    let entries = fetch(&client, &mut entries).await?;

    if let Some(output) = args.output {
        match save(&output, &entries) {
            Ok(_) => {
                println!("Successfully written to `{}`", output.to_str().unwrap())
            }
            Err(_) => {
                let output = output.to_str().unwrap();
                usage!("Couldn't write to ", output);
            }
        }
    } else {
        println!("{:#?}", entries);
    }

    let response = fetch_rate_limit(&client).await;
    match response {
        Ok(rate_limit) => {
            let core = rate_limit.resources.core;
            println!("Requsts remaining: {:?}", core.remaining);
            println!("Resets at: {:?}", Utc.timestamp(core.reset, 0).to_rfc2822());
        }
        Err(_) => {
            println!("Could not fetch rate limits!");
        }
    }

    Ok(())
}

async fn fetch_rate_limit(client: &Client) -> Result<RateLimit, Box<dyn std::error::Error>> {
    let request_url = "https://api.github.com/rate_limit".to_string();
    let response = send_request(client, request_url).await?;
    Ok(response)
}

async fn fetch(
    client: &Client,
    entries: &HashMap<String, Data>,
) -> Result<HashMap<String, Data>, Box<dyn std::error::Error>> {
    let mut updated_entries = HashMap::new();

    for (key, data) in entries {
        let request_url = data.api_url.clone().unwrap();

        let users: Vec<Entry> = send_request(client, request_url).await?;

        let mut new_data = data.clone();

        if users.len() > 0 {
            // TODO: NO NO NO AUTHOR
            new_data.first_review_date = Some(users.get(0).unwrap().submitted_at.clone());
        }

        let (owner, repo, pull_number) = &new_data.path_segments;
        let request_url = format!(
            "https://api.github.com/repos/{}/{}/pulls/{}",
            owner, repo, pull_number
        );

        let response = send_request(client, request_url).await?;

        new_data.author = Some(response);

        updated_entries.insert(key.clone(), new_data);
    }

    Ok(updated_entries)
}

async fn send_request<T>(
    client: &Client,
    request_url: String,
) -> Result<T, Box<dyn std::error::Error>>
where
    for<'a> T: Deserialize<'a>,
{
    println!("Sending request to {}", request_url);
    let response = client
        .get(request_url)
        .header(USER_AGENT, "reqwest")
        .send()
        .await?;

    let response: T = response.json().await?;

    Ok(response)
}

impl fmt::Display for Data {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PR opened on: {}\nFirst review received on: {}",
            self.author.as_ref().unwrap().created_at,
            self.first_review_date
                .clone()
                .unwrap_or(String::from("None"))
        )
    }
}

pub fn save(output: &PathBuf, m: &HashMap<String, Data>) -> Result<(), Box<dyn std::error::Error>> {
    let data: serde_json::Value = m
        .iter()
        .map(|(key, data)| {
            return json!({
                "pr_url": key,
                "pr_open_date": format!("{}", data.author.clone().unwrap().created_at.parse::<DateTime<Utc>>().unwrap().to_rfc2822()),
                "first_review_date": match data.first_review_date.clone() {
                    Some(date) => format!("{}", date.parse::<DateTime<Utc>>().unwrap().to_rfc2822()),
                    None => format!("")
                },
                "author": data.author.as_ref().unwrap().user.login,
            });
        })
        .collect::<_>();

    std::fs::write(output, data.to_string())?;

    Ok(())
}
