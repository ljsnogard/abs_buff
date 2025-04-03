use core::{
    borrow::BorrowMut,
    future::{IntoFuture, Future},
    iter::IntoIterator,
    marker::PhantomData,
    mem::MaybeUninit,
    pin::Pin,
    ptr::NonNull,
    task::{Context, Poll},
};

use abs_sync::cancellation::{NonCancellableToken, TrCancellationToken, TrMayCancel};
use anylr::SomeOf;

use crate::{Demand, TrBuffWrite, TrBuffSegmMut, TrOutput};

pub struct BuffWriteAsOutput<B, W, T>(B, PhantomData<W>, PhantomData<[T]>)
where
    B: BorrowMut<W>,
    W: TrBuffWrite<T>;

impl<B, W, T> BuffWriteAsOutput<B, W, T>
where
    B: BorrowMut<W>,
    W: TrBuffWrite<T>,
{
    pub const fn new(r: B) -> Self {
        BuffWriteAsOutput(r, PhantomData, PhantomData)
    }
}

impl<'a, W, T> From<&'a mut W> for BuffWriteAsOutput<&'a mut W, W, T>
where
    W: TrBuffWrite<T>,
{
    fn from(value: &'a mut W) -> Self {
        BuffWriteAsOutput::<&'a mut W, W, T>::new(value)
    }
}

impl<W, T> From<W> for BuffWriteAsOutput<W, W, T>
where
    W: TrBuffWrite<T>,
{
    fn from(value: W) -> Self {
        BuffWriteAsOutput::new(value)
    }
}

impl<B, W, T> TrOutput<T> for BuffWriteAsOutput<B, W, T>
where
    B: BorrowMut<W>,
    W: TrBuffWrite<T>,
{
    type Err = <W as TrBuffWrite<T>>::Err;

    type WriteAsync<'a> = BuffWriteOutputAsync<'a, W, T>
    where
        T: 'a,
        Self: 'a;

    fn write_async<'a>(
        &'a mut self,
        source: &'a [MaybeUninit<T>],
    ) -> Self::WriteAsync<'a> {
        BuffWriteOutputAsync::new(self.0.borrow_mut(), source)
    }
}

pub struct BuffWriteOutputAsync<'a, W, T>
where
    W: TrBuffWrite<T>,
{
    writer_: &'a mut W,
    source_: &'a [MaybeUninit<T>],
}

impl<'a, W, T> BuffWriteOutputAsync<'a, W, T>
where
    W: TrBuffWrite<T>,
{
    pub const fn new(writer: &'a mut W, source: &'a [MaybeUninit<T>]) -> Self {
        BuffWriteOutputAsync {
            writer_: writer,
            source_: source,
        }
    }

    pub fn may_cancel_with<'f, C: TrCancellationToken>(
        self,
        cancel: Pin<&'f mut C>,
    ) -> BuffWriteOutputFuture<'f, C, W, T>
    where
        Self: 'f,
    {
        BuffWriteOutputFuture::new(self.writer_, self.source_, cancel)
    }
}

impl<'a, W, T> IntoFuture for BuffWriteOutputAsync<'a, W, T>
where
    W: TrBuffWrite<T>,
{
    type IntoFuture = BuffWriteOutputFuture<'a, NonCancellableToken, W, T>;
    type Output = <Self::IntoFuture as Future>::Output;

    #[inline]
    fn into_future(self) -> Self::IntoFuture {
        let cancel = NonCancellableToken::pinned();
        BuffWriteOutputFuture::new(
            self.writer_,
            self.source_,
            cancel,
        )
    }
}

impl<'a, W, T> TrMayCancel<'a> for BuffWriteOutputAsync<'a, W, T>
where
    W: TrBuffWrite<T>,
{
    type MayCancelOutput = <Self as IntoFuture>::Output;

    #[inline]
    fn may_cancel_with<'f, C: TrCancellationToken>(
        self,
        cancel: Pin<&'f mut C>,
    ) -> impl IntoFuture<Output = Self::MayCancelOutput>
    where
        Self: 'f
    {
        BuffWriteOutputAsync::may_cancel_with(self, cancel)
    }
}

