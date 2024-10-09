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
    utils::{IoAbortReport, TrChunkWriter},
    TrBuffWriter,
};

pub struct ChunkWriter<B, W, T>
where
    B: BorrowMut<W>,
    W: TrBuffWriter<T>,
    T: Clone,
{
    _use_w_: PhantomData<W>,
    _use_t_: PhantomData<[T]>,
    writer_: B,
}

impl<B, W, T> ChunkWriter<B, W, T>
where
    B: BorrowMut<W>,
    W: TrBuffWriter<T>,
    T: Clone,
{
    pub const fn new(writer: B) -> Self {
        ChunkWriter {
            _use_w_: PhantomData,
            _use_t_: PhantomData,
            writer_: writer,
        }
    }

    pub fn can_write(&mut self) -> bool {
        self.writer_.borrow_mut().can_write()
    }

    pub fn write_async<'a>(
        &'a mut self,
        source: &'a [T],
    ) -> WriteChunkAsync<'a, B, W, T> {
        WriteChunkAsync::new(self, source)
    }
}

impl<B, W, T> TrChunkWriter<T> for ChunkWriter<B, W, T>
where
    B: BorrowMut<W>,
    W: TrBuffWriter<T>,
    T: Clone,
{
    type WriterError = IoAbortReport<<W as TrBuffWriter<T>>::Error>;
    type WriteAsync<'a> = WriteChunkAsync<'a, B, W, T> where Self: 'a;

    #[inline(always)]
    fn can_write(&mut self) -> bool {
        ChunkWriter::can_write(self)
    }

    #[inline(always)]
    fn write_async<'a>(&'a mut self, source: &'a [T]) -> Self::WriteAsync<'a> {
        ChunkWriter::write_async(self, source)
    }
}

pub struct WriteChunkAsync<'a, B, W, T>
where
    B: BorrowMut<W>,
    W: TrBuffWriter<T>,
    T: Clone,
{
    writer_: &'a mut ChunkWriter<B, W, T>,
    source_: &'a [T],
}

impl<'a, B, W, T> WriteChunkAsync<'a, B, W, T>
where
    B: BorrowMut<W>,
    W: TrBuffWriter<T>,
    T: Clone,
{
    pub fn new(
        writer: &'a mut ChunkWriter<B, W, T>,
        source: &'a [T],
    ) -> Self {
        WriteChunkAsync {
            writer_: writer,
            source_: source,
        }
    }

    pub fn may_cancel_with<C>(
        self,
        cancel: Pin<&'a mut C>,
    ) -> WriterChunkFuture<'a, C, B, W, T>
    where
        C: TrCancellationToken,
    {
        WriterChunkFuture::new(self.writer_, self.source_, cancel)
    }
}

impl<'a, B, W, T> IntoFuture for WriteChunkAsync<'a, B, W, T>
where
    B: BorrowMut<W>,
    W: TrBuffWriter<T>,
    T: Clone,
{
    type IntoFuture = WriterChunkFuture<'a, NonCancellableToken, B, W, T>;
    type Output = <Self::IntoFuture as Future>::Output;

    fn into_future(self) -> Self::IntoFuture {
        let cancel = NonCancellableToken::pinned();
        WriterChunkFuture::new(self.writer_, self.source_, cancel)
    }
}

impl<'a, B, W, T> TrIntoFutureMayCancel<'a> for WriteChunkAsync<'a, B, W, T>
where
    B: BorrowMut<W>,
    W: TrBuffWriter<T>,
    T: Clone,
{
    type MayCancelOutput = <<Self as IntoFuture>::IntoFuture as Future>::Output;

    #[inline(always)]
    fn may_cancel_with<C>(
        self,
        cancel: Pin<&'a mut C>,
    ) -> impl Future<Output = Self::MayCancelOutput>
    where
        C: TrCancellationToken,
    {
        WriteChunkAsync::may_cancel_with(self, cancel)
    }
}

#[pin_project]
pub struct WriterChunkFuture<'a, C, Bw, Tw, T>
where
    C: TrCancellationToken,
    Bw: BorrowMut<Tw>,
    Tw: TrBuffWriter<T>,
    T: Clone,
{
    #[pin]writer_: &'a mut ChunkWriter<Bw, Tw, T>,
    source_: &'a [T],
    cancel_: Pin<&'a mut C>,
}

impl<'a, C, Bw, Tw, T> WriterChunkFuture<'a, C, Bw, Tw, T>
where
    C: TrCancellationToken,
    Bw: BorrowMut<Tw>,
    Tw: TrBuffWriter<T>,
    T: Clone,
{
    pub fn new(
        writer: &'a mut ChunkWriter<Bw, Tw, T>,
        source: &'a [T],
        cancel: Pin<&'a mut C>,
    ) -> Self {
        WriterChunkFuture {
            writer_: writer,
            source_: source,
            cancel_: cancel,
        }
    }

    async fn write_chunk_async_(
        self: Pin<&mut Self>,
    ) -> Result<usize, IoAbortReport<<Tw as TrBuffWriter<T>>::Error>> {
        let mut this = self.project();
        let mut writer = this.writer_.as_mut();
        let writer = writer.writer_.borrow_mut();
        let source = *this.source_;
        let source_len = source.len();
        let mut perform_len = 0usize;
        loop {
            if perform_len >= source_len {
                break Result::Ok(perform_len);
            }
            let req_len = source_len - perform_len;

            #[cfg(test)]
            log::trace!(
                "[WriterCopyFuture::copy_async_] \
                source_len({source_len}), perform_len({perform_len})"
            );

            let w = writer
                .write_async(req_len)
                .may_cancel_with(this.cancel_.as_mut())
                .await;
            let Result::Ok(mut dst) = w else {
                let Result::Err(last_error) = w else {
                    unreachable!("[WriterCopyFuture::copy_async_]")
                };
                let report = IoAbortReport::new(perform_len, last_error);
                break Result::Err(report);
            };
            let dst_len = dst.len();
            debug_assert!(dst_len + perform_len <= source_len);
            let src = &source[perform_len..perform_len + dst_len];
            dst.clone_from_slice(src);
            perform_len += dst_len;
            drop(dst);
        }
    }
}

impl<'a, C, Bw, Tw, T> Future for WriterChunkFuture<'a, C, Bw, Tw, T>
where
    C: TrCancellationToken,
    Bw: BorrowMut<Tw>,
    Tw: TrBuffWriter<T>,
    T: Clone,
{
    type Output = Result<usize, IoAbortReport<<Tw as TrBuffWriter<T>>::Error>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let f = self.write_chunk_async_();
        pin_mut!(f);
        f.poll(cx)
    }
}
