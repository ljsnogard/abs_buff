use core::{
    borrow::BorrowMut,
    future::{Future, IntoFuture},
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use pin_project::pin_project;
use pin_utils::pin_mut;

use abs_sync::{cancellation::*, x_deps::pin_utils};

use crate::{
    utils::{IoAbortReport, TrChunkFiller},
    TrBuffReader,
};

pub struct ReaderAsChunkFiller<B, R, T>
where
    B: BorrowMut<R>,
    R: TrBuffReader<T>,
    T: Clone,
{
    _use_r_: PhantomData<R>,
    _use_t_: PhantomData<[T]>,
    reader_: B,
}

impl<B, R, T> ReaderAsChunkFiller<B, R, T>
where
    B: BorrowMut<R>,
    R: TrBuffReader<T>,
    T: Clone,
{
    pub const fn new(reader: B) -> Self {
        ReaderAsChunkFiller {
            _use_r_: PhantomData,
            _use_t_: PhantomData,
            reader_: reader,
        }
    }

    pub fn can_fill(&mut self) -> bool {
        self.reader_.borrow_mut().can_read()
    }

    pub fn fill_async<'a>(
        &'a mut self,
        target: &'a mut [T],
    ) -> ReaderFillAsync<'a, B, R, T> {
        ReaderFillAsync::new(self, target)
    }
}

impl<B, R, T> TrChunkFiller<T> for ReaderAsChunkFiller<B, R, T>
where
    B: BorrowMut<R>,
    R: TrBuffReader<T>,
    T: Clone,
{
    type FillError = IoAbortReport<<R as TrBuffReader<T>>::Error>;
    type FillAsync<'a> = ReaderFillAsync<'a, B, R, T> where Self: 'a;

    #[inline(always)]
    fn can_fill(&mut self) -> bool {
        ReaderAsChunkFiller::can_fill(self)
    }

    #[inline(always)]
    fn fill_async<'a>(
        &'a mut self,
        target: &'a mut [T],
    ) -> Self::FillAsync<'a> {
        ReaderAsChunkFiller::fill_async(self, target)
    }
}

pub struct ReaderFillAsync<'a, B, R, T>
where
    B: BorrowMut<R>,
    R: TrBuffReader<T>,
    T: Clone,
{
    filler_: &'a mut ReaderAsChunkFiller<B, R, T>,
    target_: &'a mut [T],
}

impl<'a, B, R, T> ReaderFillAsync<'a, B, R, T>
where
    B: BorrowMut<R>,
    R: TrBuffReader<T>,
    T: Clone,
{
    pub fn new(
        filler: &'a mut ReaderAsChunkFiller<B, R, T>,
        target: &'a mut [T],
    ) -> Self {
        ReaderFillAsync {
            filler_: filler,
            target_: target,
        }
    }

    pub fn may_cancel_with<C>(
        self,
        cancel: Pin<&'a mut C>,
    ) -> ReaderFillFuture<'a, C, B, R, T>
    where
        C: TrCancellationToken,
    {
        ReaderFillFuture::new(self.filler_, self.target_, cancel)
    }
}

impl<'a, B, R, T> IntoFuture for ReaderFillAsync<'a, B, R, T>
where
    B: BorrowMut<R>,
    R: TrBuffReader<T>,
    T: Clone,
{
    type IntoFuture = ReaderFillFuture<'a, NonCancellableToken, B, R, T>;
    type Output = <Self::IntoFuture as Future>::Output;

    fn into_future(self) -> Self::IntoFuture {
        let cancel = NonCancellableToken::pinned();
        ReaderFillFuture::new(self.filler_, self.target_, cancel)
    }
}

impl<'a, B, R, T> TrIntoFutureMayCancel<'a> for ReaderFillAsync<'a, B, R, T>
where
    B: BorrowMut<R>,
    R: TrBuffReader<T>,
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
        ReaderFillAsync::may_cancel_with(self, cancel)
    }
}

#[pin_project]
pub struct ReaderFillFuture<'a, C, B, R, T>
where
    C: TrCancellationToken,
    B: BorrowMut<R>,
    R: TrBuffReader<T>,
    T: Clone,
{
    #[pin]filler_: &'a mut ReaderAsChunkFiller<B, R, T>,
    #[pin]target_: &'a mut [T],
    cancel_: Pin<&'a mut C>,
}

impl<'a, C, B, R, T> ReaderFillFuture<'a, C, B, R, T>
where
    C: TrCancellationToken,
    B: BorrowMut<R>,
    R: TrBuffReader<T>,
    T: Clone,
{
    pub fn new(
        filler: &'a mut ReaderAsChunkFiller<B, R, T>,
        target: &'a mut [T],
        cancel: Pin<&'a mut C>,
    ) -> Self {
        ReaderFillFuture {
            filler_: filler,
            target_: target,
            cancel_: cancel,
        }
    }

    async fn fill_async_(
        self: Pin<&mut Self>,
    ) -> Result<usize, IoAbortReport<<R as TrBuffReader<T>>::Error>> {
        let mut this = self.project();
        let mut filler = this.filler_.as_mut();
        let reader = filler.reader_.borrow_mut();
        let mut target = this.target_.as_mut();
        let target_len = target.len();
        let mut perform_len = 0usize;
        loop {
            if perform_len >= target_len {
                break Result::Ok(perform_len);
            }
            let req_len = target_len - perform_len;

            #[cfg(test)]
            log::trace!(
                "[ReaderFillFuture::fill_async_] \
                target_len({target_len}), perform_len({perform_len})"
            );
            let r = reader
                .read_async(req_len)
                .may_cancel_with(this.cancel_.as_mut())
                .await;
            let Result::Ok(src) = r else {
                let Result::Err(last_error) = r else {
                    unreachable!("[ReaderFillFuture::fill_async_]")
                };
                let report = IoAbortReport::new(perform_len, last_error);
                break Result::Err(report);
            };
            let src_len = src.len();

            #[cfg(test)]
            log::trace!(
                "[ReaderFillFuture::fill_async_] read_async src_len({src_len})"
            );
            debug_assert!(src_len <= req_len);
            debug_assert!(src_len + perform_len <= target_len);
            let dst = &mut target[perform_len..perform_len + src_len];
            dst.clone_from_slice(&src);
            perform_len += src_len;
            drop(src);
        }
    }
}

impl<'a, C, B, R, T> Future for ReaderFillFuture<'a, C, B, R, T>
where
    C: TrCancellationToken,
    B: BorrowMut<R>,
    R: TrBuffReader<T>,
    T: Clone,
{
    type Output = Result<
        usize,
        IoAbortReport<<R as TrBuffReader<T>>::Error>,
    >;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let f = self.fill_async_();
        pin_mut!(f);
        f.poll(cx)
    }
}

