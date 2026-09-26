use core::{
    error::Error,
    fmt,
    future::Future,
    marker::PhantomPinned,
    mem::MaybeUninit,
    pin::Pin,
    slice,
    task::{Context, Poll},
};

use abs_cancel::{TrCancellationToken, TrMayCancel};
use anylr::SomeOf;

use crate::{
    Demand, TrBuffRead, TrBuffTryRead, TrBuffTryWrite, TrBuffWrite,
    buffer::{
        SegmMut, SegmReclaim, SegmRef,
        TrBuffSegmMut, TrBuffSegmRef, TrBuffSegmView,
        TrConsumerState, TrProducerState,
    },
    error::{ReadErrTag, TrErrTag, TrTaggedError, WriteErrTag},
};

/// Error returned when a borrowed byte slice is empty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorrowedSliceError<TyTag>
where
    TyTag: TrErrTag,
{
    Empty(TyTag),
}

impl<TyTag> fmt::Display for BorrowedSliceError<TyTag>
where
    TyTag: TrErrTag,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BorrowedSliceError::Empty(tag) => {
                write!(f, "{}: borrowed byte slice is empty", tag)
            }
        }
    }
}

impl<TyTag: TrErrTag> Error for BorrowedSliceError<TyTag> {}

impl<TyTag> TrTaggedError<TyTag> for BorrowedSliceError<TyTag>
where
    TyTag: TrErrTag,
{
    fn err_tag(&self) -> TyTag {
        match self {
            BorrowedSliceError::Empty(tag) => *tag,
        }
    }
}

/// 一个「立即就绪」的 future，产出 [`SomeOf<S, E>`]。
///
/// 本类型既服务于本 crate 的切片实现（`TrInput` / `TrOutput` 的异步方法直接
/// 返回它），也作为**公开工具**提供给下游实现者：任何自定义 `TrInput` /
/// `TrOutput` 都可以在同步就绪的场合把它当作 `ReadAsync<'f>` /
/// `WriteAsync<'f>` 的具体类型，而不必自己再写一个 `Future`。
///
/// # Panics
///
/// [`Future::poll`] 在同一个实例上第二次被调用时会 panic：它只承载「一次就绪
/// 结果」，被 poll 一次后内部值已被取走。正常使用（`await` 一次）不会触发。
pub struct ReadySegm<S, E>(Option<SomeOf<S, E>>);

impl<S, E> ReadySegm<S, E> {
    /// 用一个已经就绪的结果构造本 future。
    ///
    /// # Examples
    ///
    /// ```
    /// use abs_buff::{ReadySegm, x_deps::anylr::SomeOf};
    ///
    /// let ready: ReadySegm<usize, ()> = ReadySegm::new(SomeOf::new_left(3usize));
    /// ```
    pub fn new(value: SomeOf<S, E>) -> Self {
        ReadySegm(Option::Some(value))
    }
}

impl<S, E> Future for ReadySegm<S, E> {
    type Output = SomeOf<S, E>;

    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        Poll::Ready(this.0.take().expect("a ready future must be polled once"))
    }
}

impl<'a, S, E> TrMayCancel<'a> for ReadySegm<S, E>
where
    S: 'a,
    E: 'a,
{
    type MayCancelFuture<'f, C> = ReadySegm<S, E>
    where
        'f: 'a,
        Self: 'f,
        C: 'f + TrCancellationToken;

    type MayCancelOutput = SomeOf<S, E>;

    fn may_cancel_with<C>(self, _: C) -> Self::MayCancelFuture<'a, C>
    where
        C: 'a + TrCancellationToken,
    {
        self
    }
}

// ---------------------------------------------------------------------------
// Advancing a borrowed slice when a segment is reclaimed.
//
// The blanket implementations cover every `Borrow<[u8]>` / `BorrowMut<[u8]>`
// type.  The default does not mutate the underlying value, while the slice
// reference specializations mirror `std::io::Read for &[u8]` and
// `std::io::Write for &mut [u8]` by advancing the reference itself.
// ---------------------------------------------------------------------------

