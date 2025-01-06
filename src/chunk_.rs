use core::{
    borrow::Borrow,
    iter::{self, IntoIterator},
    marker::PhantomData,
};

/// To encapsulate data chunks that either its items cannot be cloned, or items
/// can be cloned and represented within a slice.
pub enum Chunk<I, S, T>
where
    I: IntoIterator,
    S: Borrow<[T]>,
    T: Clone,
{
    Iterable(I),
    Slice(S, PhantomData<[T]>),
}

impl<I> Chunk<I, PhantomChunk<()>, ()>
where
    I: IntoIterator,
{
    pub const fn iterable(iterable: I) -> Self {
        Chunk::Iterable(iterable)
    }
}

impl<S, T> Chunk<PhantomChunk<T>, S, T>
where
    S: Borrow<[T]>,
    T: Clone,
{
    pub const fn slice(slice: S) -> Self {
        Chunk::Slice(slice, PhantomData)
    }
}

impl<I, S, T> Chunk<I, S, T>
where
    I: IntoIterator,
    S: Borrow<[T]>,
    T: Clone,
{
    /// Returns the transmute result wrapped with [Some] only when self matches
    /// [Chunk::Iterable] and transmute returns [Some], or returns [None].
    pub fn try_as_slice<'a, TSlice, TItem>(
        &'a self,
        transmute: impl FnOnce(&'a I) -> Option<TSlice>,
    ) -> Option<Chunk<PhantomChunk<TItem>, TSlice, TItem>>
    where
        TSlice: 'a + Borrow<[TItem]>,
        TItem: Clone,
    {
        if let Chunk::Iterable(iter) = &self {
            let s = transmute(iter)?;
            Option::Some(Chunk::Slice(s, PhantomData))
        } else {
            Option::None
        }
    }
}

impl<S, T> From<S> for Chunk<PhantomChunk<T>, S, T>
where
    S: Borrow<[T]>,
    T: Clone,
{
    fn from(value: S) -> Self {
        Chunk::Slice(value, PhantomData)
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

    fn accept_slice<I, S, T>(
        x: impl Into<Chunk<I, S, T>>,
    ) -> Chunk<I, S, T>
    where
        I: IntoIterator,
        S: Borrow<[T]>,
        T: Clone,
    {
        x.into()
    }

    #[test]
    fn slice_should_be_chunk_slice() {
        let arr = [0u8; 1];
        let chunk = accept_slice(arr.as_ref());
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
