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

use std::fs::{read, File};
use std::path::PathBuf;

use futures::{future::FutureExt, select};
use log::warn;
use openssl::{ec::EcKey, pkey::PKey, x509::X509};
use structopt::StructOpt;
use tokio::{runtime::Builder, task::LocalSet};
use url::Url;

use ref_erx_fd_server::{
    error::Error, logging::init_logger, pki_store::PkiStore, service::Service, state::State,
};

fn main() -> Result<(), Error> {
    let opts = Options::from_args();

    init_logger(&opts.log_config)?;

    let mut runtime = Builder::new().threaded_scheduler().enable_all().build()?;

    runtime.block_on(run(opts))
}

async fn run(opts: Options) -> Result<(), Error> {
    let sig_key = read(&opts.sig_key)?;
    let sig_key = EcKey::private_key_from_pem(&sig_key).map_err(Error::OpenSslError)?;
    let sig_key = PKey::from_ec_key(sig_key)?;

    let sig_cert = read(&opts.sig_cert)?;
    let sig_cert = X509::from_pem(&sig_cert)?;

    let enc_key = read(&opts.enc_key)?;
    let enc_key = EcKey::private_key_from_pem(&enc_key).map_err(Error::OpenSslError)?;

    let enc_cert = read(&opts.enc_cert)?;
    let enc_cert = X509::from_pem(&enc_cert)?;

    let local = LocalSet::new();

    let pki_store = PkiStore::new(enc_key, enc_cert, opts.tsl, opts.bnetza, opts.token)?;
    let state = State::new(sig_key, sig_cert, opts.max_communications);

    if let Some(path) = &opts.state {
        if path.is_file() {
            let file = File::open(path)?;

            let mut state = state.lock().await;
            state.load(file)?;
        }
    }

    let handle = Service::new(state.clone(), pki_store)
        .listen(&opts.server_addr)?
        .run(&local)?;

    let ret = local
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
        .await;

    if let Some(path) = &opts.state {
        match File::create(path) {
            Ok(file) => {
                let state = state.lock().await;
                if let Err(err) = state.save(file) {
                    warn!("Unable to write state to file: {}", err);
                }
            }
            Err(err) => warn!("Unable to open state file: {}", err),
        }
    }

    ret
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

    /// File to write the state of the service to.
    #[structopt(verbatim_doc_comment, long = "state")]
    state: Option<PathBuf>,

    /// URI to get the public key for the access token from.
    /// This parameter accepts normal web URLs and files.
    /// e.g.:
    ///     * https://my-idp-service.de/pub_token
    ///     * file://idp/token.pub
    #[structopt(verbatim_doc_comment, long = "token")]
    token: Url,

    /// BNetzA-VL containing all valid QES-CA-certificates in Germany.
    #[structopt(verbatim_doc_comment, long = "bnetza")]
    bnetza: Url,

    /// URL to load TSL (Trust Status List) from.
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

    /// Max number of communications for each task
    #[structopt(
        verbatim_doc_comment,
        short = "m",
        long = "max-communications",
        default_value = "10"
    )]
    max_communications: usize,
}
