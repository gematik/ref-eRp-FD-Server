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

use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::mem::take;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::vec::IntoIter;

use futures::{
    future::Future,
    ready,
    stream::{Stream, StreamExt, TryStreamExt},
};
use miscellaneous::str::icase_eq;
use thiserror::Error;

use crate::fhir::{Format, WithFormat};

use super::Item;

pub struct DecodeStream<S>
where
    S: DataStream,
{
    stream: S,
    expect_root: bool,
    state: Vec<State>,
    barriers: Vec<Barrier>,
    buffer: Option<Result<Item, S::Error>>,
    extensions: Vec<IntoIter<Item>>,
}

#[allow(dead_code)]
#[derive(Error, Debug)]
pub enum DecodeError<E>
where
    E: Debug + Display,
{
    #[error("Data Error: {0}")]
    Data(E),

    #[error("Invalid Profile (actual={actual:?}, expected={expected:?})!")]
    InvalidProfile {
        actual: Vec<String>,
        expected: Vec<String>,
    },

    #[error("Invalid Value (value={value}, path={path})!")]
    InvalidValue { value: String, path: OptStr },

    #[error("Invalid Value (actual={actual}, expected={expected}, path={path})!")]
    InvalidFixedValue {
        actual: OptStr,
        expected: OptStr,
        path: OptStr,
    },

    #[error("Element Out Of Order (id={id}, path{path})!")]
    ElementOutOfOrder { id: OptStr, path: OptStr },

    #[error("Missing Field (id={id}, path={path})!")]
    MissingField { id: OptStr, path: OptStr },

    #[error("Missing Extension (url={url}, path={path})!")]
    MissingExtension { url: OptStr, path: OptStr },

    #[error("Unexpected Element (id={id}, path={path})!")]
    UnexpectedElement { id: OptStr, path: OptStr },

    #[error("Unexpected End (path={path})!")]
    UnexpectedEnd { path: OptStr },

    #[error("Unexpected Field (id={id}, path={path})!")]
    UnexpectedField { id: OptStr, path: OptStr },

    #[error("Unexpected Field (value={value}, path={path})!")]
    UnexpectedValue { value: OptStr, path: OptStr },

    #[error("Unexpected Vector (path={path})!")]
    UnexpectedElements { path: OptStr },

    #[error("Unexpected End Of Barrier (name={name}, path={path})!")]
    UnexpectedEoB { name: OptStr, path: OptStr },

    #[error("Unexpected End Of Element (path={path})!")]
    UnexpectedEoE { path: OptStr },

    #[error("Unexpected End Of File (path={path})!")]
    UnexpectedEoF { path: OptStr },

    #[error("{message} (path={path})!")]
    Custom { message: String, path: OptStr },
}

impl<E> DecodeError<E>
where
    E: Debug + Display,
{
    pub fn path(&self) -> Option<&String> {
        match self {
            Self::InvalidValue { path, .. } => path.0.as_ref(),
            Self::InvalidFixedValue { path, .. } => path.0.as_ref(),
            Self::ElementOutOfOrder { path, .. } => path.0.as_ref(),
            Self::MissingField { path, .. } => path.0.as_ref(),
            Self::MissingExtension { path, .. } => path.0.as_ref(),
            Self::UnexpectedElement { path, .. } => path.0.as_ref(),
            Self::UnexpectedEnd { path, .. } => path.0.as_ref(),
            Self::UnexpectedField { path, .. } => path.0.as_ref(),
            Self::UnexpectedValue { path, .. } => path.0.as_ref(),
            Self::UnexpectedElements { path, .. } => path.0.as_ref(),
            Self::UnexpectedEoB { path, .. } => path.0.as_ref(),
            Self::UnexpectedEoE { path, .. } => path.0.as_ref(),
            Self::UnexpectedEoF { path, .. } => path.0.as_ref(),
            Self::Custom { path, .. } => path.0.as_ref(),
            _ => None,
        }
    }
}

pub trait DataStream: Unpin {
    type Stream: Stream<Item = Result<Item, Self::Error>> + Unpin;
    type Error: Debug + Display + Unpin;

    fn as_stream(&mut self) -> &mut Self::Stream;

    fn format(&self) -> Option<Format>;
}

