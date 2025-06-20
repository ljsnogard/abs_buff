use core::error::Error;

use abs_sync::may_cancel::TrMayCancel;
use anylr::SomeOf;

use crate::{TrBuffSegmRef, TrInput};

/// Buffer that will borrow zero or more segments for data observation without
/// consuming them.
pub trait TrBuffPeek<T = u8> {
    type PeekerSegm<'a>: TrBuffSegmRef<T>
    where
        Self: 'a;

    type PeekAsync<'a>: TrMayCancel<'a,
        MayCancelOutput = SomeOf<Self::PeekerSegm<'a>, Self::Err>>
    where
        Self: 'a;

    type Err: Error;

    /// Lend some slices for peeking. The number and the length of the slices 
    /// to peek are decided by the buffer.
    fn peek_async(&mut self) -> Self::PeekAsync<'_>;

    fn as_intput(&mut self) -> impl TrInput<T>;
}

pub trait TrBuffTryPeek<T = u8>: TrBuffPeek<T> {
    fn try_peek(&mut self) -> SomeOf<Self::PeekerSegm<'_>, Self::Err>;
}
