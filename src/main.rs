use regex_lite::Regex;
use rpassword::read_password;
use serde::Deserialize;
use std::io::{stderr, stdin, BufRead, Result as IoResult, Write};
use std::process::exit;
use std::thread::sleep;
use std::time::Duration;
use ureq::{get, post};

const PIPELINE: &str =
    r"\A(https?://[-.0-9a-zA-Z]+)/([-./0-9a-zA-Z]+?)(?:/-)?/pipelines/(\d+)\s*\z";

fn main() -> IoResult<()> {
    let mut std_err = stderr().lock();
    let mut parent_url = String::new();

    write!(std_err, "GitLab parent pipeline URL: ")?;
    stdin().lock().read_line(&mut parent_url)?;

    let parent = match PipelineUrl::parse(&parent_url) {
        None => {
            writeln!(
                std_err,
                "Invalid pipeline URL, doesn't match pattern: {}",
                PIPELINE
            )?;

            exit(1);
        }
        Some(v) => v,
    };

    write!(std_err, "GitLab API token with api scope:")?;

    let token = read_password()?;

    let url = format!(
        "{}/api/v4/projects/{}/pipelines/{}/bridges?scope[]=running&scope[]=pending&per_page=100",
        parent.gitlab, parent.project, parent.id
    );

    match get(url.clone())
        .header("PRIVATE-TOKEN", token.clone())
        .call()
    {
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
                    match PipelineUrl::parse(&bridge.downstream_pipeline.web_url) {
                        None => {
                            writeln!(
                                std_err,
                                "Invalid downstream pipeline URL, doesn't match pattern {}: {}",
                                PIPELINE, bridge.downstream_pipeline.web_url
                            )?;

                            exit(1);
                        }
                        Some(child) => {
                            let url = format!(
                                "{}/api/v4/projects/{}/pipelines/{}/jobs?scope[]=manual",
                                child.gitlab, child.project, child.id
                            );

                            match get(url.clone())
                                .header("PRIVATE-TOKEN", token.clone())
                                .call()
                            {
                                Err(err) => {
                                    writeln!(std_err, "GET {}: {}", url, err)?;
                                    exit(1);
                                }
                                Ok(mut resp) => match resp.body_mut().read_json::<Vec<Job>>() {
                                    Err(err) => {
                                        writeln!(
                                            std_err,
                                            "Got invalid JSON from GET {}: {}",
                                            url, err
                                        )?;

                                        exit(1);
                                    }
                                    Ok(body) => {
                                        for job in body {
                                            let url = format!(
                                                "{}/api/v4/projects/{}/jobs/{}/play",
                                                child.gitlab, child.project, job.id
                                            );

                                            match post(url.clone())
                                                .header("PRIVATE-TOKEN", token.clone())
                                                .send_empty()
                                            {
                                                Err(err) => {
                                                    writeln!(std_err, "POST {}: {}", url, err)?;
                                                    exit(1);
                                                }
                                                Ok(_) => {
                                                    writeln!(std_err, "{}", job.web_url)?;
                                                    sleep(Duration::from_secs(900));
                                                }
                                            }
                                        }
                                    }
                                },
                            }
                        }
                    }
                }
            }
        },
    }

    Ok(())
}

struct PipelineUrl<'a> {
    gitlab: &'a str,
    project: String,
    id: &'a str,
}

impl<'a> PipelineUrl<'a> {
    fn parse(s: &'a String) -> Option<Self> {
        match Regex::new(PIPELINE).unwrap().captures(s.as_str()) {
            None => None,
            Some(cap) => Some(Self {
                gitlab: cap.get(1).unwrap().as_str(),
                project: cap.get(2).unwrap().as_str().replace("/", "%2F"),
                id: cap.get(3).unwrap().as_str(),
            }),
        }
    }
}

#[derive(Deserialize)]
struct Bridge {
    downstream_pipeline: DownstreamPipeline,
}

#[derive(Deserialize)]
struct DownstreamPipeline {
    web_url: String,
}

#[derive(Deserialize)]
struct Job {
    web_url: String,
    id: u64,
}
