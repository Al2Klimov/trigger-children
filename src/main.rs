use regex_lite::Regex;
use rpassword::read_password;
use std::io::{stderr, stdin, BufRead, Result as IoResult, Write};
use std::process::exit;

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

    Ok(())
}
