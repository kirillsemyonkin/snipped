use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fs::File;
use std::io;
use std::io::stdout;
use std::io::Read;
use std::io::Seek;
use std::io::Write;
use std::path::Path;
use std::rc::Rc;

use indexmap::IndexMap;
use log::warn;
use petname::petname;
use reqwest::blocking::Client;
use reqwest::header;
use serde::Deserialize;
use tempfile::tempfile;
use url::Url;

use crate::ask;
use crate::USER_AGENT;

#[derive(Debug, Clone)]
pub enum Error {
    GitHubFormat,

    GistFormat,
    GistNotFound(String),
    GistUserNotFound(String),

    URLFormat,
    PathIsNotFile,

    NetworkError(Rc<::reqwest::Error>),
    IOError(Rc<io::Error>),

    CancelledRun,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use Error::*;
        match self {
            GitHubFormat => write!(
                f,
                "Could not parse `github:` formatted input"
            ),
            GistFormat => write!(
                f,
                "Could not parse `gist:` formatted input"
            ),
            GistNotFound(gist) => write!(f, "Gist {gist} not found"),
            GistUserNotFound(user) => write!(f, "User {user} not found"),
            URLFormat => write!(f, "Could not parse URL"),
            PathIsNotFile => write!(
                f,
                "Path does not exist or does not point to a file"
            ),
            NetworkError(e) => write!(f, "Network error: {e}"),
            IOError(e) => write!(f, "IO error: {e}"),
            CancelledRun => write!(f, "Cancelled"),
        }
    }
}

impl std::error::Error for Error {
}

impl From<::reqwest::Error> for Error {
    fn from(err: ::reqwest::Error) -> Self { Error::NetworkError(err.into()) }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self { Error::IOError(err.into()) }
}

#[derive(Deserialize)]
pub struct GistFile {
    raw_url: Url,
}

#[derive(Deserialize)]
pub struct Gist {
    files: IndexMap<String, GistFile>,
}

pub fn parse_and_load(target: &str) -> Result<Box<dyn Read>, Error> {
    use Error::*;
    let target = target.trim();

    let client = Client::new();

    // GitHub: `github:{user}/{repo}/{filepath}(#{branch})`
    if target.starts_with("github:") {
        let target = &target[5..];

        let (user, rest) = target
            .split_once('/')
            .ok_or(GitHubFormat)?;
        let (repo, filepath) = rest
            .split_once('/')
            .ok_or(GitHubFormat)?;

        let url = Url::parse(&format!(
            "https://raw.githubusercontent.com/{user}/{repo}/master/{filepath}"
        ))
        .map_err(|_| GitHubFormat)?;
        let branch = url
            .fragment()
            .unwrap_or("master");

        let url = Url::parse(&format!(
            "https://raw.githubusercontent.com/{user}/{repo}/{branch}/{filepath}"
        ))
        .map_err(|_| GitHubFormat)?;

        return target_from_url(client, url);
    }

    // Gist: `gist:{gist_id}(#{file})` `gist:{user}/{file}`
    if target.starts_with("gist:") {
        let target = &target[5..];
        match target.split_once('/') {
            None => {
                let gist_id = target;

                let url = Url::parse(&format!(
                    "https://api.github.com/gists/{gist_id}"
                ))
                .map_err(|_| GistFormat)?;

                let file = url
                    .fragment()
                    .unwrap_or(gist_id)
                    .to_string();

                let response = client
                    .get(url)
                    .header(header::USER_AGENT, USER_AGENT)
                    .send()?;
                if response.status() == 404 {
                    return Err(GistNotFound(
                        gist_id.to_string(),
                    ));
                }
                let gist = response
                    .error_for_status()?
                    .json::<Gist>()?;

                let file = gist
                    .files
                    .iter()
                    .find(|(name, _)| **name == file)
                    .map(|(_, gist)| gist)
                    .or_else(|| {
                        gist.files
                            .iter()
                            .next()
                            .map(|(_, gist)| gist)
                    })
                    .expect("gist should have at least 1 file");

                return target_from_url(
                    client,
                    file.raw_url
                        .clone(),
                );
            },
            Some((user, file)) =>
                for page in 1.. {
                    let url = Url::parse(&format!(
                        "https://api.github.com/users/{user}/gists?page={page}&per_page=100",
                    ))
                    .map_err(|_| GistFormat)?;

                    let response = client
                        .get(url)
                        .header(header::USER_AGENT, USER_AGENT)
                        .send()?;
                    if response.status() == 404 {
                        return Err(GistUserNotFound(
                            user.to_string(),
                        ));
                    }
                    let response = response.error_for_status()?;

                    let gists = response.json::<Vec<Gist>>()?;
                    if gists.is_empty() {
                        return Err(GistNotFound(file.to_string()));
                    }

                    for mut gist in gists {
                        if let Some(file) = gist
                            .files
                            .swap_remove(file)
                        {
                            return target_from_url(client, file.raw_url);
                        }
                    }
                },
        }
    }

    // URL: `https://example.com`
    if target.starts_with("https://") || target.starts_with("http://") {
        return target_from_url(
            client,
            Url::parse(target).map_err(|_| URLFormat)?,
        );
    }

    // Path
    target_from_path(target)
}

fn target_from_path(target: impl AsRef<Path>) -> Result<Box<dyn Read>, Error> {
    use Error::*;
    let file = File::open(target).map_err(|e| match e {
        e if e.kind() == io::ErrorKind::NotFound => PathIsNotFile,
        e => IOError(e.into()),
    })?;
    Ok(Box::new(file))
}

fn target_from_url(client: Client, target: Url) -> Result<Box<dyn Read>, Error> {
    let mut response = client
        .get(target.clone())
        .header(header::USER_AGENT, USER_AGENT)
        .send()?
        .error_for_status()?;

    let mut file = tempfile()?;
    io::copy(&mut response, &mut file)?;

    warn!("Running snippets from the internet is not safe. Be careful.");
    println!("Snippet: {target}");
    stdout().flush()?;

    println!();
    file.seek(io::SeekFrom::Start(0))?;
    io::copy(&mut file, &mut stdout())?;
    println!();
    println!();
    stdout().flush()?;

    let confirm = petname(2, " ").unwrap();
    let result = ask(&format!(
        "To run this snippet, type `{confirm}`, or anything else to cancel"
    ))?;
    if confirm != result {
        return Err(Error::CancelledRun);
    }

    file.seek(io::SeekFrom::Start(0))?;
    Ok(Box::new(file))
}
