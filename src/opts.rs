use clap::Parser;
use std::path::PathBuf;
use url::Url;

// TODO: date range flag to fetch all PRs from owner+repo combo
// TODO: input file + make use of https://docs.github.com/en/rest/overview/resources-in-the-rest-api#conditional-requests
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Opts {
    #[arg(short, long, num_args = 1.., help = "space separated list of PR links; LINK should look like: https://github.com/{OWNER}/{REPO}/pull/{PULL_NUMBER}")]
    pub links: Vec<Url>,

    #[arg(short, long, help = "output filepath")]
    pub output: Option<PathBuf>,
}
