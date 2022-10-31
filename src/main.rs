use clap::Parser;
use reqwest::header::USER_AGENT;
use reqwest::{Client, Error};
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

#[derive(Deserialize, Debug)]
struct User {
    login: String,
    id: u32,
}

#[derive(Serialize, Debug, Clone)]
pub struct Data {
    path_segments: (String, String, String),
    api_url: Option<String>,
    pr_open_date: String,
    first_review_date: Option<String>,
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
    reset: u64,
}

// TODO: /rate_limit; https://docs.github.com/en/rest/rate-limit
// TODO: show how many requests left; UTC, ISO 8601
// TODO: github api token for personal private repos
// TODO: print when was PR created
#[tokio::main]
async fn main() -> Result<(), Error> {
    let args = Opts::parse();

    let mut entries: HashMap<String, Data> = HashMap::new();
    if args.links.is_empty() {
        // fetch().await?;
    } else {
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
                pr_open_date: String::from(""),
                first_review_date: None,
            };

            entries.insert(url, data);
        }

        fetch(&mut entries).await?;
    }

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
    }

    let client = reqwest::Client::new();
    let response = print_rate_limit(&client).await;
    match response {
        Ok(rate_limit) => {
            println!("{:#?}", rate_limit);
        }
        Err(_) => {
            println!("Could not fetch rate limits!");
        }
    }

    Ok(())
}

async fn print_rate_limit(client: &Client) -> Result<RateLimit, Error> {
    let request_url = "https://api.github.com/rate_limit";
    let response = client
        .get(request_url)
        .header(USER_AGENT, "reqwest")
        .send()
        .await?;

    let response: RateLimit = response.json().await?;

    Ok(response)
}

async fn fetch(m: &HashMap<String, Data>) -> Result<(), Error> {
    // let request_url = format!(
    //     "https://api.github.com/repos/{owner}/{repo}/pulls/{pull_number}/reviews",
    //     owner = "toptal",
    //     repo = "picasso",
    //     pull_number = 3180
    // );

    // println!("{}", request_url);
    // let response = clientto_string
    //     .get(&request_url)
    //     .header(USER_AGENT, "reqwest")
    //     .send()
    //     .await?;

    // // TODO: review of author does not count
    // // TODO: get author first
    // let users: Vec<Entry> = response.json().await?;
    // println!("{:#?}", users);

    Ok(())
}

impl fmt::Display for Data {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PR opened on: {}\nFirst review received on: {}",
            self.pr_open_date,
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
                "pr_open_date": data.pr_open_date,
                "first_review_date": data.first_review_date,
            });
        })
        .collect::<_>();

    std::fs::write(output, data.to_string())?;

    Ok(())
}
