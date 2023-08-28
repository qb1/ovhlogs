use bytes::Bytes;
use chrono::prelude::*;
use chrono::Duration;
use clap::Parser;
use reqwest::blocking::Client;

use std::fs;
use std::path::PathBuf;

/// Tools to fetch logs from a simple OVH web-hosting account.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Earliest date to get logs from. Logs at this date must exist.
    #[arg(short, long)]
    from: NaiveDate,

    /// Lastest date to get logs from (inclusive). Defaults to yesterday
    #[arg(short, long, default_value_t = Local::now().date_naive() - Duration::days(1))]
    to: NaiveDate,

    /// Whether to get today's partial logs. If true, will erase all existing "*.log" files from output folder before fetch the daily partial.
    #[arg(short = 'P', long, action)]
    partial: bool,

    /// Output folder to store logs in. There probably should not be any other files in there, as the tool will clean any unknown .log file.
    #[arg(short, long)]
    output: PathBuf,

    /// Basic auth username to use for queries
    #[arg(short, long)]
    user: String,

    /// Basic auth password to use for queries
    #[arg(short, long)]
    password: String,

    /// URL's account name
    #[arg(short = 'U', long)]
    url_user: String,

    /// URL's cluster name
    #[arg(short = 'C', long)]
    url_cluster: String,
}

fn fetch_log(
    client: &Client,
    day: NaiveDate,
    url_user: &str,
    url_cluster: &str,
    user: &str,
    password: &str,
) -> Bytes {
    // URL as expected on OVH
    let url = format!(
        "https://logs.{url_cluster}.hosting.ovh.net/{url_user}.{url_cluster}.hosting.ovh.net/\
        logs/logs-{0}/{url_user}.{url_cluster}.hosting.ovh.net-{1}.log.gz",
        day.format("%m-%Y"),
        day.format("%d-%m-%Y"),
    );

    return fetch_url(client, &url, user, password);
}

fn fetch_partial_log(
    client: &Client,
    day: NaiveDate,
    url_user: &str,
    url_cluster: &str,
    user: &str,
    password: &str,
) -> Bytes {
    // URL as expected on OVH for partial daily. Note it only works on current day
    let url = format!(
        "https://logs.{url_cluster}.hosting.ovh.net/{url_user}.{url_cluster}.hosting.ovh.net/\
        osl/{url_user}.{url_cluster}.hosting.ovh.net-{}.log",
        day.format("%d-%m-%Y"),
    );

    return fetch_url(client, &url, user, password);
}

fn fetch_url(client: &Client, url: &str, user: &str, password: &str) -> Bytes {
    let res = client
        .get(url)
        .basic_auth(&user, Some(&password))
        .send()
        .expect("Could not access URL");

    if !res.status().is_success() {
        panic!("[!] Could not access URL '{}' - '{}'", &url, res.status());
    }

    return res.bytes().unwrap();
}

fn clean_partial_logs(output: &PathBuf) {
    let paths = fs::read_dir(output).unwrap();

    for path in paths {
        let path = path.unwrap().path();
        if path.extension().map_or("", |x| x.to_str().unwrap()) != "log" {
            continue;
        }

        println!("[-] Removing partial daily {}...", path.display());
        fs::remove_file(path).expect("[!] Could not remove file.");
    }
}

fn build_filename(day: NaiveDate, url_user: &str, url_cluster: &str) -> String {
    // Use date format that keeps alphabetical order
    return format!(
        "{url_user}.{url_cluster}.hosting.ovh.net-{}",
        day.format("%Y-%m-%d")
    );
}

fn main() {
    let args = Args::parse();
    let client = Client::new();

    let mut day = args.from;
    while day <= args.to {
        let filepath = args
            .output
            .join(build_filename(day, &args.url_user, &args.url_cluster) + ".log.gz");

        if filepath.exists() {
            println!("[-] Skipping '{}': already exists...", filepath.display());
            day += Duration::days(1);
            continue;
        }

        println!("[-] Fetching {}", filepath.display());

        let content = fetch_log(
            &client,
            day,
            &args.url_user,
            &args.url_cluster,
            &args.user,
            &args.password,
        );

        fs::write(filepath, &content).expect("[!] Could not write output file");
        day += Duration::days(1);
    }

    if args.partial {
        let day = Local::now().date_naive();
        clean_partial_logs(&args.output);

        let filepath = args
            .output
            .join(build_filename(day, &args.url_user, &args.url_cluster) + ".log");

        println!("[-] Fetching partial daily {}", filepath.display());

        let content = fetch_partial_log(
            &client,
            day,
            &args.url_user,
            &args.url_cluster,
            &args.user,
            &args.password,
        );

        fs::write(filepath, &content).expect("[!] Could not write output file");
    }
}
