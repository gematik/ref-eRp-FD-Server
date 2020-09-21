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

use structopt::StructOpt;
use tokio::runtime::Builder;
use url::Url;

use ref_erx_fd_server::{error::Error, logging::init_logger, service::Service};

fn main() -> Result<(), Error> {
    let opts = Options::from_args();

    init_logger(&opts.log_config)?;

    let mut runtime = Builder::new().threaded_scheduler().enable_all().build()?;

    runtime.block_on(run(opts))
}

async fn run(opts: Options) -> Result<(), Error> {
    Service::new(opts.key, opts.cert, opts.token)
        .listen(&opts.server_addr)?
        .run()
        .await?;

    Ok(())
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
