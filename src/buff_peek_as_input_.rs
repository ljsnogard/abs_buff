use core::{
    borrow::BorrowMut,
    future::{IntoFuture, Future},
    marker::PhantomData,
    mem::MaybeUninit,
    pin::Pin,
    ptr::NonNull,
    task::{Context, Poll},
};

use abs_sync::cancellation::{NonCancellableToken, TrCancellationToken, TrMayCancel};
use anylr::SomeOf;

use crate::{TrBuffPeek, TrBuffSegmRef, TrInput};

pub struct BuffPeekAsInput<B, P, T>
where
    B: BorrowMut<P>,
    P: TrBuffPeek<T>,
{
    peeker_: B,
    offset_: usize,
    _use_p_: PhantomData<P>,
    _use_t_: PhantomData<[T]>,
}

impl<B, P, T> BuffPeekAsInput<B, P, T>
where
    B: BorrowMut<P>,
    P: TrBuffPeek<T>,
{
    pub const fn new(peeker: B, offset: usize) -> Self {
        BuffPeekAsInput {
            peeker_: peeker,
            offset_: offset,
            _use_p_: PhantomData,
            _use_t_: PhantomData,
        }
    }
}

impl<'a, P, T> From<&'a mut P> for BuffPeekAsInput<&'a mut P, P, T>
where
    P: TrBuffPeek<T>,
{
    fn from(value: &'a mut P) -> Self {
        BuffPeekAsInput::new(value, 0usize)
    }
}

impl<P, T> From<P> for BuffPeekAsInput<P, P, T>
where
    P: TrBuffPeek<T>,
{
    fn from(value: P) -> Self {
        BuffPeekAsInput::new(value, 0usize)
    }
}

impl<B, P, T> TrInput<T> for BuffPeekAsInput<B, P, T>
where
    B: BorrowMut<P>,
    P: TrBuffPeek<T>,
{
    type Err = <P as TrBuffPeek<T>>::Err;

    type ReadAsync<'a> = BuffPeekInputAsync<'a, B, P, T>
    where
        T: 'a,
        Self: 'a;

    fn read_async<'a>(
        &'a mut self,
        target: &'a mut [MaybeUninit<T>],
    ) -> Self::ReadAsync<'a> {
        BuffPeekInputAsync::new(self, target)
    }
}

pub struct BuffPeekInputAsync<'a, B, P, T>
where
    B: BorrowMut<P>,
    P: TrBuffPeek<T>,
{
    input_: &'a mut BuffPeekAsInput<B, P, T>,
    target_: &'a mut [MaybeUninit<T>],
}

impl<'a, B, P, T> BuffPeekInputAsync<'a, B, P, T>
where
    B: BorrowMut<P>,
    P: TrBuffPeek<T>,
{
    pub const fn new(
        input: &'a mut BuffPeekAsInput<B, P, T>,
        target: &'a mut [MaybeUninit<T>],
    ) -> Self {
        BuffPeekInputAsync {
            input_: input,
            target_: target,
        }
    }

    pub fn may_cancel_with<'f, C: TrCancellationToken>(
        self,
        cancel: Pin<&'f mut C>,
    ) -> BuffPeekInputFuture<'f, C, B, P, T>
    where
        Self: 'f,
    {
        BuffPeekInputFuture::new(self.input_, self.target_, cancel)
    }
}

impl<'a, B, P, T> IntoFuture for BuffPeekInputAsync<'a, B, P, T>
where
    B: BorrowMut<P>,
    P: TrBuffPeek<T>,
{
    type IntoFuture = BuffPeekInputFuture<'a, NonCancellableToken, B, P, T>;
    type Output = <Self::IntoFuture as Future>::Output;

    fn into_future(self) -> Self::IntoFuture {
        let cancel = NonCancellableToken::pinned();
        BuffPeekInputFuture::new(self.input_, self.target_, cancel)
    }
}

