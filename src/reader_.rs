use core::{
    future::{self, Future, IntoFuture, Ready},
    marker::PhantomData,
    ops::Deref,
    pin::Pin,
};
use abs_sync::cancellation::{TrCancellationToken, TrIntoFutureMayCancel};

use crate::utils::{ReaderAsChunkFiller, TrChunkFiller};

pub trait TrBuffReader<T: Clone = u8> {
    type BuffRef<'a>: Deref<Target = [T]> where Self: 'a;

    type Error;

    type ReadAsync<'a>: TrIntoFutureMayCancel<'a, MayCancelOutput =
        Result<Self::BuffRef<'a>, Self::Error>>
    where
        Self: 'a;

    fn can_read(&mut self) -> bool;

    fn read_async(&mut self, length: usize) -> Self::ReadAsync<'_>;

    fn as_filler(&mut self) -> impl TrChunkFiller<T> where Self: Sized {
        ReaderAsChunkFiller::<&mut Self, Self, T>::new(self)
    }
}

pub struct DisabledBuffReader<T: Clone>(PhantomData<[T; 0]>);

impl<T: Clone> TrBuffReader<T> for DisabledBuffReader<T> {
    type BuffRef<'a> = &'a [T] where Self: 'a;

    type Error = ();

    type ReadAsync<'a> = ReadAsync<'a, T> where Self: 'a;

    fn can_read(&mut self) -> bool {
        false
    }

    fn read_async(&mut self, _: usize) -> Self::ReadAsync<'_> {
        self::ReadAsync::new()
    }
}

pub struct ReadAsync<'a, T: Clone>(PhantomData<&'a mut DisabledBuffReader<T>>);

impl<'a, T: Clone> ReadAsync<'a, T> {
    fn new() -> Self {
        ReadAsync(PhantomData)
    }
}

impl<'a, T: Clone> IntoFuture for ReadAsync<'a, T> {
    type IntoFuture = Ready<Self::Output>;
    type Output = Result<&'a [T], ()>;

    fn into_future(self) -> Self::IntoFuture {
        future::ready(Result::Err(()))
    }
}

impl<'a, T: Clone> TrIntoFutureMayCancel<'a> for ReadAsync<'a, T> {
    type MayCancelOutput = <<Self as IntoFuture>::IntoFuture as Future>::Output;

    fn may_cancel_with<C>(
        self,
        cancel: Pin<&'a mut C>,
    ) -> impl Future<Output = Self::MayCancelOutput>
    where
        C: TrCancellationToken,
    {
        let _ = cancel;
        future::ready(Result::Err(()))
    }
}
