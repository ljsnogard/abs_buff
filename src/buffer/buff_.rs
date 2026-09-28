use core::{
    borrow::{Borrow, BorrowMut},
    mem::{self, MaybeUninit},
    slice,
};

/// A marker trait specifically abstracted from `MaybeUninit<T>` or types alike.
///
/// # Safety
/// - The only reasonable implementation is core::mem::MaybeUninit<T>, which is
///   already included in this crate.
pub impl(crate) unsafe trait TrMaybeUninit {
    type Inner: ?Sized;

    /// See [core::mem::MaybeUninit::uninit]
    fn uninit() -> Self;

    /// See [core::mem::MaybeUninit::zeroed]
    fn zeroed() -> Self;

    /// See [core::mem::MaybeUninit::as_bytes]
    fn as_bytes(&self) -> &[MaybeUninit<u8>];

    /// See [core::mem::MaybeUninit::as_bytes_mut]
    fn as_bytes_mut(&mut self) -> &mut [MaybeUninit<u8>];

    /// Extracts the value from the `MaybeUninit<T>` container. This is a great way
    /// to ensure that the data will get dropped, because the resulting `T` is
    /// subject to the usual drop handling.
    ///
    /// # Safety
    /// See [core::mem::MaybeUninit::assume_init].
    unsafe fn assume_init(self) -> Self::Inner;

    /// Reads the value from the `MaybeUninit<T>` container. The resulting `T` is subject
    /// to the usual drop handling.
    ///
    /// # Safety
    /// See [core::mem::MaybeUninit::assume_init_read].
    unsafe fn assume_init_read(&self) -> Self::Inner;

    /// Gets a shared reference to the contained value.
    ///
    /// # Safety
    /// See [core::mem::MaybeUninit::assume_init_ref].
    unsafe fn assume_init_ref(&self) -> &Self::Inner;

    /// Gets a mutable reference to the containted value.
    ///
    /// # Safety
    /// See [core::mem::MaybeUninit::assume_init_mut]
    unsafe fn assume_init_mut(&mut self) -> &mut Self::Inner;

    /// Drops the contained value in place.
    ///
    /// # Safety
    /// See [core::mem::MaybeUninit::assume_init_drop]
    unsafe fn assume_init_drop(&mut self);

    /// See [core::mem::MaybeUninit::write]
    fn write(&mut self, value: Self::Inner) -> &mut Self::Inner;
}


/// A trait to unify `[T]` and `[MaybeUninit<T>]` when `T: Copy`.
/// # Safety
/// - The only reasonable implementations are `[MaybeUninit<T>]` and `MaybeUninit<[T; N]>`
pub impl(crate) unsafe trait TrMaybeUninitSlice<T> {
    /// 返回一个指向相同内存的 `[MaybeUninit<T>]` 视图。
    fn as_uninit_slice(&self) -> &[MaybeUninit<T>];
}

/// A trait to unify `[T]` and `[MaybeUninit<T>]` when `T: Copy`.
/// # Safety
/// - The only reasonable implementations are `[MaybeUninit<T>]` and `MaybeUninit<[T; N]>`
pub impl(crate) unsafe trait TrMaybeUninitSliceMut<T>
where
    Self: TrMaybeUninitSlice<T>,
{
    /// 返回一个可变的 `[MaybeUninit<Self::Item>]` 视图。
    fn as_uninit_slice_mut(&mut self) -> &mut [MaybeUninit<T>];
}

//-- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ----
// impl TrMaybeUninit for `MaybeUninit<T>`
//-- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ----

unsafe impl<T> TrMaybeUninit for MaybeUninit<T> {
    type Inner = T;

    #[inline]
    fn uninit() -> Self {
        MaybeUninit::uninit()
    }

    #[inline]
    fn zeroed() -> Self {
        MaybeUninit::zeroed()
    }

    #[inline]
    fn as_bytes(&self) -> &[MaybeUninit<u8>] {
        // self.as_bytes()
        // SAFETY: MaybeUninit<u8> is always valid, even for padding bytes
        unsafe {
            slice::from_raw_parts(
                self.as_ptr().cast::<MaybeUninit<u8>>(),
                mem::size_of::<T>(),
            )
        }
    }

    #[inline]
    fn as_bytes_mut(&mut self) -> &mut [MaybeUninit<u8>] {
        // self.as_bytes_mut()
        unsafe {
            slice::from_raw_parts_mut(
                self.as_mut_ptr().cast::<MaybeUninit<u8>>(),
                mem::size_of::<T>(),
            )
        }
    }

    #[inline]
    unsafe fn assume_init(self) -> Self::Inner {
        unsafe { self.assume_init() }
    }

    #[inline]
    unsafe fn assume_init_read(&self) -> Self::Inner {
        unsafe { self.assume_init_read() }
    }

    #[inline]
    unsafe fn assume_init_ref(&self) -> &Self::Inner {
        unsafe { self.assume_init_ref() }
    }

    #[inline]
    unsafe fn assume_init_mut(&mut self) -> &mut Self::Inner {
        unsafe { self.assume_init_mut() }
    }

    #[inline]
    unsafe fn assume_init_drop(&mut self) {
        unsafe {
            self.assume_init_drop();
        }
    }

    #[inline]
    fn write(&mut self, value: Self::Inner) -> &mut Self::Inner {
        MaybeUninit::write(self, value)
    }
}

//-- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ----
// impl TrMaybeUninitSlice<T> for `[MaybeUninit<T>]`
//-- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ---- ----

unsafe impl<C, T> TrMaybeUninitSlice<T> for C
where
    C: Borrow<[MaybeUninit<T>]>,
{
    fn as_uninit_slice(&self) -> &[MaybeUninit<T>] {
        self.borrow()
    }
}

unsafe impl<C, T> TrMaybeUninitSliceMut<T> for C
where
    C: BorrowMut<[MaybeUninit<T>]>,
{
    fn as_uninit_slice_mut(&mut self) -> &mut [MaybeUninit<T>] {
        self.borrow_mut()
    }
}

unsafe impl<T> TrMaybeUninitSlice<T> for [T]
where
    T: Copy,
{
    fn as_uninit_slice(&self) -> &[MaybeUninit<T>] {
        let data = self.as_ptr() as *const MaybeUninit<T>;
        let len = self.len();
        // SAFETY:
        // 1. `MaybeUninit<[T]>` 和 `[MaybeUninit<T>]` 具有相同的内存布局。
        // 2. 我们只是重新解释内存，不改变其内容。
        unsafe { slice::from_raw_parts(data, len) }
    }
}

unsafe impl<T> TrMaybeUninitSliceMut<T> for [T]
where
    T: Copy,
{
    fn as_uninit_slice_mut(&mut self) -> &mut [MaybeUninit<T>] {
        let data = self.as_mut_ptr() as *mut MaybeUninit<T>;
        let len = self.len();
        // SAFETY: 同上，但返回可变引用。
        // 需要确保在可变借用期间没有其他别名。
        unsafe { slice::from_raw_parts_mut(data, len) }
    }
}
