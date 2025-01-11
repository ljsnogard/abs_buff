use core::{
    borrow::Borrow,
    error::Error,
    iter::IntoIterator,
};

use abs_sync::cancellation::TrIntoFutureMayCancel;

/// Buffer that will borrow zero or more segments for data observation without
/// consuming them.
pub trait TrBuffIterPeek<T = u8> {
    type SegmRef<'a>: Borrow<[T]>
    where
        Self: 'a;

    type Segments<'a>: IntoIterator<Item = Self::SegmRef<'a>>
    where
        Self: 'a;

    type PeekAsync<'a>: TrIntoFutureMayCancel<MayCancelOutput =
        Result<Self::Segments<'a>, Self::Err>>
    where
        Self: 'a;

    type Err: Error;

    /// Lend some slices for peeking. The number and the length of the slices 
    /// to peek are decided by the buffer.
    fn peek_async(&mut self) -> Self::PeekAsync<'_>;
}

pub trait TrBuffIterTryPeek<T = u8>: TrBuffIterPeek<T> {
    fn try_peek(&mut self) -> Result<Self::Segments<'_>, Self::Err>;
}
