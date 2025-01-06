use core::{
    borrow::Borrow,
    iter::{self, IntoIterator},
    marker::PhantomData,
};

pub trait TrChunk {
    type IterItem;
    type SliceElem: Clone;
    type IntoIter: IntoIterator<Item = Self::IterItem>;
    type BorrowedSlice: Borrow<[Self::SliceElem]>;

    fn try_as_slice<'a, TSlice, TElem>(
        &'a self,
        transmute: impl FnOnce(&'a Self::IntoIter) -> Option<TSlice>,
    ) -> Option<impl 'a + TrChunk<SliceElem = TElem, BorrowedSlice = TSlice>>
    where
        Self::IntoIter: 'a,
        TSlice: 'a + Borrow<[TElem]>,
        TElem: 'a + Clone;

    fn try_as_iterable(&self) -> Option<&Self::IntoIter>;
}

/// To encapsulate data chunks that either its items cannot be cloned, or items
/// can be cloned and represented within a slice.
pub enum Chunk<I, T, S, E>
where
    I: IntoIterator<Item = T>,
    S: Borrow<[E]>,
    E: Clone,
{
    Iterable(I),
    Slice(S, PhantomData<[E]>),
}

impl<I, E> Chunk<I, E, PhantomChunk<()>, ()>
where
    I: IntoIterator<Item = E>,
{
    pub const fn iterable(iterable: I) -> Self {
        Chunk::Iterable(iterable)
    }
}

impl<S, T> Chunk<PhantomChunk<T>, T, S, T>
where
    S: Borrow<[T]>,
    T: Clone,
{
    pub const fn slice(slice: S) -> Self {
        Chunk::Slice(slice, PhantomData)
    }
}

impl<I, T, S, E> Chunk<I, T, S, E>
where
    I: IntoIterator<Item = T>,
    S: Borrow<[E]>,
    E: Clone,
{
    /// Returns the transmute result wrapped with [Some] only when self matches
    /// [Chunk::Iterable] and transmute returns [Some], or returns [None].
    pub fn try_as_slice<'a, TSlice, TElem>(
        &'a self,
        transmute: impl FnOnce(&'a I) -> Option<TSlice>,
    ) -> Option<Chunk<PhantomChunk<TElem>, TElem, TSlice, TElem>>
    where
        TSlice: 'a + Borrow<[TElem]>,
        TElem: Clone,
    {
        if let Chunk::Iterable(iter) = &self {
            let s = transmute(iter)?;
            Option::Some(Chunk::Slice(s, PhantomData))
        } else {
            Option::None
        }
    }

    pub fn try_as_iterable(&self) -> Option<&I> {
        if let Chunk::Iterable(iter) = &self {
            Option::Some(iter)
        } else {
            Option::None
        }
    }
}

impl<I, T, S, E> From<S> for Chunk<I, T, S, E>
where
    I: IntoIterator<Item = T>,
    S: Borrow<[E]>,
    E: Clone,
{
    fn from(value: S) -> Self {
        Chunk::Slice(value, PhantomData)
    }
}

impl<I, T, S, E> TrChunk for Chunk<I, T, S, E>
where
    I: IntoIterator<Item = T>,
    S: Borrow<[E]>,
    E: Clone,
{
    type IterItem = T;
    type SliceElem = E;
    type IntoIter = I;
    type BorrowedSlice = S;

    #[inline]
    fn try_as_iterable(&self) -> Option<&Self::IntoIter> {
        Chunk::try_as_iterable(self)
    }

    fn try_as_slice<'a, TSlice, TElem>(
        &'a self,
        transmute: impl FnOnce(&'a Self::IntoIter) -> Option<TSlice>,
    ) -> Option<impl 'a + TrChunk<SliceElem = TElem, BorrowedSlice = TSlice>>
    where
        Self::IntoIter: 'a,
        TSlice: 'a + Borrow<[TElem]>,
        TElem: 'a + Clone,
    {
        Chunk::try_as_slice(self, transmute)
    }
}

pub struct PhantomChunk<T>([T; 0]);

impl<T> IntoIterator for PhantomChunk<T> {
    type IntoIter = iter::Empty<T>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        iter::empty()
    }
}

impl<T> Borrow<[T]> for PhantomChunk<T>
where
    T: Clone,
{
    fn borrow(&self) -> &[T] {
        &self.0
    }
}

#[cfg(test)]
mod tests_ {
    use std::boxed::Box;
    use super::*;

    fn accept_slice<I, T, S, E>(
        x: impl Into<Chunk<I, T, S, E>>,
    ) -> Chunk<I, T, S, E>
    where
        I: IntoIterator<Item = T>,
        S: Borrow<[E]>,
        E: Clone,
    {
        x.into()
    }

    #[test]
    fn slice_should_be_chunk_slice() {
        let arr = [0u8; 1];
        let chunk: Chunk<PhantomChunk<u8>, u8, &[u8], u8> = accept_slice(arr.as_slice());
        assert!(matches!(chunk, Chunk::Slice(_, _)));

        let chunk = Chunk::iterable(arr.as_ref());
        assert!(matches!(chunk, Chunk::Iterable(_)));
        let try_as =  chunk.try_as_slice(|i| Option::Some(*i));
        let Option::Some(chunk) = try_as else {
            panic!()
        };
        assert!(matches!(chunk, Chunk::Slice(_, _)));

        let arr = Box::new([0u8; 1]);
        let slice = arr.as_slice();
        let _chunk = Chunk::slice(slice);
    }
}
