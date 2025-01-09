use core::{
    borrow::Borrow,
    error::Error,
    iter::IntoIterator,
};

use abs_sync::cancellation::TrIntoFutureMayCancel;

/// Buffer that will borrow zero or more slices for peeking.
pub trait TrBuffIterPeek<T = u8> {
    type SegmRef<'a>: 'a + Borrow<[T]>
    where
        T: 'a,
        Self: 'a;

    type BuffIter<'a>: IntoIterator<Item = Self::SegmRef<'a>>
    where
        T: 'a,
        Self: 'a;

    type PeekAsync<'a>: TrIntoFutureMayCancel<'a, MayCancelOutput =
        Result<Self::BuffIter<'a>, Self::Err>>
    where
        T: 'a,
        Self: 'a;

    type Err: Error;

    /// Lend some slices for peeking. The number and the length of the slices 
    /// to peek are decided by the buffer.
    fn peek_async(&mut self) -> Self::PeekAsync<'_>;
}

pub trait TrBuffIterTryPeek<T = u8>: TrBuffIterPeek<T> {
    fn try_peek(&mut self) -> Result<Self::BuffIter<'_>, Self::Err>;
}
