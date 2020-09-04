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

mod deserializer;
mod enums;
mod maps;
mod sequences;
mod values;

use std::io::BufRead;

use quick_xml::DeError as Error;
use serde::de::DeserializeOwned;

pub use deserializer::Deserializer;

pub fn from_str<T: DeserializeOwned>(s: &str) -> Result<T, Error> {
    from_reader(s.as_bytes())
}

pub fn from_reader<R: BufRead, T: DeserializeOwned>(reader: R) -> Result<T, Error> {
    let mut de = Deserializer::from_reader(reader);

    T::deserialize(&mut de)
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use std::collections::BTreeMap;

    use serde::Deserialize;

    #[test]
    fn deserialize_enum() {
        #[derive(Deserialize, Debug, PartialEq)]
        enum MyEnum {
            Fuu,
            Bar,
        };

        #[serde(rename_all = "PascalCase")]
        #[derive(Deserialize, Debug, PartialEq)]
        pub struct Test {
            #[serde(rename = "use")]
            my_enum: MyEnum,

            #[serde(rename = "value-tag=fuu")]
            fuu: MyEnum,
        }

        const TEST_XML: &str = r##"
            <Test>
                <use>Fuu</use>
                <fuu value="Bar"/>
            </Test>
        "##;

        let expected = Test {
            my_enum: MyEnum::Fuu,
            fuu: MyEnum::Bar,
        };
        let actual: Test = from_str(TEST_XML).unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn deserialize_value_tag() {
        #[derive(Debug, PartialEq, Deserialize)]
        #[serde(rename = "value-tag=Test")]
        struct Test(String);

        const TEST_XML: &str = r##"
            <Test value="some test string"/>
        "##;

        let expected = Test("some test string".to_owned());
        let actual: Test = from_str(TEST_XML).unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn deserialize_struct_simple() {
        #[derive(Debug, PartialEq, Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct Test {
            #[serde(rename = "attrib=TestAttrib")]
            pub as_attrib: u32,

            pub as_node: u32,

            #[serde(rename = "value-tag=TestValueTag")]
            pub as_value_tag: u32,
        };

        const TEST_XML: &str = r##"
            <Test TestAttrib="456">
                <TestValueTag value="789"/>
                <AsNode>123</AsNode>
                <UnknownTag value="some test string"/>
            </Test>
        "##;

        let expected = Test {
            as_node: 123,
            as_attrib: 456,
            as_value_tag: 789,
        };
        let actual: Test = from_str(TEST_XML).unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn deserialize_struct_inner() {
        #[derive(Debug, PartialEq, Deserialize)]
        struct Inner {
            #[serde(rename = "attrib=fuu")]
            value: String,
        };

        #[derive(Debug, PartialEq, Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct Test {
            #[serde(rename = "attrib=TestAttrib")]
            pub as_attrib: u32,

            pub as_node: u32,

            #[serde(rename = "value-tag=TestValueTag")]
            pub as_value_tag: u32,

            #[serde(rename = "RenamedInner")]
            pub inner: Inner,
        };

        const TEST_XML: &str = r##"
            <Test TestAttrib="456">
                <RenamedInner fuu="some test string"/>
                <TestValueTag value="789"/>
                <AsNode>123</AsNode>
            </Test>
        "##;

        let expected = Test {
            as_node: 123,
            as_attrib: 456,
            as_value_tag: 789,
            inner: Inner {
                value: "some test string".to_owned(),
            },
        };
        let actual: Test = from_str(TEST_XML).unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn deserialize_struct_value_tag() {
        #[derive(Debug, PartialEq, Deserialize)]
        #[serde(rename = "value-tag=Inner")]
        struct Inner(String);

        #[derive(Debug, PartialEq, Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct Test {
            #[serde(rename = "attrib=TestAttrib")]
            pub as_attrib: u32,

            pub as_node: u32,

            #[serde(rename = "value-tag=TestValueTag")]
            pub as_value_tag: u32,

            #[serde(rename = "RenamedInner")]
            pub inner: Inner,
        };

        const TEST_XML: &str = r##"
            <Test TestAttrib="456">
                <RenamedInner value="some test string"/>
                <TestValueTag value="789"/>
                <AsNode>123</AsNode>
            </Test>
        "##;

        let expected = Test {
            as_node: 123,
            as_attrib: 456,
            as_value_tag: 789,
            inner: Inner("some test string".to_owned()),
        };
        let actual: Test = from_str(TEST_XML).unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn deserialize_struct_nested() {
        #[derive(Deserialize, Debug, PartialEq)]
        #[serde(rename_all = "PascalCase")]
        struct Nested {
            #[serde(rename = "value-tag=Test")]
            pub test: u32,
        };

        #[derive(Deserialize, Debug, PartialEq)]
        #[serde(rename_all = "PascalCase")]
        struct Test {
            pub nested: Nested,
        };

        const TEST_XML: &str = r##"
            <Test>
                <Nested>
                    <Test value="123"/>
                </Nested>
            </Test>
        "##;

        let expected = Test {
            nested: Nested { test: 123 },
        };
        let actual: Test = from_str(TEST_XML).unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn deserialize_sequence() {
        #[derive(Deserialize, Debug, PartialEq)]
        #[serde(rename_all = "PascalCase")]
        struct Test {
            pub element: Vec<u32>,
        };

        const TEST_XML: &str = r##"
            <Test>
                <Element>1</Element>
                <Element>2</Element>
                <Element>3</Element>
            </Test>
        "##;

        let expected = Test {
            element: vec![1, 2, 3],
        };
        let actual: Test = from_str(TEST_XML).unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn deserialize_sequence_struct() {
        #[derive(Deserialize, Debug, PartialEq)]
        #[serde(rename_all = "PascalCase")]
        struct Foo {
            #[serde(rename = "attrib=fuu")]
            pub value: u32,

            pub bar: u32,
        };

        #[derive(Deserialize, Debug, PartialEq)]
        #[serde(rename_all = "PascalCase")]
        struct Test {
            pub test: u32,
            pub element: Vec<Foo>,
        };

        const TEST_XML: &str = r##"
            <Test>
                <Test>123</Test>
                <Element fuu="1">
                    <Bar>101</Bar>
                </Element>
                <Element fuu="2">
                    <Bar>102</Bar>
                </Element>
                <Element fuu="3">
                    <Bar>103</Bar>
                </Element>
            </Test>
        "##;

        let expected = Test {
            test: 123,
            element: vec![
                Foo { value: 1, bar: 101 },
                Foo { value: 2, bar: 102 },
                Foo { value: 3, bar: 103 },
            ],
        };
        let actual: Test = from_str(TEST_XML).unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn deserialize_sequence_value_tag() {
        #[derive(Deserialize, Debug, PartialEq)]
        #[serde(rename = "value-tag=Foo")]
        struct Foo(u32);

        #[derive(Deserialize, Debug, PartialEq)]
        #[serde(rename_all = "PascalCase")]
        struct Test {
            pub element: Vec<Foo>,
        };

        const TEST_XML: &str = r##"
            <Test>
                <Element value="1"/>
                <Element value="2"/>
                <Element value="3"/>
            </Test>
        "##;

        let expected = Test {
            element: vec![Foo(1), Foo(2), Foo(3)],
        };
        let actual: Test = from_str(TEST_XML).unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn deserialize_sequence_empty() {
        #[derive(Deserialize, Debug, PartialEq)]
        #[serde(rename_all = "PascalCase")]
        struct Test {
            #[serde(default)]
            pub element: Vec<u32>,
        };

        const TEST_XML: &str = r##"
            <Test></Test>
        "##;

        let expected = Test { element: vec![] };
        let actual: Test = from_str(TEST_XML).unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn deserialize_option_some() {
        #[derive(Deserialize, Debug, PartialEq)]
        #[serde(rename_all = "PascalCase")]
        struct Test {
            pub element: Option<u32>,
        };

        const TEST_XML: &str = r##"
            <Test>
                <Element>123</Element>
            </Test>
        "##;

        let expected = Test { element: Some(123) };
        let actual: Test = from_str(TEST_XML).unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn deserialize_option_none() {
        #[derive(Deserialize, Debug, PartialEq)]
        #[serde(rename_all = "PascalCase")]
        struct Test {
            pub element: Option<u32>,
        };

        const TEST_XML: &str = r##"
            <Test></Test>
        "##;

        let expected = Test { element: None };
        let actual: Test = from_str(TEST_XML).unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn deserialize_map() {
        #[derive(Deserialize, Debug, PartialEq)]
        #[serde(rename_all = "PascalCase")]
        struct Test {
            pub element: BTreeMap<String, u32>,
        };

        const TEST_XML: &str = r##"
            <Test>
                <Element>
                    <One>1</One>
                    <Three>3</Three>
                    <Two>2</Two>
                </Element>
            </Test>
        "##;

        let mut element = BTreeMap::default();
        element.insert("One".to_owned(), 1);
        element.insert("Two".to_owned(), 2);
        element.insert("Three".to_owned(), 3);

        let expected = Test { element };
        let actual: Test = from_str(TEST_XML).unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn deserialize_flattened_attribs() {
        #[derive(Debug, PartialEq, Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct Inner {
            fuu: u32,
        }

        #[derive(Debug, PartialEq, Deserialize)]
        #[serde(rename_all = "PascalCase")]
        #[serde(rename = "xml:placeholder")]
        struct Test {
            #[serde(alias = "bar")]
            #[serde(rename = "attrib=bar")]
            bar: u32,

            #[serde(rename = "flatten-take-name")]
            inner: Inner,
        };

        const TEST_XML: &str = r##"
            <Inner bar="123">
                <Fuu>456</Fuu>
            </Inner>
        "##;

        let expected = Test {
            bar: 123,
            inner: Inner { fuu: 456 },
        };
        let actual: Test = from_str(TEST_XML).unwrap();

        assert_eq!(actual, expected);
    }
}