#[derive(Debug, Default)]
pub struct OptStr(Option<String>);

pub trait Optional {
    type Item;

    fn some(item: Self::Item) -> Self;
    fn none() -> Self;
}

pub trait Vector: Default {
    type Item;

    fn push(&mut self, item: Self::Item);
}

#[derive(Debug, Clone)]
pub enum Fields {
    Many {
        index: usize,
        names: &'static [&'static str],
    },
    Any,
}

pub enum Search<'a> {
    Exact(&'a str),
    Any,
}

pub trait AsyncFnOnce<A0> {
    type Output;
    type Future: Future<Output = Self::Output>;

    fn call_once(self, a0: A0) -> Self::Future;
}

pub trait AsyncFnMut<A0>: AsyncFnOnce<A0> {
    fn call_mut(&mut self, a0: A0) -> Self::Future;
}

pub trait AsyncFn<A0>: AsyncFnMut<A0> {
    fn call(&self, a0: A0) -> Self::Future;
}

#[derive(Debug)]
enum State {
    Substream { name: Option<&'static str> },
    Root { name: Option<&'static str> },
    Element,
    ExtendedValue,
}

#[derive(Debug)]
struct Barrier {
    depth: usize,
    name: Option<&'static str>,
}

enum FieldMatch {
    OutOfOrder,
    Match,
    Skip,
    Finished,
}

impl<S> DecodeStream<S>
where
    S: DataStream,
{
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            expect_root: true,
            state: Vec::new(),
            barriers: Vec::new(),
            buffer: None,
            extensions: Vec::new(),
        }
    }

    pub fn format(&self) -> Option<Format> {
        self.stream.format()
    }

    pub fn path(&self) -> Option<String> {
        if self.state.is_empty() {
            return None;
        }

        let mut path = String::default();

        for state in &self.state {
            match state {
                State::Substream { name: Some(name) } => path = format!("{}/{}", path, name),
                State::Root { name: Some(name) } => path = format!("{}/{}", path, name),
                _ => (),
            }
        }

        Some(path)
    }

    pub async fn root(&mut self, name: &'static str) -> Result<(), DecodeError<S::Error>> {
        if take(&mut self.expect_root) {
            self.state.push(State::Root { name: Some(name) });
            self.expect_element(Search::Exact(name), false).await?;
        } else {
            self.state.push(State::Root { name: None });
            self.expect_element(Search::Any, false).await?;
        }

        Ok(())
    }

    pub async fn element(&mut self) -> Result<String, DecodeError<S::Error>> {
        self.state.push(State::Element);

        let name = self.expect_element(Search::Any, false).await?.unwrap();

        Ok(name)
    }

    pub async fn peek_element(&mut self) -> Result<String, DecodeError<S::Error>> {
        Ok(self.expect_element(Search::Any, true).await?.unwrap())
    }

    pub async fn end(&mut self) -> Result<(), DecodeError<S::Error>> {
        match self.state.pop() {
            Some(State::ExtendedValue { .. }) => {
                match self.barriers.pop() {
                    Some(barrier) if barrier.depth != 0 => {
                        return Err(DecodeError::UnexpectedEoB {
                            name: barrier.name.into(),
                            path: self.path().into(),
                        })
                    }
                    None => {
                        return Err(DecodeError::UnexpectedEoB {
                            name: Default::default(),
                            path: self.path().into(),
                        })
                    }
                    _ => (),
                }

                self.buffer = None;
                self.extensions.pop();

                Ok(())
            }
            Some(_) => {
                self.skip(1).await?;

                Ok(())
            }
            None => Err(DecodeError::UnexpectedEnd {
                path: self.path().into(),
            }),
        }
    }

    pub async fn resource<T, F>(
        &mut self,
        fields: &mut Fields,
        f: F,
    ) -> Result<T, DecodeError<S::Error>>
    where
        F: for<'a> AsyncFnOnce<&'a mut Self, Output = Result<T, DecodeError<S::Error>>>,
    {
        let fields = fields.next();

        self.substream_inner(&fields, false).await?;
        self.element().await?;

        self.expect_root = true;

        let item = f.call_once(self).await?;

        self.end().await?;
        self.end_substream().await?;

        Ok(item)
    }

