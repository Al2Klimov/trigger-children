use regex_lite::Regex;
use rpassword::read_password;
use serde::Deserialize;
use std::io::{stderr, stdin, BufRead, Result as IoResult, Write};
use std::process::exit;
use ureq::get;

fn main() -> IoResult<()> {
    const PIPELINE: &str =
        r"\A(https?://[-.0-9a-zA-Z]+)/([-./0-9a-zA-Z]+?)(?:/-)?/pipelines/(\d+)\s*\z";

    let mut std_err = stderr().lock();
    let mut parent = String::new();

    write!(std_err, "GitLab parent pipeline URL: ")?;
    stdin().lock().read_line(&mut parent)?;

    let (gitlab, project, id) = match Regex::new(PIPELINE).unwrap().captures(parent.as_str()) {
        None => {
            writeln!(
                std_err,
                "Invalid pipeline URL, doesn't match pattern: {}",
                PIPELINE
            )?;

            exit(1);
        }
        Some(cap) => (
            cap.get(1).unwrap().as_str(),
            cap.get(2).unwrap().as_str(),
            cap.get(3).unwrap().as_str(),
        ),
    };

    write!(std_err, "GitLab API token with api scope:")?;

    let token = read_password()?;

    let url = format!(
        "{}/api/v4/projects/{}/pipelines/{}/bridges?scope[]=running",
        gitlab,
        project.replace("/", "%2F"),
        id
    );

    match get(url.clone()).header("PRIVATE-TOKEN", token).call() {
        Err(err) => {
            writeln!(std_err, "GET {}: {}", url, err)?;
            exit(1);
        }
        Ok(mut resp) => match resp.body_mut().read_json::<Vec<Bridge>>() {
            Err(err) => {
                writeln!(std_err, "Got invalid JSON from GET {}: {}", url, err)?;
                exit(1);
            }
            Ok(body) => {
                for bridge in body {
                    writeln!(std_err, "{}", bridge.downstream_pipeline.web_url)?;
                }
            }
        },
    }

    Ok(())
}

#[derive(Deserialize)]
struct Bridge {
    downstream_pipeline: DownstreamPipeline,
}

#[derive(Deserialize)]
struct DownstreamPipeline {
    web_url: String,
}
