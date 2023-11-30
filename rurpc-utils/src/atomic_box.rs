pub struct AtomicBox<T> {
    inner: std::sync::atomic::AtomicPtr<T>,
}

impl<T> AtomicBox<T> {
    const NULL: *mut T = std::ptr::null_mut();
    const INVALID: *mut T = unsafe { std::mem::transmute(usize::MAX) };

    pub fn load(&self) -> *mut T {
        self.inner.load(std::sync::atomic::Ordering::Acquire)
    }

    pub fn null() -> Self {
        Self {
            inner: std::sync::atomic::AtomicPtr::new(Self::NULL),
        }
    }

    pub fn is_null(&self) -> bool {
        self.load().is_null()
    }

    pub fn invalid() -> Self {
        Self {
            inner: std::sync::atomic::AtomicPtr::new(Self::INVALID),
        }
    }

    pub fn is_invalid(&self) -> bool {
        self.load() == Self::INVALID
    }

    pub fn swap(&mut self, new: Box<T>) -> Option<Box<T>> {
        self.inner_swap(Box::into_raw(new))
    }

    pub fn swap_and(&mut self, new: Box<T>, func: impl FnOnce(&mut T, Option<Box<T>>) -> ()) {
        let raw = Box::into_raw(new);
        func(unsafe { &mut *raw }, self.inner_swap(raw));
    }

    pub fn take(&mut self) -> Option<Box<T>> {
        self.inner_swap(std::ptr::null_mut())
    }

    fn inner_swap(&mut self, new: *mut T) -> Option<Box<T>> {
        let old = self.inner.swap(new, std::sync::atomic::Ordering::SeqCst);
        match old as usize {
            usize::MIN => None,
            usize::MAX => None,
            _ => Some(unsafe { Box::from_raw(old as *mut T) }),
        }
    }
}

impl<T> Drop for AtomicBox<T> {
    fn drop(&mut self) {
        self.take();
    }
}

impl<T> Default for AtomicBox<T> {
    fn default() -> Self {
        AtomicBox::<T>::null()
    }
}

#[cfg(test)]
mod test {
    use super::AtomicBox;

    #[test]
    fn null_ptr() {
        let a = AtomicBox::<u32>::null();
        assert_eq!(a.is_null(), true);
        drop(a);
    }

    #[test]
    #[should_panic]
    fn it_should_panic() {
        struct S;
        impl Drop for S {
            fn drop(&mut self) {
                panic!();
            }
        }

        let mut a: AtomicBox<S> = Default::default();
        assert_eq!(a.is_null(), true);

        a.swap(Box::new(S));
        drop(a);
    }
}