    #[allow(dead_code)]
    pub async fn resource_opt<T, F>(
        &mut self,
        fields: &mut Fields,
        f: F,
    ) -> Result<T, DecodeError<S::Error>>
    where
        T: Optional,
        F: for<'a> AsyncFnOnce<&'a mut Self, Output = Result<T::Item, DecodeError<S::Error>>>,
    {
        let fields = fields.next();

        if self.substream_inner(&fields, true).await? {
            self.element().await?;

            self.expect_root = true;

            let item = f.call_once(self).await?;

            self.end().await?;
            self.end_substream().await?;

            Ok(T::some(item))
        } else {
            Ok(T::none())
        }
    }

    pub async fn resource_vec<T, F>(
        &mut self,
        fields: &mut Fields,
        f: F,
    ) -> Result<T, DecodeError<S::Error>>
    where
        T: Vector,
        F: for<'a> AsyncFn<&'a mut Self, Output = Result<T::Item, DecodeError<S::Error>>>,
    {
        let fields = fields.next();
        let mut ret = T::default();

        while self.substream_inner(&fields, true).await? {
            self.element().await?;

            self.expect_root = true;

            let item = f.call(self).await?;

            ret.push(item);

            self.end().await?;
            self.end_substream().await?;
        }

        Ok(ret)
    }

    pub async fn decode<T, F>(
        &mut self,
        fields: &mut Fields,
        f: F,
    ) -> Result<T, DecodeError<S::Error>>
    where
        F: for<'a> AsyncFnOnce<&'a mut Self, Output = Result<T, DecodeError<S::Error>>>,
    {
        let fields = fields.next();

        self.substream_inner(&fields, false).await?;

        let item = f.call_once(self).await?;

        self.end_substream().await?;

        Ok(item)
    }

    pub async fn decode_opt<T, F>(
        &mut self,
        fields: &mut Fields,
        f: F,
    ) -> Result<T, DecodeError<S::Error>>
    where
        T: Optional,
        F: for<'a> AsyncFnOnce<&'a mut Self, Output = Result<T::Item, DecodeError<S::Error>>>,
    {
        let fields = fields.next();

        if self.substream_inner(&fields, true).await? {
            let item = f.call_once(self).await?;

            self.end_substream().await?;

            Ok(T::some(item))
        } else {
            Ok(T::none())
        }
    }

    pub async fn decode_vec<T, F>(
        &mut self,
        fields: &mut Fields,
        f: F,
    ) -> Result<T, DecodeError<S::Error>>
    where
        T: Vector,
        F: for<'a> AsyncFn<&'a mut Self, Output = Result<T::Item, DecodeError<S::Error>>>,
    {
        let fields = fields.next();
        let mut ret = T::default();

        while self.substream_inner(&fields, true).await? {
            let item = f.call(self).await?;

            ret.push(item);

            self.end_substream().await?;
        }

        Ok(ret)
    }

    pub async fn fixed(
        &mut self,
        fields: &mut Fields,
        expected: &str,
    ) -> Result<(), DecodeError<S::Error>> {
        let fields = fields.next();

        self.substream_inner(&fields, false).await?;

        let actual = self.value(Search::Any).await?.unwrap();
        if actual != expected {
            return Err(DecodeError::InvalidFixedValue {
                actual: actual.into(),
                expected: expected.into(),
                path: self.path().into(),
            });
        }

        self.end_substream().await?;

        Ok(())
    }

    pub async fn fixed_opt(
        &mut self,
        fields: &mut Fields,
        expected: Option<&str>,
    ) -> Result<(), DecodeError<S::Error>> {
        if let Some(expected) = expected {
            self.fixed(fields, expected).await?;
        } else {
            fields.next();
        }

        Ok(())
    }

    pub async fn ifixed(
        &mut self,
        fields: &mut Fields,
        expected: &str,
    ) -> Result<(), DecodeError<S::Error>> {
        let fields = fields.next();

        self.substream_inner(&fields, false).await?;

        let actual = self.value(Search::Any).await?.unwrap();
        if !icase_eq(&actual, expected) {
            return Err(DecodeError::InvalidFixedValue {
                actual: actual.into(),
                expected: expected.into(),
                path: self.path().into(),
            });
        }

        self.end_substream().await?;

        Ok(())
    }

