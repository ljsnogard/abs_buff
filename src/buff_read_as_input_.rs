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

use crate::{
    io::TrUnbufferedInput,
    TrBuffIterRead, TrBuffSegmRef,
};

pub struct BuffReadAsInput<B, R, T>(B, PhantomData<R>, PhantomData<[T]>)
where
    B: BorrowMut<R>,
    R: TrBuffIterRead<T>;

impl<B, R, T> BuffReadAsInput<B, R, T>
where
    B: BorrowMut<R>,
    R: TrBuffIterRead<T>,
{
    pub const fn new(r: B) -> Self {
        BuffReadAsInput(r, PhantomData, PhantomData)
    }
}

impl<'a, R, T> From<&'a mut R> for BuffReadAsInput<&'a mut R, R, T>
where
    R: TrBuffIterRead<T>,
{
    fn from(value: &'a mut R) -> Self {
        BuffReadAsInput::new(value)
    }
}

impl<R, T> From<R> for BuffReadAsInput<R, R, T>
where
    R: TrBuffIterRead<T>,
{
    fn from(value: R) -> Self {
        BuffReadAsInput::new(value)
    }
}

impl<B, R, T> TrUnbufferedInput<T> for BuffReadAsInput<B, R, T>
where
    B: BorrowMut<R>,
    R: TrBuffIterRead<T>,
{
    type Err = <R as TrBuffIterRead<T>>::Err;

    type ReadAsync<'a> = BuffReadInputAsync<'a, R, T>
    where
        T: 'a,
        Self: 'a;

    fn read_async<'a>(
        &'a mut self,
        target: &'a mut [MaybeUninit<T>],
    ) -> Self::ReadAsync<'a> {
        BuffReadInputAsync::new(self.0.borrow_mut(), target)
    }
}

pub struct BuffReadInputAsync<'a, R, T>
where
    R: TrBuffIterRead<T>,
{
    reader_: &'a mut R,
    target_: &'a mut [MaybeUninit<T>],
}

impl<'a, R, T> BuffReadInputAsync<'a, R, T>
where
    R: TrBuffIterRead<T>,
{
    pub const fn new(
        reader: &'a mut R,
        target: &'a mut [MaybeUninit<T>],
    ) -> Self {
        BuffReadInputAsync {
            reader_: reader,
            target_: target,
        }
    }

    pub fn may_cancel_with<'f, C: TrCancellationToken>(
        self,
        cancel: Pin<&'f mut C>,
    ) -> BuffReadInputFuture<'f, C, R, T>
    where
        Self: 'f,
    {
        BuffReadInputFuture::new(self.reader_, self.target_, cancel)
    }
}

impl<'a, R, T> IntoFuture for BuffReadInputAsync<'a, R, T>
where
    R: TrBuffIterRead<T>,
{
    type IntoFuture = BuffReadInputFuture<'a, NonCancellableToken, R, T>;
    type Output = <Self::IntoFuture as Future>::Output;

    fn into_future(self) -> Self::IntoFuture {
        let cancel = NonCancellableToken::pinned();
        BuffReadInputFuture::new(self.reader_, self.target_, cancel)
    }
}

impl<'a, R, T> TrMayCancel<'a> for BuffReadInputAsync<'a, R, T>
where
    R: TrBuffIterRead<T>,
{
    type MayCancelOutput = SomeOf<usize, <R as TrBuffIterRead<T>>::Err>;

    fn may_cancel_with<'f, C: abs_sync::preludes::TrCancellationToken>(
        self,
        cancel: Pin<&'f mut C>,
    ) -> impl IntoFuture<Output = Self::MayCancelOutput>
    where
        Self: 'f
    {
        BuffReadInputAsync::may_cancel_with(self, cancel)
    }
}

pub struct BuffReadInputFuture<'a, C, R, T>
where
    C: TrCancellationToken,
    R: TrBuffIterRead<T>,
{
    reader_: &'a mut R,
    target_: &'a mut [MaybeUninit<T>],
    cancel_: Pin<&'a mut C>,
    future_: Option<<FutImpl<'a, C, R, T> as AsyncFnOnce<()>>::CallOnceFuture>,
}

impl<'a, C, R, T> BuffReadInputFuture<'a, C, R, T>
where
    C: TrCancellationToken,
    R: TrBuffIterRead<T>,
{
    pub const fn new(
        reader: &'a mut R,
        target: &'a mut [MaybeUninit<T>],
        cancel: Pin<&'a mut C>,
    ) -> Self {
        BuffReadInputFuture {
            reader_: reader,
            target_: target,
            cancel_: cancel,
            future_: Option::None,
        }
    }
}

impl<'a, C, R, T> Future for BuffReadInputFuture<'a, C, R, T>
where
    C: TrCancellationToken,
    R: TrBuffIterRead<T>,
{
    type Output = SomeOf<usize, <R as TrBuffIterRead<T>>::Err>;

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

struct FutImpl<'a, C, R, T>(Pin<&'a mut BuffReadInputFuture<'a, C, R, T>>)
where
    C: TrCancellationToken,
    R: TrBuffIterRead<T>;

impl<C, R, T> AsyncFnOnce<()> for FutImpl<'_, C, R, T>
where
    C: TrCancellationToken,
    R: TrBuffIterRead<T>,
{
    type CallOnceFuture = impl Future<Output = Self::Output>;
    type Output = SomeOf<usize, <R as TrBuffIterRead<T>>::Err>;

    #[inline]
    extern "rust-call" fn async_call_once(
        self,
        _: (),
    ) -> Self::CallOnceFuture {
        let future = unsafe { self.0.get_unchecked_mut() };
        Self::may_cancel_impl(
            future.reader_,
            future.target_,
            future.cancel_.as_mut(),
        )
    }
}

impl<'a, C, R, T> FutImpl<'a, C, R, T>
where
    C: TrCancellationToken,
    R: TrBuffIterRead<T>,
{
    pub const fn new(f: Pin<&'a mut BuffReadInputFuture<'a, C, R, T>>) -> Self {
        FutImpl(f)
    }

    pub async fn may_cancel_impl<'f>(
        reader: &'f mut R,
        target: &'f mut [MaybeUninit<T>],
        mut cancel: Pin<&'f mut C>,
    ) -> SomeOf<usize, <R as TrBuffIterRead<T>>::Err> {
        return reader
            .read_async(target.len())
            .may_cancel_with(cancel.as_mut())
            .await
            .map_left(|segms| fill_buff_with_segms(segms, target));

        fn fill_buff_with_segms<'d, I, S, X>(
            segments: I,
            buffer: &'d mut [MaybeUninit<X>],
        ) -> usize
        where
            I: IntoIterator<Item = S>,
            S: TrBuffSegmRef<X>,
        {
            let mut copied = 0usize;
            let buff_len = buffer.len();
            for mut s in segments.into_iter() {
                let target = &mut buffer[copied..buff_len - copied];
                let c = s.fill_into_buff(target);
                copied += c;
                if copied == buff_len {
                    break
                }
            }
            copied
        }
    }
}
