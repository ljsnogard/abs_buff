use core::mem::MaybeUninit;

use abs_iter::{TrMutSliceLike, TrSliceLike};
use crate::{BuffSegmRefAsInput, BuffSegmMutAsOutput, Demand, TrInput, TrOutput};

pub trait TrBuffSegmView {
    type Item: Sized;

    /// Returns true if no available items to consume, false otherwise.
    fn is_empty(&self) -> bool;

    /// The items count of the unconsumed part of the segment.
    fn len(&self) -> usize;

    /// Returns the capacity of the segment, no matter the elements are
    /// consumed or not.
    fn capacity(&self) -> usize;

    /// Iterate over the elements of the internal buffer retained by the segment
    /// and retrieve as pointers.
    fn iter_ptr(&self) -> impl Iterator<Item = *const Self::Item>;
}

pub trait TrBuffSegmRef<T>
where
    Self: TrBuffSegmView<Item = T>,
{
    type Slice<'a>: TrSliceLike<Elem = T>
    where
        T: 'a,
        Self: 'a;

    type Segm<'a>: TrBuffSegmRef<T>
    where
        T: 'a,
        Self: 'a;

    /// Take a slice starting from the beginning out of this segment, length
    /// specified by the demand argument, reducing the length of this segment
    /// when the taken slice drops.
    fn take_segm_ref<'a>(
        &'a mut self,
        length: Demand<usize>,
    ) -> Option<Self::Segm<'a>>;

    /// Iterate the unconsumed slices.
    fn iter_slices<'a>(&'a mut self) -> impl IntoIterator<Item = Self::Slice<'a>>
    where
        T: 'a;
    fn as_input(&mut self) -> impl TrInput<T> 
    where
        Self: Sized,
    {
        BuffSegmRefAsInput::<&mut Self, Self, T>::new(self)
    }
}

pub trait TrBuffSegmMut<T>
where
    Self: TrBuffSegmView<Item = MaybeUninit<T>>,
{
    type Slice<'a>: TrMutSliceLike<Elem = MaybeUninit<T>>
    where
        T: 'a,
        Self: 'a;

    type Segm<'a>: TrBuffSegmMut<T>
    where
        T: 'a,
        Self: 'a;

    /// Take a slice starting from the beginning out of this segment, length
    /// specified by the demand argument, reducing the length of this segment
    /// when the taken slice drops.
    fn take_segm_mut<'a>(
        &'a mut self, 
        length: Demand<usize>,
    ) -> Option<Self::Segm<'a>>;

    fn iter_slices<'a>(&'a mut self) -> impl IntoIterator<Item = Self::Slice<'a>>
    where
        T: 'a;

    fn as_output(&mut self) -> impl TrOutput<T>
    where
        Self: Sized,
    {
        BuffSegmMutAsOutput::<&mut Self, Self, T>::new(self)
    }
}