pub struct BuffWriteOutputFuture<'a, C, W, T>
where
    C: TrCancellationToken,
    W: TrBuffWrite<T>,
{
    writer_: &'a mut W,
    source_: &'a [MaybeUninit<T>],
    cancel_: Pin<&'a mut C>,
    future_: Option<<FutImpl<'a, C, W, T> as AsyncFnOnce<()>>::CallOnceFuture>,
}

impl<'a, C, W, T> BuffWriteOutputFuture<'a, C, W, T>
where
    C: TrCancellationToken,
    W: TrBuffWrite<T>,
{
    pub const fn new(
        writer: &'a mut W,
        source: &'a [MaybeUninit<T>],
        cancel: Pin<&'a mut C>,
    ) -> Self {
        BuffWriteOutputFuture {
            writer_: writer,
            source_: source,
            cancel_: cancel,
            future_: Option::None,
        }
    }
}

impl<'a, C, W, T> Future for BuffWriteOutputFuture<'a, C, W, T>
where
    C: TrCancellationToken,
    W: TrBuffWrite<T>,
{
    type Output = SomeOf<usize, <W as TrBuffWrite<T>>::Err>;

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Self::Output> {
        let mut this = unsafe {
            let p = self.as_mut().get_unchecked_mut();
            NonNull::new_unchecked(p)
        };
        loop {
            let mut p = unsafe {
                let ptr = &mut this.as_mut().future_;
                NonNull::new_unchecked(ptr)
            };
            let opt_f = unsafe { p.as_mut() };
            if let Option::Some(f) = opt_f {
                let f_pinned = unsafe { Pin::new_unchecked(f) };
                break f_pinned.poll(cx)
            } else {
                let h = FutImpl::new(unsafe { 
                    Pin::new_unchecked(this.as_mut())
                });
                let f = h();
                let opt = unsafe { p.as_mut() };
                *opt = Option::Some(f);
            }
        }
    }
}

struct FutImpl<'a, C, W, T>(Pin<&'a mut BuffWriteOutputFuture<'a, C, W, T>>)
where
    C: TrCancellationToken,
    W: TrBuffWrite<T>;

impl<C, W, T> AsyncFnOnce<()> for FutImpl<'_, C, W, T>
where
    C: TrCancellationToken,
    W: TrBuffWrite<T>,
{
    type CallOnceFuture = impl Future<Output = Self::Output>;
    type Output = SomeOf<usize, <W as TrBuffWrite<T>>::Err>;

    #[inline]
    extern "rust-call" fn async_call_once(
        self,
        _: (),
    ) -> Self::CallOnceFuture {
        let future = unsafe { self.0.get_unchecked_mut() };
        Self::may_cancel_impl(
            future.writer_,
            future.source_,
            future.cancel_.as_mut(),
        )
    }
}

impl<'a, C, W, T> FutImpl<'a, C, W, T>
where
    C: TrCancellationToken,
    W: TrBuffWrite<T>,
{
    pub const fn new(f: Pin<&'a mut BuffWriteOutputFuture<'a, C, W, T>>) -> Self {
        FutImpl(f)
    }

    pub async fn may_cancel_impl<'f>(
        writer: &'f mut W,
        source: &'f [MaybeUninit<T>],
        mut cancel: Pin<&'f mut C>,
    ) -> SomeOf<usize, <W as TrBuffWrite<T>>::Err> {
        return writer
            .write_async(&Demand::at_most(source.len()))
            .may_cancel_with(cancel.as_mut())
            .await
            .map_left(|segms| dump_buff_into_segms(source, segms));

        fn dump_buff_into_segms<'d, I, S, X>(
            buffer: &'d [MaybeUninit<X>],
            segments: I,
        ) -> usize
        where
            I: IntoIterator<Item = S>,
            S: TrBuffSegmMut<X>,
        {
            let mut copied = 0usize;
            let buff_len = buffer.len();
            for mut s in segments.into_iter() {
                let source = &buffer[copied..buff_len - copied];
                let c = s.dump_from_slice(source);
                copied += c;
                if copied == buff_len {
                    break
                }
            }
            copied
        }
    }
}
