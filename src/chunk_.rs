use core::{
    iter::{self, IntoIterator},
    ops::Deref
};

/// To encapsulate data chunks that either its items cannot be cloned, or items
/// can be cloned and represented with in a slice.
pub enum Chunk<I, S, T>
where
    I: IntoIterator,
    S: Deref<Target = [T]>,
    T: Clone,
{
    Iterable(I),
    Slice(S),
}

impl<I> Chunk<I, PhantomChunk<()>, ()>
where
    I: IntoIterator,
{
    /// Create a `Chunk::Iterable` with the given argument.
    pub const fn iterable(i: I) -> Self {
        Chunk::Iterable(i)
    }
}

impl<S, T> From<S> for Chunk<PhantomChunk<T>, S, T>
where
    S: Deref<Target = [T]>,
    T: Clone,
{
    fn from(value: S) -> Self {
        Chunk::Slice(value)
    }
}

/// A wrapper around a zero-len array that can be placeholder for
/// [`IntoIterator`] and [`Deref<Target = [T]>`](core::ops::Deref), which are
/// used in [Chunk] type parameter.
pub struct PhantomChunk<T>([T; 0]);

impl<T> Deref for PhantomChunk<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> IntoIterator for PhantomChunk<T> {
    type IntoIter = iter::Empty<T>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        iter::empty()
    }
}

#[cfg(test)]
mod tests_ {
    #[test]
    fn slice_should_be_chunk_slice() {
        use super::*;

        fn accept_chunk<I, S, T>(
            chunk: Chunk<I, S, T>,
        ) -> Chunk<I, S, T>
        where
            I: IntoIterator,
            S: Deref<Target = [T]>,
            T: Clone,
        {
            chunk
        }

        let arr = [0u8; 1];
        let chunk = accept_chunk(arr.as_ref().into());
        assert!(matches!(chunk, Chunk::Slice(_)));

        let chunk = Chunk::iterable(arr);
        assert!(matches!(chunk, Chunk::Iterable(_)));
    }
}
