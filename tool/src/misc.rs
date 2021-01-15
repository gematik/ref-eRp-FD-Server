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

use std::fs::{read, write};
use std::io::{stdin, stdout, Read, Write};
use std::path::PathBuf;

pub fn read_input(input: &Option<PathBuf>) -> Vec<u8> {
    if let Some(input) = input {
        read(input).expect("Unable to read input file!")
    } else {
        let mut input = Vec::new();
        stdin()
            .lock()
            .read_to_end(&mut input)
            .expect("Unable to read from stdin");

        input
    }
}

pub fn write_output(output: &Option<PathBuf>, data: &[u8]) {
    if let Some(output) = output {
        write(output, data).expect("Unable to write to output file");
    } else {
        stdout().write_all(data).expect("Unable to write to stdout");
    }
}
