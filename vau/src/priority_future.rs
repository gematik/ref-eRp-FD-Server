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

use std::cmp::min;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll};

use pin_project::{pin_project, pinned_drop};

#[pin_project(PinnedDrop)]
pub struct PriorityFuture<F> {
    #[pin]
    future: F,
    high_prio: bool,
    delay: usize,
}

impl<F> PriorityFuture<F> {
    pub fn new(future: F, high_prio: bool) -> Self {
        if high_prio {
            HIGH_PRIORITY_FUTURES.fetch_add(1, Ordering::Release);
        }

        Self {
            future,
            high_prio,
            delay: if high_prio {
                0
            } else {
                min(MAX_DELAY, HIGH_PRIORITY_FUTURES.load(Ordering::Acquire))
            },
        }
    }
}

#[pinned_drop]
impl<F> PinnedDrop for PriorityFuture<F> {
    fn drop(self: Pin<&mut Self>) {
        if self.high_prio {
            HIGH_PRIORITY_FUTURES.fetch_sub(1, Ordering::Release);
        }
    }
}

impl<F: Future> Future for PriorityFuture<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        if *this.delay == 0 {
            *this.delay = if *this.high_prio {
                0
            } else {
                min(MAX_DELAY, HIGH_PRIORITY_FUTURES.load(Ordering::Acquire))
            };

            this.future.poll(cx)
        } else {
            *this.delay -= 1;
            cx.waker().wake_by_ref();

            Poll::Pending
        }
    }
}

lazy_static! {
    static ref HIGH_PRIORITY_FUTURES: AtomicUsize = AtomicUsize::new(0);
}

const MAX_DELAY: usize = 20;

#[cfg(test)]
mod test {
    use super::*;

    use std::cell::RefCell;
    use std::rc::Rc;

    use futures::join;

    #[tokio::test]
    async fn execute_priotized() {
        let id = Rc::new(RefCell::new(0));

        let fut1 = PriorityFuture::new(test_fut(id.clone()), true);
        let fut2 = PriorityFuture::new(test_fut(id.clone()), false);
        let fut3 = PriorityFuture::new(test_fut(id.clone()), true);

        let ret = join!(fut1, fut2, fut3);
        assert_eq!((1, 3, 2), ret);
    }

    #[tokio::test]
    async fn execute_non_priotized() {
        let id = Rc::new(RefCell::new(0));

        let fut1 = PriorityFuture::new(test_fut(id.clone()), true);
        let fut2 = PriorityFuture::new(test_fut(id.clone()), true);
        let fut3 = PriorityFuture::new(test_fut(id.clone()), true);

        let ret = join!(fut1, fut2, fut3);

        assert_eq!((1, 2, 3), ret);
    }

    fn test_fut(id: Rc<RefCell<usize>>) -> TestFuture {
        TestFuture { id }
    }

    struct TestFuture {
        id: Rc<RefCell<usize>>,
    }

    impl Future for TestFuture {
        type Output = usize;

        fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
            let this = self.get_mut();

            *this.id.borrow_mut() += 1;

            Poll::Ready(*this.id.borrow())
        }
    }
}
