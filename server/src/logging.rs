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

use std::env::var;
use std::ffi::OsStr;
use std::path::Path;

use log::LevelFilter;
use log4rs::{
    append::console::ConsoleAppender,
    config::{Appender, Config, Logger, Root},
    encode::pattern::PatternEncoder,
    file::Deserializers,
    init_config, load_config_file,
};

use crate::error::Error;

pub fn init_logger(config: &Path) -> Result<(), Error> {
    let config =
        load_config_file(config, Deserializers::default()).or_else(|_| create_default_config())?;

    init_config(config)?;

    Ok(())
}

fn create_default_config() -> Result<Config, Error> {
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%d %H:%M:%S%.6f)(utc)} {l} {t} - {m}{n}",
        )))
        .build();

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .logger(Logger::builder().appender("stdout").build(
            "access_log",
            level_from_env("access_log_level", LevelFilter::Info),
        ))
        .logger(Logger::builder().appender("stdout").build(
            "req_res_log",
            level_from_env("req_res_log_level", LevelFilter::Info),
        ))
        .logger(Logger::builder().appender("stdout").build(
            "ref_erx_fd_server",
            level_from_env("ref_erx_fd_server_log_level", LevelFilter::Debug),
        ))
        .build(Root::builder().appender("stdout").build(LevelFilter::Info))?;

    Ok(config)
}

fn level_from_env<K: AsRef<OsStr>>(key: K, default: LevelFilter) -> LevelFilter {
    match var(key) {
        Ok(v) if v.to_lowercase() == "off" => LevelFilter::Off,
        Ok(v) if v.to_lowercase() == "error" => LevelFilter::Error,
        Ok(v) if v.to_lowercase() == "warn" => LevelFilter::Warn,
        Ok(v) if v.to_lowercase() == "info" => LevelFilter::Info,
        Ok(v) if v.to_lowercase() == "debug" => LevelFilter::Debug,
        Ok(v) if v.to_lowercase() == "trace" => LevelFilter::Trace,
        _ => default,
    }
}
