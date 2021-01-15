/*
 * Copyright (c) 2021 gematik GmbH
 * 
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 * 
 *    http://www.apache.org/licenses/LICENSE-2.0
 * 
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
 */

use std::fs::read_to_string;
use std::io::Error as IoError;
use std::process::Command;
use std::str::{from_utf8, Utf8Error};

use chrono::Utc;
use thiserror::Error;

fn main() -> Result<(), Error> {
    let mut git_head = read_to_string("../.git/HEAD").ok();

    match Command::new("git").arg("version").status() {
        Ok(s) if s.success() => (),
        _ => git_head = None,
    }

    let timestamp = get_timestamp();
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", timestamp);

    if let Some(git_head) = git_head {
        let git_head = if let Some(stripped) = git_head.strip_prefix("ref: ") {
            stripped
        } else {
            &git_head
        };

        println!("cargo:rerun-if-changed=../.git/HEAD");
        println!("cargo:rerun-if-changed=../.git/{}", git_head);

        match get_hash() {
            Ok(hash) => println!("cargo:rustc-env=GIT_HASH={}", hash),
            Err(err) => println!("cargo:warning=Unable get git hash: {}!", err),
        }

        match get_is_dirty() {
            Ok(is_dirty) => println!("cargo:rustc-env=GIT_DIRTY={}", is_dirty),
            Err(err) => println!("cargo:warning=Unable get git dirty flag: {}!", err),
        }

        match get_version_tag() {
            Ok(version_tag) => {
                println!("cargo:rustc-env=GIT_VERSION_TAG={}", version_tag);

                match get_commits_behind(&version_tag) {
                    Ok(commits_behind) => {
                        println!("cargo:rustc-env=GIT_COMMITS_BEHIND={}", commits_behind)
                    }
                    Err(err) => println!("cargo:warning=Unable get git commits behind: {}!", err),
                }
            }
            Err(err) => println!("cargo:warning=Unable get git version tag: {}!", err),
        }
    } else {
        println!("cargo:warning=Unable to get version information from git, using reduced version information!");
    }

    Ok(())
}

fn get_hash() -> Result<String, Error> {
    let lines = get_lines(Command::new("git").args(&["log", "-n", "1", "--pretty=format:%H"]))?;

    let hash = lines
        .into_iter()
        .next()
        .ok_or(Error::UnableToGetCommitHash)?;

    Ok(hash)
}

fn get_version_tag() -> Result<String, Error> {
    let lines = get_lines(Command::new("git").args(&[
        "describe",
        "--abbrev=0",
        "--tags",
        "--match",
        "[0-9]*.[0-9]*.[0-9]*",
    ]))?;

    let version_tag = lines
        .into_iter()
        .next()
        .ok_or(Error::UnableToGetCommitVersion)?;

    Ok(version_tag)
}

fn get_commits_behind(reference: &str) -> Result<String, Error> {
    let lines = get_lines(
        Command::new("git")
            .arg("rev-list")
            .arg(format!("{}..HEAD", reference)),
    )?;

    let commits_behind = lines.len().to_string();

    Ok(commits_behind)
}

fn get_is_dirty() -> Result<&'static str, Error> {
    let output = get_output(Command::new("git").args(&["status", "-s", "-uall"]))?;

    let is_dirty = if output.is_empty() { "0" } else { "1" };

    Ok(is_dirty)
}

fn get_timestamp() -> String {
    Utc::now().to_rfc3339()
}

fn get_lines(command: &mut Command) -> Result<Vec<String>, Error> {
    let lines = get_output(command)?
        .trim()
        .split('\n')
        .map(Into::into)
        .collect();

    Ok(lines)
}

fn get_output(command: &mut Command) -> Result<String, Error> {
    let output = command.output()?;

    if !output.status.success() {
        let stderr = from_utf8(&output.stderr)?;

        return Err(Error::CommandExecutionFailed(stderr.into()));
    }

    let output = from_utf8(&output.stdout)?.into();

    Ok(output)
}

#[derive(Error, Debug)]
#[allow(clippy::enum_variant_names)]
enum Error {
    #[error("IO Error: {0}")]
    IoError(IoError),

    #[error("UTF-8 Error: {0}")]
    Utf8Error(Utf8Error),

    #[error("Command Execution Failed: {0}")]
    CommandExecutionFailed(String),

    #[error("Unable to get Commit Version")]
    UnableToGetCommitVersion,

    #[error("Unable to get Commit Hash")]
    UnableToGetCommitHash,
}

impl From<IoError> for Error {
    fn from(err: IoError) -> Self {
        Self::IoError(err)
    }
}

impl From<Utf8Error> for Error {
    fn from(err: Utf8Error) -> Self {
        Self::Utf8Error(err)
    }
}
