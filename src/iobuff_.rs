use core::{
    borrow::Borrow,
    iter::IntoIterator,
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
};

/// A borrowed segment of the output buffer
pub trait TrBuffSegmRef<T>
where
    Self: IntoIterator<Item = T>,
{
    type SliceItem<'a>: 'a + Borrow<T> + Clone
    where
        T: 'a,
        Self: 'a;

    type SliceClone<'a>: 'a + Deref<Target = [Self::SliceItem<'a>]>
    where
        T: 'a,
        Self: 'a;

    fn slice_clone(&self) -> Option<Self::SliceClone<'_>>;
}

/// A borrowed segment of the input buffer
pub trait TrBuffSegmMut<T>
where
    Self: IntoIterator<Item = MaybeUninit<T>>,
{
    type SliceMut<'a>: DerefMut<Target = [MaybeUninit<T>]> where Self: 'a;

    fn slice_mut(&mut self) -> Option<Self::SliceMut<'_>>;
}