impl<'a, B, P, T> TrMayCancel<'a> for BuffPeekInputAsync<'a, B, P, T>
where
    B: BorrowMut<P>,
    P: TrBuffPeek<T>,
{
    type MayCancelOutput = SomeOf<usize, <P as TrBuffPeek<T>>::Err>;

    fn may_cancel_with<'f, C: TrCancellationToken>(
        self,
        cancel: Pin<&'f mut C>,
    ) -> impl IntoFuture<Output = Self::MayCancelOutput>
    where
        Self: 'f
    {
        BuffPeekInputAsync::may_cancel_with(self, cancel)
    }
}

pub struct BuffPeekInputFuture<'a, C, B, P, T>
where
    C: TrCancellationToken,
    B: BorrowMut<P>,
    P: TrBuffPeek<T>,
{
    input_: &'a mut BuffPeekAsInput<B, P, T>,
    target_: &'a mut [MaybeUninit<T>],
    cancel_: Pin<&'a mut C>,
    future_: Option<<FutImpl<'a, C, B, P, T> as AsyncFnOnce<()>>::CallOnceFuture>,
}

impl<'a, C, B, P, T> BuffPeekInputFuture<'a, C, B, P, T>
where
    C: TrCancellationToken,
    B: BorrowMut<P>,
    P: TrBuffPeek<T>,
{
    pub const fn new(
        input: &'a mut BuffPeekAsInput<B, P, T>,
        target: &'a mut [MaybeUninit<T>],
        cancel: Pin<&'a mut C>,
    ) -> Self {
        BuffPeekInputFuture {
            input_: input,
            target_: target,
            cancel_: cancel,
            future_: Option::None,
        }
    }
}

impl<'a, C, B, P, T> Future for BuffPeekInputFuture<'a, C, B, P, T>
where
    C: TrCancellationToken,
    B: BorrowMut<P>,
    P: TrBuffPeek<T>,
{
    type Output = SomeOf<usize, <P as TrBuffPeek<T>>::Err>;

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

struct FutImpl<'a, C, B, P, T>(Pin<&'a mut BuffPeekInputFuture<'a, C, B, P, T>>)
where
    C: TrCancellationToken,
    B: BorrowMut<P>,
    P: TrBuffPeek<T>;

impl<C, B, P, T> AsyncFnOnce<()> for FutImpl<'_, C, B, P, T>
where
    C: TrCancellationToken,
    B: BorrowMut<P>,
    P: TrBuffPeek<T>,
{
    type CallOnceFuture = impl Future<Output = Self::Output>;
    type Output = SomeOf<usize, <P as TrBuffPeek<T>>::Err>;

    #[inline]
    extern "rust-call" fn async_call_once(
        self,
        _: (),
    ) -> Self::CallOnceFuture {
        let future = unsafe { self.0.get_unchecked_mut() };
        Self::may_cancel_impl(
            future.input_,
            future.target_,
            future.cancel_.as_mut(),
        )
    }
}

impl<'a, C, B, P, T> FutImpl<'a, C, B, P, T>
where
    C: TrCancellationToken,
    B: BorrowMut<P>,
    P: TrBuffPeek<T>,
{
    pub const fn new(f: Pin<&'a mut BuffPeekInputFuture<'a, C, B, P, T>>) -> Self {
        FutImpl(f)
    }

    pub async fn may_cancel_impl<'f>(
        input: &'f mut BuffPeekAsInput<B, P, T>,
        target: &'f mut [MaybeUninit<T>],
        cancel: Pin<&'f mut C>,
    ) -> SomeOf<usize, <P as TrBuffPeek<T>>::Err> {
        let peeker: &mut P = input.peeker_.borrow_mut();
        let (opt_segm, opt_err) = peeker
            .peek_async()
            .may_cancel_with(cancel)
            .await
            .split();
        let mut copied = 0usize;
        if let Option::Some(mut segment) = opt_segm {
            let prev_done = segment.take_segm_ref(input.offset_);
            drop(prev_done);
            copied = segment.fill_into_buff(target);
        };
        input.offset_ += copied;
        if let Option::Some(err) = opt_err {
            if copied > 0 {
                SomeOf::new_both(copied, err)
            } else {
                SomeOf::new_right(err)
            }
        } else {
            SomeOf::new_left(copied)
        }
    }
}
