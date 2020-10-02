/*
 * Copyright (c) 2020 gematik GmbH
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

use std::path::PathBuf;
use std::sync::Arc;

use arc_swap::ArcSwapOption;
use futures::{future::FutureExt, select};
use structopt::StructOpt;
use tokio::{
    runtime::Builder,
    task::{spawn, LocalSet},
};
use url::Url;

use ref_erx_fd_server::{
    error::Error, logging::init_logger, service::Service, tsl::update as update_tsl,
};

fn main() -> Result<(), Error> {
    let opts = Options::from_args();

    init_logger(&opts.log_config)?;

    let mut runtime = Builder::new().threaded_scheduler().enable_all().build()?;

    runtime.block_on(run(opts))
}

async fn run(opts: Options) -> Result<(), Error> {
    let local = LocalSet::new();

    let tsl = Arc::new(ArcSwapOption::from(None));
    let handle = Service::new(opts.key, opts.cert, opts.token, tsl.clone())
        .listen(&opts.server_addr)?
        .run(&local)?;

    spawn(update_tsl(opts.tsl, tsl));

    local
        .run_until(async move {
            select! {
                ret = handle.clone().fuse() => ret?,
                ret = sig_handler().fuse() => {
                    let gracefull = ret?;

                    handle.stop(gracefull).await;
                },
            }

            Ok(())
        })
        .await
}

#[cfg(not(unix))]
async fn sig_handler() -> Result<bool, Error> {
    tokio::signal::ctrl_c().await?;

    Ok(true)
}

#[cfg(unix)]
async fn sig_handler() -> Result<bool, Error> {
    use futures::stream::StreamExt;
    use tokio::signal::unix::{signal, SignalKind};

    let mut sigint = signal(SignalKind::interrupt())?;
    let mut sigterm = signal(SignalKind::terminate())?;

    select! {
        x = sigint.next().fuse() => Ok(x.is_some()),
        _ = sigterm.next().fuse() => Ok(false),
    }
}

#[derive(Clone, StructOpt)]
struct Options {
    /// Private key of the ERX-FD server.
    #[structopt(verbatim_doc_comment, long = "key")]
    key: PathBuf,

    /// Certificate (with public key) of the ERX-FD server.
    #[structopt(verbatim_doc_comment, long = "cert")]
    cert: PathBuf,

    /// URI to get the public key for the access token from.
    /// This parameter accepts normal web URLs and files.
    /// e.g.:
    ///     * https://my-idp-service.de/pub_token
    ///     * file://idp/token.pub
    #[structopt(verbatim_doc_comment, long = "token")]
    token: Url,

    /// URL to load TSL (Trust Status List) from.
    /// This is the base URL the FD will look for two files in:
    ///     * {tsl-url}/TSL.xml - Actual TSL
    ///     * {tsl-url}/TSL.sha2 - SHA256 of the TSL.xml
    #[structopt(verbatim_doc_comment, long = "tsl")]
    tsl: Url,

    /// File to load log configuration from.
    #[structopt(
        verbatim_doc_comment,
        short = "c",
        long = "config",
        default_value = "./log4rs.yml"
    )]
    log_config: PathBuf,

    /// Address to listen to.
    #[structopt(
        verbatim_doc_comment,
        short = "l",
        long = "listen",
        default_value = "[::]:3000"
    )]
    server_addr: String,
}
