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
use std::path::PathBuf;
use std::process::Command;

use pkg_config::Config as PkgConfig;

use super::env::env;

pub fn find_libxml(target: &str) -> (PathBuf, PathBuf) {
    let lib_dir = env("LIBXML_LIB_DIR").map(PathBuf::from);
    let include_dir = env("LIBXML_INCLUDE_DIR").map(PathBuf::from);

    match (lib_dir, include_dir) {
        (Some(lib_dir), Some(include_dir)) => (lib_dir, include_dir),
        (lib_dir, include_dir) => {
            let dirs = env("LIBXML_DIR")
                .map(PathBuf::from)
                .map(|libxml_dir| (libxml_dir.join("lib"), libxml_dir.join("include")))
                .unwrap_or_else(|| find_libxml_dir(&target));

            let lib_dir = lib_dir.unwrap_or(dirs.0);
            let include_dir = include_dir.unwrap_or(dirs.1);

            (lib_dir, include_dir)
        }
    }
}

fn find_libxml_dir(target: &str) -> (PathBuf, PathBuf) {
    let host = var("HOST").unwrap();

    if let Ok(lib) = PkgConfig::new().probe("libxml-2.0") {
        let lib_dir = lib
            .link_paths
            .into_iter()
            .next()
            .expect("Unable to find link path");
        let include_dir = lib
            .include_paths
            .into_iter()
            .next()
            .expect("Unable to find include path");

        return (lib_dir, include_dir);
    }

    let mut msg = format!(
        "

Could not find directory of libxml installation, and this `-sys` crate cannot
proceed without this knowledge. If libxml is installed and this crate had
trouble finding it, you can set the `LIBXML_DIR` environment variable for the
compilation process.

Make sure you also have the development packages of libxml installed.

$HOST = {}
$TARGET = {}
xml2-sys = {}
",
        host,
        target,
        env!("CARGO_PKG_VERSION")
    );

    if host.contains("unknown-linux")
        && target.contains("unknown-linux-gnu")
        && Command::new("pkg-config").output().is_err()
    {
        msg.push_str(
            "

It looks like you're compiling on Linux and also targeting Linux. Currently this
requires the `pkg-config` utility to find libxml but unfortunately `pkg-config`
could not be found. If you have libxml installed you can likely fix this by
installing `pkg-config`.
",
        );
    }

    if host.contains("windows") && target.contains("windows-gnu") {
        msg.push_str(
            "

It looks like you're compiling for MinGW but you may not have either libxmldev or
pkg-config installed.
",
        );
    }

    panic!(msg);
}
