use core::{
    borrow::BorrowMut,
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
};

use crate::{BuffSegmRefInput, BuffSegmMutOutput, TrInput, TrOutput};

pub trait TrBuffSegmView {
    type Item: Sized;

    /// Returns if the elements are all consumed, or never existing.
    fn is_empty(&self) -> bool;

    /// The items count of the unconsumed part of the segment.
    fn len(&self) -> usize;

    /// Returns the capacity of the segment, no matter the elements are
    /// consumed or not. This is usually used by the reclaim function.
    fn capacity(&self) -> usize;

    /// Iterate over the elements of the internal buffer retained by the segment
    /// and retrieve as pointers.
    fn iter_ptr(&self) -> impl Iterator<Item = *const Self::Item>;
}

pub trait TrBuffSegmRef<T>
where
    Self: TrBuffSegmView<Item = T>,
{
    type Slice<'a>: Deref<Target = [Self::Item]>
    where
        T: 'a,
        Self: 'a;

    /// Take a sliced segment out from this segment with a length limited by
    /// the argument, reducing the length of this segment when the taken slice
    /// drops.
    fn take_segm_ref(
        &mut self,
        length: usize,
    ) -> impl TrBuffSegmRef<T>;

    fn iter_slices<'a>(&'a mut self) -> impl IntoIterator<Item = Self::Slice<'a>>
    where
        T: 'a;

    fn as_input(&mut self) -> impl TrInput<T> 
    where
        Self: Sized,
    {
        BuffSegmRefInput::<&mut Self, Self, T>::new(self)
    }
}

pub trait TrBuffSegmMut<T>
where
    Self: TrBuffSegmView<Item = MaybeUninit<T>> +
        AsMut<[MaybeUninit<T>]> + 
        BorrowMut<[MaybeUninit<T>]>,
{
    type Slice<'a>: DerefMut<Target = [Self::Item]>
    where
        T: 'a,
        Self: 'a;

    /// Take a sliced segment out from this segment with a length limited by the
    /// the argument, reducing the length of this segment when the taken slice
    /// drops.
    fn take_segm_mut(
        &mut self, 
        length: usize,
    ) -> impl TrBuffSegmMut<T>;

    fn iter_slices<'a>(&'a mut self) -> impl IntoIterator<Item = Self::Slice<'a>>
    where
        T: 'a;

    fn as_output(&mut self) -> impl TrOutput<T>
    where
        Self: Sized,
    {
        BuffSegmMutOutput::<&mut Self, Self, T>::new(self)
    }
}
