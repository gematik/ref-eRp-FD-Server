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

use std::io::Write;

use serde::Serialize;
use serde_json::{
    from_str,
    ser::{CompactFormatter, Serializer},
    Error, Value,
};

pub fn canonize<W>(s: &str, writer: W) -> Result<(), Error>
where
    W: Write,
{
    let mut value = from_str::<Value>(s)?;
    if let Some(obj) = value.as_object_mut() {
        obj.remove("meta");
        obj.remove("text");
        obj.remove("signature");
    }

    let formatter = CompactFormatter;
    let mut serializer = Serializer::with_formatter(writer, formatter);
    value.serialize(&mut serializer)?;

    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::fs::read_to_string;
    use std::str::from_utf8;

    #[test]
    fn test_json_canonization() {
        let input = read_to_string("./examples/task.json").unwrap();

        let mut buffer = Vec::new();
        canonize(&input, &mut buffer).unwrap();

        let expected = read_to_string("./examples/task_canonical.json").unwrap();
        let expected = expected.trim();
        let actual = from_utf8(&buffer).unwrap();
        let actual = actual.trim();

        assert_eq!(expected, actual);
    }
}
