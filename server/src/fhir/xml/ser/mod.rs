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

mod maps;
mod sequences;
mod serializer;
mod structs;
mod values;

use std::io::Write;

use quick_xml::{DeError as Error, Error as XmlError};
use serde::Serialize;

pub use serializer::Serializer;

pub fn to_writer<W: Write, S: Serialize>(write: W, value: &S) -> Result<(), Error> {
    let mut serializer = Serializer::new(write);

    value.serialize(&mut serializer)?;

    Ok(())
}

pub fn to_string<S: Serialize>(value: &S) -> Result<String, Error> {
    let mut buf = Vec::new();

    to_writer(&mut buf, value)?;

    Ok(String::from_utf8(buf).map_err(|err| XmlError::Utf8(err.utf8_error()))?)
}

#[cfg(test)]
pub mod tests {
    use super::super::super::test::trim_xml_str;
    use super::*;

    use std::collections::BTreeMap;

    #[test]
    fn serialize_enum() {
        #[derive(Serialize)]
        enum MyEnum {
            Fuu,
            Bar,
        };

        #[derive(Serialize)]
        #[serde(rename_all = "PascalCase")]
        struct Test {
            #[serde(rename = "use")]
            pub my_enum: MyEnum,

            #[serde(rename = "value-tag=fuu")]
            pub fuu: MyEnum,
        }

        let test = Test {
            my_enum: MyEnum::Fuu,
            fuu: MyEnum::Bar,
        };

        let actual = trim_xml_str(&to_string(&test).unwrap());
        let expected = trim_xml_str(
            r##"
            <Test>
                <use>Fuu</use>
                <fuu value="Bar"/>
            </Test>
        "##,
        );

        assert_eq!(actual, expected);
    }

    #[test]
    fn serialize_struct() {
        #[derive(Serialize)]
        #[serde(rename = "value-tag=Inner")]
        struct Inner(String);

        #[derive(Serialize)]
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

        let test = Test {
            as_node: 123,
            as_attrib: 456,
            as_value_tag: 789,
            inner: Inner("some test string".to_owned()),
        };

        let actual = trim_xml_str(&to_string(&test).unwrap());
        let expected = trim_xml_str(
            r##"
            <Test TestAttrib="456">
                <AsNode>123</AsNode>
                <TestValueTag value="789"/>
                <RenamedInner value="some test string"/>
            </Test>
        "##,
        );

        assert_eq!(actual, expected);
    }

    #[test]
    fn serialize_struct_nested() {
        #[derive(Serialize)]
        #[serde(rename_all = "PascalCase")]
        struct Nested {
            #[serde(rename = "value-tag=Test")]
            pub test: u32,
        };

        #[derive(Serialize)]
        #[serde(rename_all = "PascalCase")]
        struct Test {
            pub nested: Nested,
        };

        let test = Test {
            nested: Nested { test: 123 },
        };

        let actual = trim_xml_str(&to_string(&test).unwrap());
        let expected = trim_xml_str(
            r##"
            <Test>
                <Nested>
                    <Test value="123"/>
                </Nested>
            </Test>
        "##,
        );

        assert_eq!(actual, expected);
    }

    #[test]
    fn serialize_sequence() {
        #[derive(Serialize)]
        #[serde(rename_all = "PascalCase")]
        struct Test {
            pub element: Vec<u32>,
        };

        let test = Test {
            element: vec![1, 2, 3],
        };

        let actual = trim_xml_str(&to_string(&test).unwrap());
        let expected = trim_xml_str(
            r##"
            <Test>
                <Element>1</Element>
                <Element>2</Element>
                <Element>3</Element>
            </Test>
        "##,
        );

        assert_eq!(actual, expected);
    }

    #[test]
    fn serialize_sequence_attribute() {
        #[derive(Serialize)]
        #[serde(rename_all = "PascalCase")]
        struct Foo {
            #[serde(rename = "attrib=value")]
            pub value: u32,
        };

        #[derive(Serialize)]
        #[serde(rename_all = "PascalCase")]
        struct Test {
            pub element: Vec<Foo>,
        };

        let test = Test {
            element: vec![Foo { value: 1 }, Foo { value: 2 }, Foo { value: 3 }],
        };

        let actual = trim_xml_str(&to_string(&test).unwrap());
        let expected = trim_xml_str(
            r##"
            <Test>
                <Element value="1"/>
                <Element value="2"/>
                <Element value="3"/>
            </Test>
        "##,
        );

        assert_eq!(actual, expected);
    }

