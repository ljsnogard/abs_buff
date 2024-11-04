use core::{
    error::Error,
    fmt,
    future::{self, Future, IntoFuture, Ready},
    marker::PhantomData,
    pin::Pin,
};

use abs_sync::cancellation::{TrCancellationToken, TrIntoFutureMayCancel};

use crate::{
    TrBuffIterPeek, TrBuffIterRead, TrBuffIterWrite, TrBuffIterTryPeek,
    TrBuffIterTryRead, TrBuffIterTryWrite,
};

/// A placeholder type of buffer that will not lend any slices.
pub struct EmptyBuffIter<T>(PhantomData<[T; 0]>)
where
    T: Clone;

impl<T> EmptyBuffIter<T>
where
    T: Clone,
{
    pub const fn new() -> Self {
        EmptyBuffIter(PhantomData)
    }
}

impl<T> Default for EmptyBuffIter<T>
where
    T: Clone,
{
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl<T> TrBuffIterPeek<T> for EmptyBuffIter<T>
where
    T: Clone,
{
    type SliceRef<'a> = &'a [T] where Self: 'a;
    type BuffIter<'a> = [Self::SliceRef<'a>; 0] where Self: 'a;
    type Err = EmptyBuffIterError;

    type PeekAsync<'a> = DisabledPeekAsync<'a, T> where Self: 'a;

    fn peek_async(&mut self) -> Self::PeekAsync<'_> {
        DisabledPeekAsync::new()
    }
}

impl<T> TrBuffIterTryPeek<T> for EmptyBuffIter<T>
where
    T: Clone,
{
    fn try_peek(&mut self) -> Result<Self::BuffIter<'_>, Self::Err> {
        Result::Err(EmptyBuffIterError)
    }
}

impl<T> TrBuffIterRead<T> for EmptyBuffIter<T>
where
    T: Clone,
{
    type SliceRef<'a> = &'a [T] where Self: 'a;
    type BuffIter<'a> = [Self::SliceRef<'a>; 0] where Self: 'a;
    type Err = EmptyBuffIterError;

    type ReadAsync<'a> = DisabledReadAsync<'a, T> where Self: 'a;

    fn read_async(&mut self, _: usize) -> Self::ReadAsync<'_> {
        DisabledReadAsync::new()
    }
}

impl<T> TrBuffIterTryRead<T> for EmptyBuffIter<T>
where
    T: Clone,
{
    fn try_read(&mut self, _: usize) -> Result<Self::BuffIter<'_>, Self::Err> {
        Result::Err(EmptyBuffIterError)
    }
}

impl<T> TrBuffIterWrite<T> for EmptyBuffIter<T>
where
    T: Clone,
{
    type SliceMut<'a> = &'a mut [T] where Self: 'a;
    type BuffIter<'a> = [Self::SliceMut<'a>; 0] where Self: 'a;
    type Err = EmptyBuffIterError;

    type WriteAsync<'a> = DisabledWriteAsync<'a, T> where Self: 'a;

    fn write_async(&mut self, _: usize) -> Self::WriteAsync<'_> {
        DisabledWriteAsync::new()
    }
}

impl<T> TrBuffIterTryWrite<T> for EmptyBuffIter<T>
where
    T: Clone,
{
    fn try_write(&mut self, _: usize) -> Result<Self::BuffIter<'_>, Self::Err> {
        Result::Err(EmptyBuffIterError)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EmptyBuffIterError;

impl Error for EmptyBuffIterError
{}

impl fmt::Display for EmptyBuffIterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EmptyBuffIterError")
    }
}

pub struct DisabledPeekAsync<'a, T>(PhantomData<&'a mut EmptyBuffIter<T>>)
where
    T: Clone;

impl<'a, T> DisabledPeekAsync<'a, T>
where
    T: Clone,
{
    fn new() -> Self {
        DisabledPeekAsync(PhantomData)
    }
}

impl<'a, T> IntoFuture for DisabledPeekAsync<'a, T>
where
    T: Clone,
{
    type IntoFuture = Ready<Self::Output>;
    type Output = Result<[&'a [T]; 0], EmptyBuffIterError>;

    fn into_future(self) -> Self::IntoFuture {
        future::ready(Result::Err(EmptyBuffIterError))
    }
}

impl<'a, T> TrIntoFutureMayCancel<'a> for DisabledPeekAsync<'a, T>
where
    T: Clone,
{
    type MayCancelOutput = <<Self as IntoFuture>::IntoFuture as Future>::Output;

    fn may_cancel_with<C>(
        self,
        cancel: Pin<&'a mut C>,
    ) -> impl future::Future<Output = Self::MayCancelOutput>
    where
        C: TrCancellationToken,
    {
        let _ = cancel;
        future::ready(Result::Err(EmptyBuffIterError))
    }
}

pub struct DisabledReadAsync<'a, T>(PhantomData<&'a mut EmptyBuffIter<T>>)
where
    T: Clone;

impl<'a, T> DisabledReadAsync<'a, T>
where
    T: Clone,
{
    fn new() -> Self {
        DisabledReadAsync(PhantomData)
    }
}

impl<'a, T> IntoFuture for DisabledReadAsync<'a, T>
where
    T: Clone,
{
    type IntoFuture = Ready<Self::Output>;
    type Output = Result<[&'a [T]; 0], EmptyBuffIterError>;

    fn into_future(self) -> Self::IntoFuture {
        future::ready(Result::Err(EmptyBuffIterError))
    }
}

impl<'a, T> TrIntoFutureMayCancel<'a> for DisabledReadAsync<'a, T>
where
    T: Clone,
{
    type MayCancelOutput = <<Self as IntoFuture>::IntoFuture as Future>::Output;

    fn may_cancel_with<C>(
        self,
        cancel: Pin<&'a mut C>,
    ) -> impl Future<Output = Self::MayCancelOutput>
    where
        C: TrCancellationToken,
    {
        let _ = cancel;
        future::ready(Result::Err(EmptyBuffIterError))
    }
}

pub struct DisabledWriteAsync<'a, T>(PhantomData<&'a mut EmptyBuffIter<T>>)
where
    T: Clone;

impl<'a, T> DisabledWriteAsync<'a, T>
where
    T: Clone,
{
    fn new() -> Self {
        DisabledWriteAsync(PhantomData)
    }
}

impl<'a, T> IntoFuture for DisabledWriteAsync<'a, T>
where
    T: Clone,
{
    type IntoFuture = Ready<Self::Output>;
    type Output = Result<[&'a mut [T]; 0], EmptyBuffIterError>;

    fn into_future(self) -> Self::IntoFuture {
        future::ready(Result::Err(EmptyBuffIterError))
    }
}

impl<'a, T> TrIntoFutureMayCancel<'a> for DisabledWriteAsync<'a, T>
where
    T: Clone,
{
    type MayCancelOutput = <<Self as IntoFuture>::IntoFuture as Future>::Output;

    fn may_cancel_with<C>(
        self,
        cancel: Pin<&'a mut C>,
    ) -> impl Future<Output = Self::MayCancelOutput>
    where
        C: TrCancellationToken,
    {
        let _ = cancel;
        future::ready(Result::Err(EmptyBuffIterError))
    }
}
