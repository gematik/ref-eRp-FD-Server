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

mod env;
mod find_libxml;

use std::collections::HashSet;
use std::env::var;
use std::path::Path;

use find_libxml::find_libxml;

use env::env;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LinkMode {
    Static,
    Dynamic,
}

fn main() {
    let target = var("TARGET").unwrap();

    let (lib_dir, include_dir) = find_libxml(&target);

    println!("target={}", &target);
    println!("lib_dir={}", lib_dir.display());
    println!("include_dir={}", include_dir.display());

    println!(
        "cargo:rustc-link-search=native={}",
        lib_dir.to_string_lossy()
    );
    println!("cargo:include={}", include_dir.to_string_lossy());

    let libs = env("LIBXML_LIBS");
    let libs = libs.as_ref().and_then(|s| s.to_str());
    let libs = match libs {
        Some(libs) => libs.split(':').collect(),
        None => vec!["xml2"],
    };

    let mode = determine_mode(&target, Path::new(&lib_dir), &libs);
    for lib in libs.into_iter() {
        link_lib(&target, &lib_dir, &lib, mode);
    }
}

fn determine_mode(target: &str, lib_dir: &Path, libs: &[&str]) -> LinkMode {
    let kind = env("LIBXML_STATIC");
    let kind = kind.as_ref().and_then(|s| s.to_str());
    match kind {
        Some("0") => return LinkMode::Dynamic,
        Some(_) => return LinkMode::Static,
        None => {}
    }

    let files = lib_dir
        .read_dir()
        .unwrap()
        .map(|e| e.unwrap())
        .map(|e| e.file_name())
        .filter_map(|e| e.into_string().ok())
        .collect::<HashSet<_>>();

    let can_static = libs.iter().all(|l| files.contains(&format!("lib{}.a", l)));

    let can_dylib = libs.iter().all(|l| {
        if target.contains("windows") {
            files.contains(&format!("lib{}.dll.a", l))
        } else if target.contains("linux") {
            files.contains(&format!("lib{}.so", l))
        } else {
            false
        }
    });

    match (can_static, can_dylib) {
        (true, false) => LinkMode::Static,
        (false, true) => LinkMode::Dynamic,
        (true, true) => LinkMode::Dynamic,
        (false, false) => {
            panic!(
                "libxml libdir at `{}` does not contain the required files \
                 to either statically or dynamically link OpenSSL",
                lib_dir.display()
            );
        }
    }
}

fn link_lib(target: &str, lib_dir: &Path, lib: &str, mode: LinkMode) {
    match mode {
        LinkMode::Dynamic => {
            if target.contains("windows") {
                let path = lib_dir.join(format!("lib{}.dll.a", lib));

                println!("cargo:rustc-link-lib=dylib={}:{}", lib, path.display());
            } else if target.contains("linux") {
                println!("cargo:rustc-link-lib=dylib={}", lib);
            } else {
                panic!("Unable to determine library name for target {}", target);
            }
        }
        LinkMode::Static => {
            println!("cargo:rustc-link-lib=static={}", lib);
        }
    }
}
