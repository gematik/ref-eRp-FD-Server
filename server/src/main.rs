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

use futures::{future::FutureExt, select};
use std::path::PathBuf;
use structopt::StructOpt;
use tokio::{runtime::Builder, task::LocalSet};
use url::Url;

use ref_erx_fd_server::{
    error::Error,
    logging::init_logger,
    service::Service,
    tasks::{
        tsl::{prepare_no_op, prepare_tsl},
        PukToken, Tsl,
    },
};

fn main() -> Result<(), Error> {
    let opts = Options::from_args();

    init_logger(&opts.log_config)?;

    let mut runtime = Builder::new().threaded_scheduler().enable_all().build()?;

    runtime.block_on(run(opts))
}

async fn run(opts: Options) -> Result<(), Error> {
    let local = LocalSet::new();

    let tsl = Tsl::from_url(opts.tsl, prepare_tsl, true);
    let bnetza = Tsl::from_url(opts.bnetza, prepare_no_op, false);
    let puk_token = PukToken::from_url(opts.token)?;

    let handle = Service::new(
        opts.enc_key,
        opts.enc_cert,
        opts.sig_key,
        opts.sig_cert,
        puk_token,
        tsl,
        bnetza,
    )
    .listen(&opts.server_addr)?
    .run(&local)?;

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
    /// Private key of the ERX-FD server used for encryption.
    #[structopt(verbatim_doc_comment, long = "enc-key")]
    enc_key: PathBuf,

    /// Certificate (with public key) of the ERX-FD server use for encryption.
    #[structopt(verbatim_doc_comment, long = "enc-cert")]
    enc_cert: PathBuf,

    /// Private key of the ERX-FD server used for signing.
    #[structopt(verbatim_doc_comment, long = "sig-key")]
    sig_key: PathBuf,

    /// Certificate (with public key) of the ERX-FD server use for signing.
    #[structopt(verbatim_doc_comment, long = "sig-cert")]
    sig_cert: PathBuf,

    /// URI to get the public key for the access token from.
    /// This parameter accepts normal web URLs and files.
    /// e.g.:
    ///     * https://my-idp-service.de/pub_token
    ///     * file://idp/token.pub
    #[structopt(verbatim_doc_comment, long = "token")]
    token: Url,

    /// BNetzA-VL containing all valid QES-CA-certificates in Germany.
    #[structopt(verbatim_doc_comment, long = "bnetza")]
    bnetza: Option<Url>,

    /// URL to load TSL (Trust Status List) from.
    #[structopt(verbatim_doc_comment, long = "tsl")]
    tsl: Option<Url>,

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
