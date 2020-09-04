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

use ref_erx_fd_server::{error::Error, logging::init_logger, service::Service};

fn main() -> Result<(), Error> {
    let opts = Options::from_args();

    init_logger(&opts.log_config)?;

    let mut runtime = Builder::new().threaded_scheduler().enable_all().build()?;

    runtime.block_on(async move {
        Service::new(opts.vau_key, opts.vau_cert)
            .listen(&opts.server_addr)?
            .run()
            .await?;

        Ok(())
    })
}

#[derive(Clone, StructOpt)]
struct Options {
    #[structopt(short = "v", long = "vau-key")]
    vau_key: PathBuf,

    #[structopt(short = "t", long = "vau-cert")]
    vau_cert: PathBuf,

    #[structopt(short = "c", long = "config", default_value = "./log4rs.yml")]
    log_config: PathBuf,

    #[structopt(short = "l", long = "listen", default_value = "[::]:3000")]
    server_addr: String,
}
