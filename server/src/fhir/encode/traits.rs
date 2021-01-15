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

use super::{DataStorage, EncodeError, EncodeStream, Value};

pub fn encode_any<T, S>(value: T, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
where
    T: Encode,
    S: DataStorage,
{
    value.encode(stream)
}

pub trait Encode: Sized {
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage;
}

impl<T> Encode for T
where
    T: Into<Value>,
{
    fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
    where
        S: DataStorage,
    {
        stream.value(self)?;

        Ok(())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    use async_trait::async_trait;
    use futures::stream::StreamExt;

    use super::super::{EncodeError, EncodeStream, Item, ItemStream};

    #[derive(Debug, PartialEq)]
    struct Element {
        fuu: String,
        bar: Option<usize>,
    }

    #[async_trait(?Send)]
    impl Encode for Element {
        fn encode<S>(self, stream: &mut EncodeStream<S>) -> Result<(), EncodeError<S::Error>>
        where
            S: DataStorage,
        {
            stream
                .root("Element")?
                .encode("fuu", &self.fuu, encode_any)?
                .encode_opt("bar", &self.bar, encode_any)?
                .end()?;

            Ok(())
        }
    }

    #[tokio::test]
    async fn encode_element() {
        let element = Element {
            fuu: "value0".into(),
            bar: None,
        };

        let mut stream = ItemStream::default();
        let mut encode_stream = EncodeStream::new(&mut stream);
        element.encode(&mut encode_stream).unwrap();

        macro_rules! assert_item {
            ($item:expr) => {
                assert_eq!($item, stream.next().await.unwrap())
            };
        }

        assert_item!(Item::Root {
            name: "Element".into(),
        });
        assert_item!(Item::Field { name: "fuu".into() });
        assert_item!(Item::Value {
            value: Value::String("value0".into()),
            extension: Vec::new(),
        });
        assert_item!(Item::End);
        assert_eq!(true, stream.next().await.is_none());
    }
}
