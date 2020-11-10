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

use std::fmt::{Debug, Display};

use thiserror::Error;

use super::{Item, Value};

pub struct EncodeStream<S> {
    storage: S,
    state: Vec<State>,
    extension: Vec<Vec<Item>>,
}

#[derive(Debug, Error)]
pub enum EncodeError<E>
where
    E: Debug + Display,
{
    #[error("Storage Error: {0}")]
    Data(E),

    #[error("Expected Root Element!")]
    ExpectedRootElement,

    #[error("Unexpected Root Element!")]
    UnexpectedRootElement,

    #[error("Unexpected Element!")]
    UnexpectedElement,

    #[error("Unexpected Array!")]
    UnexpectedArray,

    #[error("Unexpected Attrib!")]
    UnexpectedAttrib,

    #[error("Unexpected Resource!")]
    UnexpectedResource,

    #[error("Unexpected Field!")]
    UnexpectedField,

    #[error("Unexpected Extended Value!")]
    UnexpectedExtendedValue,

    #[error("Unexpected Value!")]
    UnexpectedValue,

    #[error("Unexpected End!")]
    UnexpectedEnd,
}

pub trait DataStorage {
    type Error: Display + Debug;

    fn put_item(&mut self, item: Item) -> Result<(), Self::Error>;
}

pub trait Optional {
    type Item;

    fn get(self) -> Option<Self::Item>;
}

#[derive(Debug)]
enum State {
    ExpectRoot,
    ExpectValue,
    ExpectAttrib,
    Element,
    Array,
    ResourceArray,
    ValueExtended { value: Value },
}

