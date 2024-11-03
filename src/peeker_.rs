use core::{
    error::Error,
    iter::IntoIterator,
    ops::Deref,
};

use abs_sync::cancellation::TrIntoFutureMayCancel;

/// Buffer that will lend zero or more slices for peeking.
pub trait TrBuffIterPeek<T = u8>
where
    T: Clone,
{
    type SliceRef<'a>: Deref<Target = [T]> where Self: 'a;
    type BuffIter<'a>: IntoIterator<Item = Self::SliceRef<'a>> where Self: 'a;
    type Err: Error;

    type PeekAsync<'a>: TrIntoFutureMayCancel<'a, MayCancelOutput =
        Result<Self::BuffIter<'a>, Self::Err>>
    where
        Self: 'a;

    /// Lend some slices for peeking. The number and the length of the slices 
    /// to peek are decided by the buffer.
    fn peek_async(&mut self) -> Self::PeekAsync<'_>;
}