    pub async fn ifixed_opt(
        &mut self,
        fields: &mut Fields,
        expected: Option<&str>,
    ) -> Result<(), DecodeError<S::Error>> {
        if let Some(expected) = expected {
            self.ifixed(fields, expected).await?;
        } else {
            fields.next();
        }

        Ok(())
    }

    pub async fn value(&mut self, s: Search<'_>) -> Result<Option<String>, DecodeError<S::Error>> {
        match self.next().await {
            Some(Ok(Item::Field { name, value, .. })) => match s {
                Search::Any => Ok(Some(value)),
                Search::Exact(s) if name == s => Ok(Some(value)),
                Search::Exact(_) => Err(DecodeError::UnexpectedField {
                    id: name.into(),
                    path: self.path().into(),
                }),
            },
            Some(Ok(Item::BeginElement { name })) => Err(DecodeError::UnexpectedElement {
                id: name.into(),
                path: self.path().into(),
            }),
            Some(Ok(Item::EndElement)) => Err(DecodeError::UnexpectedEoE {
                path: self.path().into(),
            }),
            Some(Err(err)) => Err(DecodeError::Data(err)),
            None => Err(DecodeError::UnexpectedEoF {
                path: self.path().into(),
            }),
        }
    }

    pub async fn value_extended(&mut self) -> Result<String, DecodeError<S::Error>> {
        match self.next().await {
            Some(Ok(Item::Field {
                value, extension, ..
            })) => {
                self.state.push(State::ExtendedValue);
                self.barriers.push(Barrier::new(None));
                self.extensions.push(extension.into_iter());

                Ok(value)
            }
            Some(Ok(Item::BeginElement { name })) => Err(DecodeError::UnexpectedElement {
                id: name.into(),
                path: self.path().into(),
            }),
            Some(Ok(Item::EndElement)) => Err(DecodeError::UnexpectedEoE {
                path: self.path().into(),
            }),
            Some(Err(err)) => Err(DecodeError::Data(err)),
            None => Err(DecodeError::UnexpectedEoF {
                path: self.path().into(),
            }),
        }
    }

    pub async fn begin_substream(
        &mut self,
        fields: &mut Fields,
    ) -> Result<(), DecodeError<S::Error>> {
        let fields = fields.next();

        self.substream_inner(&fields, false).await?;

        Ok(())
    }

    pub async fn begin_substream_opt(
        &mut self,
        fields: &mut Fields,
    ) -> Result<bool, DecodeError<S::Error>> {
        let fields = fields.next();

        self.substream_inner(&fields, true).await
    }

    pub async fn begin_substream_vec(
        &mut self,
        fields: &mut Fields,
    ) -> Result<bool, DecodeError<S::Error>> {
        if self.substream_inner(&fields, true).await? {
            return Ok(true);
        }

        fields.next();

        Ok(false)
    }

    pub async fn end_substream(&mut self) -> Result<(), DecodeError<S::Error>> {
        match self.barriers.last() {
            Some(barrier) if barrier.depth != 0 => {
                return Err(DecodeError::UnexpectedEoB {
                    name: barrier.name.into(),
                    path: self.path().into(),
                })
            }
            None => {
                return Err(DecodeError::UnexpectedEoB {
                    name: Default::default(),
                    path: self.path().into(),
                })
            }
            _ => (),
        }

        let barrier = self.barriers.pop().unwrap();

        match self.state.pop() {
            Some(State::Substream { .. }) => Ok(()),
            _ => Err(DecodeError::UnexpectedEoB {
                name: barrier.name.into(),
                path: self.path().into(),
            }),
        }
    }

