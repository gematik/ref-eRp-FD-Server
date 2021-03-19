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

mod error;
mod node;
mod nodeset;
mod transform;

#[macro_use]
extern crate bitflags;

pub use error::*;
pub use node::*;
pub use nodeset::*;
pub use transform::*;

#[cfg(test)]
mod tests {
    use super::*;

    use libxml::*;

    #[test]
    fn test_simple_verify() {
        let doc = Doc::from_file("./examples/simple.xml").unwrap();
        let node_root = doc.root().unwrap();
        let node_signature = node_root
            .search(&mut |n: &NodeRef| n.name().unwrap() == "Signature")
            .unwrap();
        let verified_nodes = node_root.verify().unwrap();

        assert!(verified_nodes.contains(node_root, None));
        assert!(!verified_nodes.contains(node_signature, None));
    }
}