trait TrReadAdvance {
    fn advance_slice(&mut self, amount: usize);
}

impl<T> TrReadAdvance for &[T] {
    fn advance_slice(&mut self, amount: usize) {
        let old = *self;
        *self = &old[amount..];
    }
}

impl<T> TrReadAdvance for &mut [T] {
    fn advance_slice(&mut self, amount: usize) {
        let old = core::mem::take(self);
        *self = &mut old[amount..];
    }
}

trait TrWriteAdvance {
    fn advance_slice(&mut self, amount: usize);
}

impl<T> TrWriteAdvance for &mut [T] {
    fn advance_slice(&mut self, amount: usize) {
        let old = core::mem::take(self);
        *self = &mut old[amount..];
    }
}

fn advance_read<T>(mut src: &[T], amount: usize) {
    TrReadAdvance::advance_slice(&mut src, amount);
}

fn advance_write<T>(mut dst: &mut [T], amount: usize) {
    TrWriteAdvance::advance_slice(&mut dst, amount);
}

// ---------------------------------------------------------------------------
// Read segment over a `T: Borrow<[u8]>`
// ---------------------------------------------------------------------------

pub struct BorrowedReadSegm<'a, T> {
    source_: &'a [T],
    offset_: usize,
    end_: usize,
    _pinned_: PhantomPinned,
}

impl<'a, T> BorrowedReadSegm<'a, T> {
    fn with_limit(source: &'a [T], max: Option<usize>) -> Self {
        let len = source.len();
        let end_ = match max {
            Option::Some(m) if m < len => m,
            _ => len,
        };
        BorrowedReadSegm {
            source_: source,
            offset_: 0,
            end_,
            _pinned_: PhantomPinned,
        }
    }

    fn remaining(&self) -> &[T] {
        &self.source_[self.offset_..self.end_]
    }

    fn as_segm_ref<'f>(&'f mut self) -> SegmRef<'f, T, SegmReclaim<'f>> {
        let data = &self.source_[self.offset_..self.end_];
        SegmRef::new(
            data,
            SegmReclaim::new(Pin::new(&mut self.offset_))
        )
    }

    fn take_segm_ref<'f>(
        &'f mut self,
        demand: &Demand<usize>,
    ) -> Option<SegmRef<'f, T, SegmReclaim<'f>>> {
        let c = self.end_ - self.offset_;
        if c == 0 {
            return Option::None;
        }
        let available = Demand::less_than(c);
        let agreement = demand.compromise(&available)?;
        let max_len = *agreement.max()?;
        let data = &self.source_[self.offset_..self.offset_ + max_len];
        let reclaim = SegmReclaim::new(Pin::new(&mut self.offset_));
        Option::Some(SegmRef::new(data, reclaim))
    }
}

impl<T> Drop for BorrowedReadSegm<'_, T> {
    fn drop(&mut self) {
        advance_read(self.source_, self.offset_);
    }
}

impl<T> TrBuffSegmView for BorrowedReadSegm<'_, T> {
    type SlicesIter<'f> = Option<&'f [T]> where Self: 'f;
    type Item = T;

    #[inline]
    fn is_empty(&self) -> bool {
        self.remaining().is_empty()
    }

    #[inline]
    fn least_count(&self) -> usize {
        self.remaining().len()
    }

    fn iter_slices(&self) -> Self::SlicesIter<'_> {
        let data = self.remaining();
        if data.is_empty() {
            Option::None
        } else {
            Option::Some(data)
        }
    }
}

impl<'a, T> TrBuffSegmRef<'a, T> for BorrowedReadSegm<'a, T> {
    type Reclaimer<'f> = SegmReclaim<'f> where Self: 'f;

    type TakeSegmRef<'f> = Option<SegmRef<'f, T, SegmReclaim<'f>>>
        where Self: 'f;

    #[inline]
    fn take_segm_ref<'f>(
        &'f mut self,
        demand: &Demand<usize>,
    ) -> Self::TakeSegmRef<'f> {
        BorrowedReadSegm::take_segm_ref(self, demand)
    }

    #[inline]
    fn as_segm_ref<'f>(&'f mut self) -> SegmRef<'f, T, Self::Reclaimer<'f>> {
        BorrowedReadSegm::as_segm_ref(self)
    }
}