    async fn substream_inner(
        &mut self,
        fields: &Fields,
        is_optional: bool,
    ) -> Result<bool, DecodeError<S::Error>> {
        let field = fields.name();

        loop {
            let item = match self.next().await {
                Some(Ok(item)) => item,
                Some(Err(err)) => return Err(DecodeError::Data(err)),
                None if is_optional => return Ok(false),
                None => {
                    return Err(DecodeError::MissingField {
                        id: field.into(),
                        path: self.path().into(),
                    })
                }
            };

            let name = match &item {
                Item::BeginElement { name } => name,
                Item::Field { name, .. } => name,
                Item::EndElement => {
                    self.put_back(Ok(Item::EndElement), true);

                    if !is_optional {
                        return Err(DecodeError::MissingField {
                            id: field.into(),
                            path: self.path().into(),
                        });
                    }

                    return Ok(false);
                }
            };

            match fields.cmp(&name) {
                FieldMatch::OutOfOrder => {
                    return Err(DecodeError::ElementOutOfOrder {
                        id: field.into(),
                        path: self.path().into(),
                    });
                }
                FieldMatch::Finished => {
                    self.put_back(Ok(item), true);

                    if !is_optional {
                        return Err(DecodeError::MissingField {
                            id: field.into(),
                            path: self.path().into(),
                        });
                    }

                    return Ok(false);
                }
                FieldMatch::Match => {
                    self.put_back(Ok(item), true);

                    let name = fields.name();

                    self.barriers.push(Barrier::new(field));
                    self.state.push(State::Substream { name });

                    return Ok(true);
                }
                FieldMatch::Skip => {
                    self.put_back(Ok(item), true);
                    self.skip(0).await?;
                }
            }
        }
    }

    async fn expect_element(
        &mut self,
        s: Search<'_>,
        peek: bool,
    ) -> Result<Option<String>, DecodeError<S::Error>> {
        match self.next().await {
            Some(Ok(Item::BeginElement { name })) => {
                let ret = match s {
                    Search::Any => Ok(Some(name)),
                    Search::Exact(s) if name == s => Ok(Some(name)),
                    Search::Exact(_) => Err(DecodeError::UnexpectedElement {
                        id: name.into(),
                        path: self.path().into(),
                    }),
                };

                if peek {
                    if let Ok(Some(name)) = &ret {
                        self.put_back(Ok(Item::BeginElement { name: name.clone() }), true);
                    }
                }

                ret
            }
            Some(Ok(Item::Field { name, .. })) => Err(DecodeError::UnexpectedField {
                id: name.into(),
                path: self.path().into(),
            }),
            Some(Ok(Item::EndElement)) => Err(DecodeError::UnexpectedEoE {
                path: self.path().into(),
            }),
            Some(Err(err)) => Err(DecodeError::Data(err)),
            None => Err(DecodeError::UnexpectedEoF {
                path: self.path().into(),
            }),
        }
    }

    async fn skip(&mut self, mut depth: usize) -> Result<(), DecodeError<S::Error>> {
        while let Some(item) = self.try_next().await.map_err(DecodeError::Data)? {
            match item {
                Item::Field { .. } if depth == 0 => return Ok(()),
                Item::Field { .. } => (),
                Item::BeginElement { .. } => {
                    depth += 1;
                }
                Item::EndElement => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(());
                    }
                }
            }
        }

        Err(DecodeError::UnexpectedEoF {
            path: self.path().into(),
        })
    }

    fn put_back(&mut self, item: Result<Item, S::Error>, update_barrier: bool) {
        if self.buffer.is_some() {
            panic!("Unable to put back item, buffer is already occupied!");
        }

        if update_barrier {
            if let Some(barrier) = self.barriers.last_mut() {
                barrier.put_back(&item);
            }
        }

        self.buffer = Some(item);
    }
}

impl<S> Stream for DecodeStream<S>
where
    S: DataStream,
{
    type Item = Result<Item, S::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let item = if let Some(item) = self.buffer.take() {
            item
        } else if let Some(mut extension) = self.extensions.pop() {
            let item = extension.next();

            self.extensions.push(extension);

            match item {
                Some(item) => Ok(item),
                None => return Poll::Ready(None),
            }
        } else {
            match ready!(self.stream.as_stream().poll_next_unpin(cx)) {
                Some(item) => item,
                None => return Poll::Ready(None),
            }
        };

        let is_valid = self
            .barriers
            .last_mut()
            .map(|b| b.next(&item))
            .unwrap_or(true);

        if is_valid {
            Poll::Ready(Some(item))
        } else {
            self.put_back(item, false);

            Poll::Ready(None)
        }
    }
}

