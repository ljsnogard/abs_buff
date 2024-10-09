use core::{
    future::{self, Future, IntoFuture, Ready},
    marker::PhantomData,
    ops::DerefMut,
    pin::Pin,
};
use abs_sync::cancellation::{TrCancellationToken, TrIntoFutureMayCancel};

pub trait TrBuffWriter<T: Clone = u8> {
    type BuffMut<'a>: DerefMut<Target = [T]> where Self: 'a;

    type Error;

    type WriteAsync<'a>: TrIntoFutureMayCancel<'a, MayCancelOutput =
        Result<Self::BuffMut<'a>, Self::Error>>
    where
        Self: 'a;

    fn can_write(&mut self) -> bool;

    fn write_async(&mut self, length: usize) -> Self::WriteAsync<'_>;
}


pub struct DisabledBuffWriter<T: Clone>(PhantomData<[T; 0]>);

impl<T: Clone> TrBuffWriter<T> for DisabledBuffWriter<T> {
    type BuffMut<'a> = &'a mut [T] where Self: 'a;

    type Error = ();

    type WriteAsync<'a> = WriteAsync<'a, T> where Self: 'a;

    fn can_write(&mut self) -> bool {
        false
    }

    fn write_async(&mut self, _: usize) -> Self::WriteAsync<'_> {
        self::WriteAsync::new()
    }
}

pub struct WriteAsync<'a, T>(PhantomData<&'a mut DisabledBuffWriter<T>>)
where
    T: Clone;

impl<'a, T: Clone> WriteAsync<'a, T> {
    fn new() -> Self {
        WriteAsync(PhantomData)
    }
}

impl<'a, T: Clone> IntoFuture for WriteAsync<'a, T> {
    type IntoFuture = Ready<Self::Output>;
    type Output = Result<&'a mut [T], ()>;

    fn into_future(self) -> Self::IntoFuture {
        future::ready(Result::Err(()))
    }
}

impl<'a, T: Clone> TrIntoFutureMayCancel<'a> for WriteAsync<'a, T> {
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