// ---------------------------------------------------------------------------
// Write segment over a `&mut [T]`
// ---------------------------------------------------------------------------

pub struct BorrowedWriteSegm<'a, T> {
    target_: &'a mut [T],
    offset_: usize,
    end_: usize,
    _pinned: PhantomPinned,
}

impl<'a, T> BorrowedWriteSegm<'a, T> {
    fn with_limit(target: &'a mut [T], max: Option<usize>) -> Self {
        let len = target.len();
        let end_ = match max {
            Option::Some(m) if m < len => m,
            _ => len,
        };
        BorrowedWriteSegm {
            target_: target,
            offset_: 0,
            end_,
            _pinned: PhantomPinned,
        }
    }

    fn remaining(&self) -> &[MaybeUninit<T>] {
        let bytes = &self.target_[self.offset_..self.end_];
        // SAFETY: `MaybeUninit<T>` has the same layout as `T`, and the
        // slice lifetime is tied to the underlying borrowed bytes.
        unsafe {
            slice::from_raw_parts(
                bytes.as_ptr().cast::<MaybeUninit<T>>(),
                bytes.len(),
            )
        }
    }

    fn as_segm_mut<'f>(&'f mut self) -> SegmMut<'f, T, SegmReclaim<'f>> {
        let bytes = &mut self.target_[self.offset_..self.end_];
        // SAFETY: `MaybeUninit<T>` has the same layout as `T`, and the
        // mutable slice is exclusively borrowed from `T`.
        let data = unsafe {
            slice::from_raw_parts_mut(
                bytes.as_mut_ptr().cast::<MaybeUninit<T>>(),
                bytes.len(),
            )
        };
        let p_offs = Pin::new(&mut self.offset_);
        SegmMut::new(data, SegmReclaim::new(p_offs))
    }

    fn take_segm_mut<'f>(
        &'f mut self,
        demand: &Demand<usize>,
    ) -> Option<SegmMut<'f, T, SegmReclaim<'f>>> {
        let c = self.end_ - self.offset_;
        if c == 0 {
            return Option::None;
        }
        let available = Demand::less_than(c);
        let agreement = demand.compromise(&available)?;
        let max_len = *agreement.max()?;
        let data = &mut self.target_[self.offset_..self.offset_ + max_len];
        let data = unsafe {
            slice::from_raw_parts_mut(
                data.as_mut_ptr().cast::<MaybeUninit<T>>(),
                data.len(),
            )
        };
        let reclaim = SegmReclaim::new(Pin::new(&mut self.offset_));
        Option::Some(SegmMut::new(data, reclaim))
    }
}

impl<T> Drop for BorrowedWriteSegm<'_, T> {
    fn drop(&mut self) {
        advance_write(self.target_, self.offset_);
    }
}

impl<T> TrBuffSegmView for BorrowedWriteSegm<'_, T> {
    type SlicesIter<'f> = Option<&'f [MaybeUninit<T>]> where Self: 'f;
    type Item = MaybeUninit<T>;

    #[inline]
    fn is_empty(&self) -> bool {
        self.remaining().is_empty()
    }

    #[inline]
    fn least_count(&self) -> usize {
        self.remaining().len()
    }

    fn iter_slices(&self) -> Self::SlicesIter<'_> {
        let data = self.remaining();
        if data.is_empty() {
            Option::None
        } else {
            Option::Some(data)
        }
    }
}