impl<S, E> DataStream for S
where
    S: Stream<Item = Result<Item, E>> + WithFormat + Unpin,
    E: Debug + Display + Unpin,
{
    type Stream = S;
    type Error = E;

    fn as_stream(&mut self) -> &mut Self::Stream {
        self
    }

    fn format(&self) -> Option<Format> {
        WithFormat::format(self)
    }
}

impl Display for OptStr {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match &self.0 {
            Some(v) => write!(f, "{}", v),
            None => write!(f, "-"),
        }
    }
}

impl From<&str> for OptStr {
    fn from(v: &str) -> Self {
        Self(Some(v.into()))
    }
}

impl From<String> for OptStr {
    fn from(v: String) -> Self {
        Self(Some(v))
    }
}

impl From<Option<&str>> for OptStr {
    fn from(v: Option<&str>) -> Self {
        Self(v.map(Into::into))
    }
}

impl From<Option<String>> for OptStr {
    fn from(v: Option<String>) -> Self {
        Self(v)
    }
}

impl<T> Optional for Option<T> {
    type Item = T;

    fn some(item: Self::Item) -> Self {
        Some(item)
    }

    fn none() -> Self {
        None
    }
}

impl<T> Vector for Vec<T> {
    type Item = T;

    fn push(&mut self, item: Self::Item) {
        self.push(item);
    }
}

impl Fields {
    pub fn new(names: &'static [&'static str]) -> Self {
        Self::Many { names, index: 0 }
    }

    fn name(&self) -> Option<&'static str> {
        match self {
            Self::Many { names, index } => Some(&names[*index]),
            Self::Any => None,
        }
    }

    fn next(&mut self) -> Self {
        let ret = self.clone();

        match self {
            Self::Many { index, .. } => *index += 1,
            Self::Any => (),
        }

        ret
    }

    fn cmp(&self, s: &str) -> FieldMatch {
        match self {
            Self::Many { names, index } => match names.iter().position(|name| *name == s) {
                Some(pos) if pos < *index => FieldMatch::OutOfOrder,
                Some(pos) if pos > *index => FieldMatch::Finished,
                Some(_) => FieldMatch::Match,
                None => FieldMatch::Skip,
            },
            Self::Any => FieldMatch::Match,
        }
    }
}

impl Barrier {
    fn new(name: Option<&'static str>) -> Self {
        Self { depth: 0, name }
    }

    fn next<E>(&mut self, item: &Result<Item, E>) -> bool {
        if self.depth == 0 {
            match (item, &self.name) {
                (Ok(Item::BeginElement { name }), Some(expected)) if name == expected => {
                    self.depth += 1;

                    true
                }
                (Ok(Item::BeginElement { .. }), None) => {
                    self.depth += 1;

                    true
                }
                (Ok(Item::Field { name, .. }), Some(expected)) if name == expected => true,
                (Ok(Item::Field { .. }), None) => true,
                (Ok(_), _) => false,
                (Err(_), _) => true,
            }
        } else {
            match item {
                Ok(Item::BeginElement { .. }) => self.depth += 1,
                Ok(Item::EndElement { .. }) => self.depth -= 1,
                _ => (),
            }

            true
        }
    }

    fn put_back<E>(&mut self, item: &Result<Item, E>) {
        match item {
            Ok(Item::BeginElement { .. }) => self.depth -= 1,
            Ok(Item::EndElement) => self.depth += 1,
            _ => (),
        }
    }
}

impl<A0, F, Fut> AsyncFnOnce<A0> for F
where
    F: FnOnce(A0) -> Fut,
    Fut: Future,
{
    type Output = Fut::Output;
    type Future = Fut;

    fn call_once(self, a0: A0) -> Self::Future {
        self(a0)
    }
}

impl<A0, F, Fut> AsyncFnMut<A0> for F
where
    F: FnMut(A0) -> Fut,
    Fut: Future,
{
    fn call_mut(&mut self, a0: A0) -> Self::Future {
        self(a0)
    }
}

impl<A0, F, Fut> AsyncFn<A0> for F
where
    F: Fn(A0) -> Fut,
    Fut: Future,
{
    fn call(&self, a0: A0) -> Self::Future {
        self(a0)
    }
}