impl<S> EncodeStream<S>
where
    S: DataStorage,
{
    pub fn new(storage: S) -> Self {
        Self {
            storage,
            state: vec![State::ExpectRoot],
            extension: Vec::new(),
        }
    }

    pub fn root<N>(&mut self, name: N) -> Result<&mut Self, EncodeError<S::Error>>
    where
        N: Into<String>,
    {
        match self.state.pop() {
            Some(State::ExpectRoot) => {
                self.state.push(State::Element);

                self.add_item(Item::Root { name: name.into() })?;

                Ok(self)
            }
            Some(State::ExpectValue) => {
                self.state.push(State::Element);

                self.add_item(Item::Element)?;

                Ok(self)
            }
            Some(State::ResourceArray) => {
                self.state.push(State::ResourceArray);
                self.state.push(State::Element);

                self.add_item(Item::Root { name: name.into() })?;

                Ok(self)
            }
            None | Some(_) => Err(EncodeError::UnexpectedRootElement),
        }
    }

    pub fn element(&mut self) -> Result<&mut Self, EncodeError<S::Error>> {
        match self.state.pop() {
            Some(State::ExpectValue) => {
                self.state.push(State::Element);

                self.add_item(Item::Element)?;

                Ok(self)
            }
            Some(State::Array) => {
                self.state.push(State::Array);
                self.state.push(State::Element);

                self.add_item(Item::Element)?;

                Ok(self)
            }
            None | Some(_) => Err(EncodeError::UnexpectedElement),
        }
    }

    pub fn array(&mut self) -> Result<&mut Self, EncodeError<S::Error>> {
        match self.state.pop() {
            Some(State::ExpectRoot) => {
                self.state.push(State::ResourceArray);

                self.add_item(Item::Array)?;

                Ok(self)
            }
            Some(State::ExpectValue) => {
                self.state.push(State::Array);

                self.add_item(Item::Array)?;

                Ok(self)
            }
            None | Some(_) => Err(EncodeError::UnexpectedArray),
        }
    }

    pub fn end(&mut self) -> Result<&mut Self, EncodeError<S::Error>> {
        match self.state.pop() {
            Some(State::Element) | Some(State::Array) | Some(State::ResourceArray) => {
                self.add_item(Item::End)?;

                Ok(self)
            }
            Some(State::ValueExtended { value }) => {
                let extension = self
                    .extension
                    .pop()
                    .ok_or_else(|| EncodeError::UnexpectedEnd)?;

                self.add_item(Item::Value { value, extension })?;

                Ok(self)
            }
            None | Some(_) => Err(EncodeError::UnexpectedEnd),
        }
    }

    pub fn attrib_name<N>(&mut self, name: N) -> Result<&mut Self, EncodeError<S::Error>>
    where
        N: Into<String>,
    {
        match self.state.pop() {
            Some(State::Element) => {
                self.state.push(State::Element);
                self.state.push(State::ExpectAttrib);

                self.add_item(Item::Attrib { name: name.into() })?;

                Ok(self)
            }
            None | Some(_) => Err(EncodeError::UnexpectedAttrib),
        }
    }

    pub fn resource_name<N>(&mut self, name: N) -> Result<&mut Self, EncodeError<S::Error>>
    where
        N: Into<String>,
    {
        match self.state.pop() {
            Some(State::Element) => {
                self.state.push(State::Element);
                self.state.push(State::ExpectRoot);

                self.add_item(Item::Field { name: name.into() })?;

                Ok(self)
            }
            None | Some(_) => Err(EncodeError::UnexpectedResource),
        }
    }

    pub fn field_name<N>(&mut self, name: N) -> Result<&mut Self, EncodeError<S::Error>>
    where
        N: Into<String>,
    {
        match self.state.pop() {
            Some(State::Element) => {
                self.state.push(State::Element);
                self.state.push(State::ExpectValue);

                self.add_item(Item::Field { name: name.into() })?;

                Ok(self)
            }
            Some(State::ValueExtended { value }) => {
                self.state.push(State::ValueExtended { value });
                self.state.push(State::ExpectValue);

                self.add_item(Item::Field { name: name.into() })?;

                Ok(self)
            }
            None | Some(_) => Err(EncodeError::UnexpectedField),
        }
    }

    pub fn value<T>(&mut self, value: T) -> Result<&mut Self, EncodeError<S::Error>>
    where
        T: Into<Value>,
    {
        match self.state.pop() {
            Some(State::ExpectAttrib) | Some(State::ExpectValue) => {
                self.add_item(Item::Value {
                    value: value.into(),
                    extension: Vec::new(),
                })?;

                Ok(self)
            }
            Some(State::Array) => {
                self.state.push(State::Array);
                self.add_item(Item::Value {
                    value: value.into(),
                    extension: Vec::new(),
                })?;

                Ok(self)
            }
            None | Some(_) => Err(EncodeError::UnexpectedValue),
        }
    }

    pub fn value_extended<T>(&mut self, value: T) -> Result<&mut Self, EncodeError<S::Error>>
    where
        T: Into<Value>,
    {
        match self.state.pop() {
            Some(State::ExpectValue) => {
                self.state.push(State::ValueExtended {
                    value: value.into(),
                });
                self.extension.push(Vec::new());

                Ok(self)
            }
            Some(State::Array) => {
                self.state.push(State::Array);
                self.state.push(State::ValueExtended {
                    value: value.into(),
                });
                self.extension.push(Vec::new());

                Ok(self)
            }
            None | Some(_) => Err(EncodeError::UnexpectedValue),
        }
    }

    pub fn attrib<N, T, F>(
        &mut self,
        name: N,
        value: T,
        f: F,
    ) -> Result<&mut Self, EncodeError<S::Error>>
    where
        N: Into<String>,
        F: FnOnce(T, &mut Self) -> Result<(), EncodeError<S::Error>>,
    {
        self.attrib_name(name)?;

        f(value, self)?;

        Ok(self)
    }

    pub fn resource<N, T, F>(
        &mut self,
        name: N,
        value: T,
        f: F,
    ) -> Result<&mut Self, EncodeError<S::Error>>
    where
        N: Into<String>,
        F: FnOnce(T, &mut Self) -> Result<(), EncodeError<S::Error>>,
    {
        self.resource_name(name)?;

        f(value, self)?;

        Ok(self)
    }

    #[allow(dead_code)]
    pub fn resource_opt<N, T, F>(
        &mut self,
        name: N,
        value: T,
        f: F,
    ) -> Result<&mut Self, EncodeError<S::Error>>
    where
        N: Into<String>,
        T: Optional,
        F: FnOnce(T::Item, &mut Self) -> Result<(), EncodeError<S::Error>>,
    {
        match value.get() {
            Some(value) => self.encode(name, value, f),
            None => Ok(self),
        }
    }

    pub fn resource_vec<N, T, F>(
        &mut self,
        name: N,
        values: T,
        f: F,
    ) -> Result<&mut Self, EncodeError<S::Error>>
    where
        N: Into<String>,
        T: IntoIterator,
        F: Fn(T::Item, &mut Self) -> Result<(), EncodeError<S::Error>>,
    {
        self.resource_name(name)?;

        self.array()?;

        for value in values {
            f(value, self)?;
        }

        self.end()?;

        Ok(self)
    }

    pub fn encode<N, T, F>(
        &mut self,
        name: N,
        value: T,
        f: F,
    ) -> Result<&mut Self, EncodeError<S::Error>>
    where
        N: Into<String>,
        F: FnOnce(T, &mut Self) -> Result<(), EncodeError<S::Error>>,
    {
        self.field_name(name)?;

        f(value, self)?;

        Ok(self)
    }

    pub fn encode_opt<N, T, F>(
        &mut self,
        name: N,
        value: T,
        f: F,
    ) -> Result<&mut Self, EncodeError<S::Error>>
    where
        N: Into<String>,
        T: Optional,
        F: FnOnce(T::Item, &mut Self) -> Result<(), EncodeError<S::Error>>,
    {
        match value.get() {
            Some(value) => self.encode(name, value, f),
            None => Ok(self),
        }
    }

    pub fn encode_vec<N, T, F>(
        &mut self,
        name: N,
        values: T,
        f: F,
    ) -> Result<&mut Self, EncodeError<S::Error>>
    where
        N: Into<String> + Clone,
        T: IntoIterator,
        F: Fn(T::Item, &mut Self) -> Result<(), EncodeError<S::Error>>,
    {
        self.field_name(name)?;

        self.array()?;

        for value in values {
            f(value, self)?;
        }

        self.end()?;

        Ok(self)
    }

    pub fn inline<T, F>(&mut self, value: T, f: F) -> Result<&mut Self, EncodeError<S::Error>>
    where
        F: FnOnce(T, &mut Self) -> Result<(), EncodeError<S::Error>>,
    {
        f(value, self)?;

        Ok(self)
    }

    pub fn inline_opt<T, F>(&mut self, value: T, f: F) -> Result<&mut Self, EncodeError<S::Error>>
    where
        T: Optional,
        F: FnOnce(T::Item, &mut Self) -> Result<(), EncodeError<S::Error>>,
    {
        match value.get() {
            Some(value) => self.inline(value, f),
            None => Ok(self),
        }
    }

    fn add_item(&mut self, item: Item) -> Result<(), EncodeError<S::Error>> {
        match self.extension.last_mut() {
            Some(extension) => extension.push(item),
            None => self.storage.put_item(item).map_err(EncodeError::Data)?,
        }

        Ok(())
    }
}

impl<T> Optional for Option<T> {
    type Item = T;

    fn get(self) -> Option<Self::Item> {
        self
    }
}

impl<'a, T> Optional for &'a Option<T> {
    type Item = &'a T;

    fn get(self) -> Option<Self::Item> {
        self.as_ref()
    }
}

impl<'a, T> Optional for &'a mut Option<T> {
    type Item = &'a mut T;

    fn get(self) -> Option<Self::Item> {
        self.as_mut()
    }
}