impl<'a, T> TrBuffSegmMut<'a, T> for BorrowedWriteSegm<'a, T> {
    type Reclaimer<'f> = SegmReclaim<'f> where Self: 'f;

    type TakeSegmMut<'f> = Option<SegmMut<'f, T, SegmReclaim<'f>>> where Self: 'f;

    #[inline]
    fn take_segm_mut<'f>(
        &'f mut self,
        demand: &Demand<usize>,
    ) -> Self::TakeSegmMut<'f> {
        BorrowedWriteSegm::take_segm_mut(self, demand)
    }

    #[inline]
    fn as_segm_mut<'f>(&'f mut self) -> SegmMut<'f, T, Self::Reclaimer<'f>> {
        BorrowedWriteSegm::as_segm_mut(self)
    }
}

// ---------------------------------------------------------------------------
// Blanket impls
// ---------------------------------------------------------------------------

impl<T> TrConsumerState for &[T] {
    fn consumer_state(&self) -> Option<(usize, bool)> {
        Option::Some((self.len(), self.is_empty()))
    }
}

impl<T> TrBuffTryRead<T> for &[T] {
    type SegmRef<'f> = BorrowedReadSegm<'f, T> where Self: 'f;

    type Err = BorrowedSliceError<ReadErrTag>;

    fn try_read<'f>(
        &'f mut self,
        demand: &Demand<usize>,
    ) -> SomeOf<Self::SegmRef<'f>, Self::Err> {
        let len = self.len();
        let min_len = demand.min().copied().unwrap_or(0);
        if len == 0 || len < min_len {
            let err = BorrowedSliceError::Empty(ReadErrTag::Closing);
            return SomeOf::new_right(err);
        }
        let max_len = demand.max().copied();
        SomeOf::new_left(BorrowedReadSegm::with_limit(self, max_len))
    }
}

impl<T> TrBuffRead<T> for &[T] {
    type ReadAsync<'f> = ReadySegm<Self::SegmRef<'f>, Self::Err> where Self: 'f;

    fn read_async<'f>(
        &'f mut self,
        demand: &Demand<usize>,
    ) -> Self::ReadAsync<'f> {
        let len = self.len();
        let min_len = demand.min().copied().unwrap_or(0);
        if len == 0 || len < min_len {
            return ReadySegm::new(SomeOf::new_right(
                BorrowedSliceError::Empty(ReadErrTag::Closing),
            ));
        }
        let max_len = demand.max().copied();
        ReadySegm::new(SomeOf::new_left(BorrowedReadSegm::with_limit(
            self, max_len,
        )))
    }
}

// -- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ----
// -- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ----

impl<T> TrConsumerState for &mut [T] {
    fn consumer_state(&self) -> Option<(usize, bool)> {
        Option::Some((self.len(), self.is_empty()))
    }
}

impl<T> TrBuffTryRead<T> for &mut [T] {
    type SegmRef<'f> = BorrowedReadSegm<'f, T> where Self: 'f;

    type Err = BorrowedSliceError<ReadErrTag>;

    fn try_read<'f>(
        &'f mut self,
        demand: &Demand<usize>,
    ) -> SomeOf<Self::SegmRef<'f>, Self::Err> {
        let len = self.len();
        let min_len = demand.min().copied().unwrap_or(0);
        if len == 0 || len < min_len {
            let err = BorrowedSliceError::Empty(ReadErrTag::Closing);
            return SomeOf::new_right(err);
        }
        let max_len = demand.max().copied();
        SomeOf::new_left(BorrowedReadSegm::with_limit(self, max_len))
    }
}

impl<T> TrBuffRead<T> for &mut [T] {
    type ReadAsync<'f> = ReadySegm<Self::SegmRef<'f>, Self::Err> where Self: 'f;

    fn read_async<'f>(
        &'f mut self,
        demand: &Demand<usize>,
    ) -> Self::ReadAsync<'f> {
        let len = self.len();
        let min_len = demand.min().copied().unwrap_or(0);
        if len == 0 || len < min_len {
            return ReadySegm::new(SomeOf::new_right(
                BorrowedSliceError::Empty(ReadErrTag::Closing),
            ));
        }
        let max_len = demand.max().copied();
        ReadySegm::new(SomeOf::new_left(BorrowedReadSegm::with_limit(
            self, max_len,
        )))
    }
}

// -- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ----
// -- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ----

