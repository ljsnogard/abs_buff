use core::{
    borrow::BorrowMut,
    future::{Future, IntoFuture},
    marker::PhantomData,
    pin::Pin,
    ptr::NonNull,
    sync::atomic::AtomicUsize,
    task::{Context, Poll},
};

use pin_project::pin_project;
use pin_utils::pin_mut;

use atomex::AtomicCountOwned;
use abs_sync::{cancellation::*, x_deps::{atomex, pin_utils}};

use crate::{
    utils::{IoAbortReport, TrChunkFiller},
    TrBuffPeeker,
};

pub struct PeekerAsChunkFiller<B, P, T>
where
    B: BorrowMut<P>,
    P: TrBuffPeeker<T>,
    T: Clone,
{
    _use_p_: PhantomData<P>,
    _use_t_: PhantomData<[T]>,
    peeker_: B,
    offset_: AtomicCountOwned<usize>,
}

impl<B, P, T> PeekerAsChunkFiller<B, P, T>
where
    B: BorrowMut<P>,
    P: TrBuffPeeker<T>,
    T: Clone,
{
    pub const fn new(peeker: B) -> Self {
        PeekerAsChunkFiller {
            _use_p_: PhantomData,
            _use_t_: PhantomData,
            peeker_: peeker,
            offset_: AtomicCountOwned::new(AtomicUsize::new(0usize)),
        }
    }

    pub fn can_fill(&mut self) -> bool {
        let skip = self.offset_.val();
        self.peeker_.borrow_mut().can_peek(skip)
    }

    pub fn fill_async<'a>(
        &'a mut self,
        target: &'a mut [T],
    ) -> PeekerFillAsync<'a, B, P, T> {
        PeekerFillAsync::new(self, target)
    }
}

impl<B, P, T> TrChunkFiller<T> for PeekerAsChunkFiller<B, P, T>
where
    B: BorrowMut<P>,
    P: TrBuffPeeker<T>,
    T: Clone,
{
    type FillError = IoAbortReport<<P as TrBuffPeeker<T>>::Error>;
    type FillAsync<'a> = PeekerFillAsync<'a, B, P, T> where Self: 'a;

    #[inline(always)]
    fn can_fill(&mut self) -> bool {
        PeekerAsChunkFiller::can_fill(self)
    }

    #[inline(always)]
    fn fill_async<'a>(
        &'a mut self,
        target: &'a mut [T],
    ) -> Self::FillAsync<'a> {
        PeekerAsChunkFiller::fill_async(self, target)
    }
}

pub struct PeekerFillAsync<'a, B, P, T>
where
    B: BorrowMut<P>,
    P: TrBuffPeeker<T>,
    T: Clone,
{
    filler_: &'a mut PeekerAsChunkFiller<B, P, T>,
    target_: &'a mut [T],
}

impl<'a, B, P, T> PeekerFillAsync<'a, B, P, T>
where
    B: BorrowMut<P>,
    P: TrBuffPeeker<T>,
    T: Clone,
{
    pub fn new(
        filler: &'a mut PeekerAsChunkFiller<B, P, T>,
        target: &'a mut [T],
    ) -> Self {
        PeekerFillAsync {
            filler_: filler,
            target_: target,
        }
    }

    fn may_cancel_with<C>(
        self,
        cancel: Pin<&'a mut C>,
    ) -> PeekerFillFuture<'a, C, B, P, T>
    where
        C: TrCancellationToken,
    {
        PeekerFillFuture::new(self.filler_, self.target_, cancel)
    }
}

impl<'a, B, P, T> IntoFuture for PeekerFillAsync<'a, B, P, T>
where
    B: BorrowMut<P>,
    P: TrBuffPeeker<T>,
    T: Clone,
{
    type IntoFuture = PeekerFillFuture<'a, NonCancellableToken, B, P, T>;
    type Output = <Self::IntoFuture as Future>::Output;

    fn into_future(self) -> Self::IntoFuture {
        let cancel = NonCancellableToken::pinned();
        PeekerFillFuture::new(self.filler_, self.target_, cancel)
    }
}

impl<'a, B, P, T> TrIntoFutureMayCancel<'a> for PeekerFillAsync<'a, B, P, T>
where
    B: BorrowMut<P>,
    P: TrBuffPeeker<T>,
    T: Clone,
{
    type MayCancelOutput = <Self as IntoFuture>::Output;

    #[inline(always)]
    fn may_cancel_with<C>(
        self,
        cancel: Pin<&'a mut C>,
    ) -> impl Future<Output = Self::MayCancelOutput>
    where
        C: TrCancellationToken,
    {
        PeekerFillAsync::may_cancel_with(self, cancel)
    }
}

#[pin_project]
pub struct PeekerFillFuture<'a, C, B, P, T>
where
    C: TrCancellationToken,
    B: BorrowMut<P>,
    P: TrBuffPeeker<T>,
    T: Clone,
{
    #[pin]filler_: &'a mut PeekerAsChunkFiller<B, P, T>,
    #[pin]target_: &'a mut [T],
    cancel_: Pin<&'a mut C>,
}

impl<'a, C, B, P, T> PeekerFillFuture<'a, C, B, P, T>
where
    C: TrCancellationToken,
    B: BorrowMut<P>,
    P: TrBuffPeeker<T>,
    T: Clone,
{
    pub fn new(
        filler: &'a mut PeekerAsChunkFiller<B, P, T>,
        target: &'a mut [T],
        cancel: Pin<&'a mut C>,
    ) -> Self {
        PeekerFillFuture {
            filler_: filler,
            target_: target,
            cancel_: cancel,
        }
    }

    async fn fill_async_(
        self: Pin<&mut Self>,
    ) -> Result<usize, IoAbortReport<<P as TrBuffPeeker<T>>::Error>> {
        let mut this = self.project();
        let mut filler = this.filler_.as_mut();
        let offset_ref = unsafe {
            // Safe because `offset_` is atomic
            let p = NonNull::new_unchecked(&mut filler.offset_);
            p.as_ref()
        };
        let peeker = filler.peeker_.borrow_mut();
        let mut target = this.target_.as_mut();
        let target_len = target.len();
        let mut perform_len = 0usize;
        let r = loop {
            if perform_len >= target_len {
                break Result::Ok(perform_len);
            }
            let req_len = target_len - perform_len;
            let r = peeker
                .peek_async(perform_len)
                .may_cancel_with(this.cancel_.as_mut())
                .await;
            let Result::Ok(src) = r else {
                let Result::Err(last_error) = r else {
                    unreachable!("[PeekerFillFuture::fill_async_]")
                };
                let report = IoAbortReport::new(perform_len, last_error);
                break Result::Err(report);
            };
            let src_len = src.len();
            let opr_len = core::cmp::min(src_len, req_len);
            
            #[cfg(test)]
            log::trace!(
                "[PeekerFillFuture::fill_async_] \
                peek_async src_len({src_len}) req_len({req_len})"
            );
            let dst = &mut target[perform_len..perform_len + opr_len];
            dst.clone_from_slice(&src[..opr_len]);
            perform_len += opr_len;
            drop(src);
        };
        offset_ref.add(perform_len);
        r
    }
}

impl<'a, C, B, P, T> Future for PeekerFillFuture<'a, C, B, P, T>
where
    C: TrCancellationToken,
    B: BorrowMut<P>,
    P: TrBuffPeeker<T>,
    T: Clone,
{
    type Output = Result<
        usize,
        IoAbortReport<<P as TrBuffPeeker<T>>::Error>,
    >;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let f = self.fill_async_();
        pin_mut!(f);
        f.poll(cx)
    }
}