    #[test]
    fn serialize_sequence_empty() {
        #[derive(Serialize)]
        #[serde(rename_all = "PascalCase")]
        struct Test {
            pub element: Vec<u32>,
        };

        let test = Test {
            element: Vec::default(),
        };

        let actual = trim_xml_str(&to_string(&test).unwrap());
        let expected = trim_xml_str(r##"<Test/>"##);

        assert_eq!(actual, expected);
    }

    #[test]
    fn serialize_option_some() {
        #[derive(Serialize)]
        #[serde(rename_all = "PascalCase")]
        struct Test {
            pub element: Option<u32>,
        };

        let test = Test { element: Some(123) };

        let actual = trim_xml_str(&to_string(&test).unwrap());
        let expected = trim_xml_str(
            r##"
            <Test>
                <Element>123</Element>
            </Test>
        "##,
        );

        assert_eq!(actual, expected);
    }

    #[test]
    fn serialize_option_none() {
        #[derive(Serialize)]
        #[serde(rename_all = "PascalCase")]
        struct Test {
            pub element: Option<u32>,
        };

        let test = Test { element: None };

        let actual = trim_xml_str(&to_string(&test).unwrap());
        let expected = trim_xml_str(r##"<Test/>"##);

        assert_eq!(actual, expected);
    }

    #[test]
    fn serialize_map() {
        #[derive(Serialize)]
        #[serde(rename_all = "PascalCase")]
        struct Test {
            pub element: BTreeMap<String, u32>,
        };

        let mut element = BTreeMap::default();
        element.insert("One".to_owned(), 1);
        element.insert("Two".to_owned(), 2);
        element.insert("Three".to_owned(), 3);

        let test = Test { element };

        let actual = trim_xml_str(&to_string(&test).unwrap());
        let expected = trim_xml_str(
            r##"
            <Test>
                <Element>
                    <One>1</One>
                    <Three>3</Three>
                    <Two>2</Two>
                </Element>
            </Test>
        "##,
        );

        assert_eq!(actual, expected);
    }

    #[test]
    fn serialize_flattened_attribs() {
        #[derive(Serialize)]
        #[serde(rename_all = "PascalCase")]
        struct Inner {
            fuu: u32,
        }

        #[derive(Serialize)]
        #[serde(rename_all = "PascalCase")]
        #[serde(rename = "xml:placeholder")]
        struct Test {
            #[serde(alias = "bar")]
            #[serde(rename = "attrib=bar")]
            bar: u32,

            #[serde(rename = "flatten-take-name")]
            inner: Inner,
        };

        let test = Test {
            bar: 123,
            inner: Inner { fuu: 456 },
        };

        let actual = trim_xml_str(&to_string(&test).unwrap());
        let expected = trim_xml_str(
            r##"
            <Inner bar="123">
                <Fuu>456</Fuu>
            </Inner>
        "##,
        );

        assert_eq!(actual, expected);
    }

    #[test]
    fn serialize_newtype_variant() {
        #[derive(Serialize)]
        #[serde(rename_all = "PascalCase")]
        struct Inner {
            fuu: u32,
        }

        #[allow(dead_code)]
        #[derive(Serialize)]
        #[serde(rename_all = "PascalCase")]
        enum Test {
            A(Inner),
            B(Inner),
        };

        let test = Test::A(Inner { fuu: 123 });

        let actual = trim_xml_str(&to_string(&test).unwrap());
        let expected = trim_xml_str(
            r##"
            <A>
                <Fuu>123</Fuu>
            </A>
        "##,
        );

        assert_eq!(actual, expected);
    }

    #[test]
    fn serialize_value_tag_sequence() {
        #[derive(Serialize)]
        #[serde(rename_all = "PascalCase")]
        struct Test {
            #[serde(alias = "Item")]
            #[serde(rename = "value-tag=Item")]
            item: Vec<String>,
        };

        let test = Test {
            item: vec!["fuu".into(), "bar".into()],
        };

        let actual = trim_xml_str(&to_string(&test).unwrap());
        let expected = trim_xml_str(
            r##"
            <Test>
                <Item value="fuu"/>
                <Item value="bar"/>
            </Test>
        "##,
        );

        assert_eq!(actual, expected);
    }
}
