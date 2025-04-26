use core::iter::IntoIterator;

use crate::{TrBuffSegmRef, TrBuffSegmView};

impl<T> TrBuffSegmView for &[T] {
    type Item = T;

    #[inline]
    fn is_empty(&self) -> bool {
        self.as_ref().is_empty()
    }

    #[inline]
    fn len(&self) -> usize {
        self.as_ref().len()
    }

    #[inline]
    fn capacity(&self) -> usize {
        self.len()
    }

    #[inline]
    fn iter_ptr(&self) -> impl Iterator<Item = *const Self::Item> {
        self.iter().map(|t| t as *const T)
    }
}

impl<T> TrBuffSegmRef<T> for &[T] {
    type Slice<'a> = Self where Self: 'a;

    fn take_segm_ref(&mut self, length: usize) -> impl TrBuffSegmRef<T> {
        let (a, b) = self.split_at(length);
        *self = b;
        a
    }

    fn iter_slices<'a>(&'a mut self) -> impl IntoIterator<Item = Self::Slice<'a>>
    where
        T: 'a,
    {
        let opt = if self.is_empty() {
            Option::None
        } else {
            Option::Some(&self[0..])
        };
        opt.into_iter()
    }
}
