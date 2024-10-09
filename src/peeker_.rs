use core::{
    future::{self, Future, IntoFuture, Ready},
    marker::PhantomData,
    ops::Deref,
    pin::Pin,
};
use abs_sync::cancellation::{TrCancellationToken, TrIntoFutureMayCancel};

use crate::utils::{PeekerAsChunkFiller, TrChunkFiller};

pub trait TrBuffPeeker<T: Clone = u8> {
    type BuffRef<'a>: Deref<Target = [T]> where Self: 'a;

    type Error;

    type PeekAsync<'a>: TrIntoFutureMayCancel<'a, MayCancelOutput =
        Result<Self::BuffRef<'a>, Self::Error>>
    where
        Self: 'a;

    fn can_peek(&mut self, skip: usize) -> bool;

    fn peek_async(&mut self, skip: usize) -> Self::PeekAsync<'_>;

    fn as_filler(&mut self) -> impl TrChunkFiller<T> where Self: Sized {
        PeekerAsChunkFiller::<&mut Self, Self, T>::new(self)
    }
}

pub struct DisabledBuffPeeker<T: Clone>(PhantomData<[T; 0]>);

impl<T: Clone> TrBuffPeeker<T> for DisabledBuffPeeker<T> {
    type BuffRef<'a> = &'a [T] where Self: 'a;

    type Error = ();

    type PeekAsync<'a> = PeekAsync<'a, T> where Self: 'a;

    fn can_peek(&mut self, _: usize) -> bool {
        false
    }

    fn peek_async(&mut self, _: usize) -> Self::PeekAsync<'_> {
        self::PeekAsync::new()
    }
}

pub struct PeekAsync<'a, T: Clone>(PhantomData<&'a mut DisabledBuffPeeker<T>>);

impl<'a, T: Clone> PeekAsync<'a, T> {
    fn new() -> Self {
        PeekAsync(PhantomData)
    }
}

impl<'a, T: Clone> IntoFuture for PeekAsync<'a, T> {
    type IntoFuture = Ready<Self::Output>;
    type Output = Result<&'a [T], ()>;

    fn into_future(self) -> Self::IntoFuture {
        future::ready(Result::Err(()))
    }
}

impl<'a, T: Clone> TrIntoFutureMayCancel<'a> for PeekAsync<'a, T> {
    type MayCancelOutput = <<Self as IntoFuture>::IntoFuture as Future>::Output;

    fn may_cancel_with<C>(
        self,
        cancel: Pin<&'a mut C>,
    ) -> impl future::Future<Output = Self::MayCancelOutput>
    where
        C: TrCancellationToken,
    {
        let _ = cancel;
        future::ready(Result::Err(()))
    }
}
