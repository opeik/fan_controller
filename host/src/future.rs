use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use embassy_time::{Duration, Instant};
use pin_project::pin_project;

/// A wrapper around a [`core::future::Future`] which adds timing data.
#[pin_project]
#[must_use = "futures do nothing unless polled"]
pub struct TimedFuture<Fut>
where
    Fut: Future,
{
    #[pin]
    inner: Fut,
    start: Option<Instant>,
}

#[must_use]
pub struct Timed<T> {
    inner: T,
    duration: Duration,
}

impl<T> Timed<T> {
    pub fn inner(self) -> T {
        self.inner
    }

    pub fn duration(&self) -> Duration {
        self.duration
    }
}

impl<Fut> Future for TimedFuture<Fut>
where
    Fut: Future,
{
    type Output = Timed<Fut::Output>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = self.project();
        let start = this.start.get_or_insert_with(Instant::now);

        match this.inner.poll(cx) {
            // If the inner future is still pending, this wrapper is still pending.
            Poll::Pending => Poll::Pending,

            // If the inner future is done, measure the elapsed time and finish this wrapper future.
            Poll::Ready(v) => Poll::Ready(Timed {
                inner: v,
                duration: start.elapsed(),
            }),
        }
    }
}

/// An extension trait for [`core::future::Future`] that provides the
/// [`TimedExt::timed`] adaptor, adding timing data to futures.
pub trait TimedExt: Sized + Future {
    fn timed(self) -> TimedFuture<Self> {
        TimedFuture {
            inner: self,
            start: None,
        }
    }
}

// All futures can use the `.timed` method defined above
impl<F: Future> TimedExt for F {}

/// Polls a future along with a timeout. If the timeout finishes first, returns
/// `None`, else returns `Some(Future::Output)`.
macro_rules! timeout {
    ($future:expr, $timeout:expr) => {{
        use futures::{
            future::{self, Either},
            pin_mut,
        };
        let future = $future;
        let timeout = $timeout;
        pin_mut!(future);
        pin_mut!(timeout);

        match future::select(future, timeout).await {
            Either::Left((v, _)) => Some(v),
            Either::Right((_, _)) => None,
        }
    }};
}

/// Times the given [`core::future::Future`], returning `(Future::Output,
/// duration)`.
macro_rules! timed {
    ($e:expr) => {{
        use crate::future::TimedExt;
        let result = $e.timed().await;
        let duration = result.duration();
        let inner = result.inner();
        (inner, duration)
    }};
}

pub(crate) use timed;
pub(crate) use timeout;