impl<T> TrProducerState for &mut [T] {
    fn producer_state(&self) -> Option<(usize, bool)> {
        Option::Some((self.len(), self.is_empty()))
    }
}

impl<T> TrBuffTryWrite<T> for &mut [T] {
    type SegmMut<'f> = BorrowedWriteSegm<'f, T> where Self: 'f;

    type Err = BorrowedSliceError<WriteErrTag>;

    fn try_write<'f>(
        &'f mut self,
        demand: &Demand<usize>,
    ) -> SomeOf<Self::SegmMut<'f>, Self::Err> {
        let len = self.len();
        let min_len = demand.min().copied().unwrap_or(0);
        if len == 0 || len < min_len {
            let err = BorrowedSliceError::Empty(WriteErrTag::Closing);
            return SomeOf::new_right(err);
        }
        let max_len = demand.max().copied();
        SomeOf::new_left(BorrowedWriteSegm::with_limit(self, max_len))
    }
}

impl<T> TrBuffWrite<T> for &mut [T] {
    type WriteAsync<'f> = ReadySegm<Self::SegmMut<'f>, Self::Err>
    where
        Self: 'f;

    fn write_async<'f>(
        &'f mut self,
        demand: &Demand<usize>,
    ) -> Self::WriteAsync<'f> {
        let len = self.len();
        let min_len = demand.min().copied().unwrap_or(0);
        if len == 0 || len < min_len {
            return ReadySegm::new(SomeOf::new_right(
                BorrowedSliceError::Empty(WriteErrTag::Closing),
            ));
        }
        let max_len = demand.max().copied();
        ReadySegm::new(SomeOf::new_left(BorrowedWriteSegm::with_limit(
            self, max_len,
        )))
    }
}


#[cfg(test)]
mod tests_ {
    use super::*;

    #[test]
    fn read_borrowed_slice_advances_like_std() {
        let mut data: &[u8] = b"hello";

        let demand = Demand::less_than(5);
        let mut segm = data
            .try_read(&demand)
            .pick_left()
            .expect("read should return a segment");
        let mut child = segm.as_segm_ref();
        let mut dst = [MaybeUninit::<u8>::uninit(); 2];
        let n = unsafe { child.move_items_to_buff(&mut dst) };
        assert_eq!(n, 2);
        drop(child);
        drop(segm);

        assert_eq!(data, b"llo");
        assert!(!data.consumer_state().is_none_or(|(c, b)| c == 0 && b));
    }

    #[test]
    fn read_borrowed_mut_slice_advances() {
        let mut storage = [1u8, 2, 3, 4];
        let mut data: &mut [u8] = &mut storage;

        let demand = Demand::less_than(4);
        let mut segm = data
            .try_read(&demand)
            .pick_left()
            .expect("read should return a segment");
        let mut child = segm.as_segm_ref();
        let mut dst = [MaybeUninit::<u8>::uninit(); 3];
        let n = unsafe { child.move_items_to_buff(&mut dst) };
        assert_eq!(n, 3);
        drop(child);
        drop(segm);

        assert_eq!(data, &[4u8][..]);
    }

    #[test]
    fn write_borrowed_mut_slice_advances_like_std() {
        let mut storage = [0u8; 5];
        {
            let mut data: &mut [u8] = &mut storage;
            let demand = Demand::less_than(5);
            let mut segm = data
                .try_write(&demand)
                .pick_left()
                .expect("write should return a segment");
            let mut child = segm.as_segm_mut();
            let src = [
                MaybeUninit::new(b'a'),
                MaybeUninit::new(b'b'),
                MaybeUninit::new(b'c'),
            ];
            let n = unsafe { child.move_items_from_buff(&mut src.clone()) };
            assert_eq!(n, 3);
            drop(child);
            drop(segm);

            // `&mut [u8]` advances past the written prefix, exactly like
            // `std::io::Write for &mut [u8]`.
            assert_eq!(data, [0u8, 0]);
        }
        assert_eq!(&storage[..3], b"abc");
    }
}
