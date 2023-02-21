extern crate alloc;
use alloc::sync::Arc;
use alloc::sync::Weak;
use core::cell::RefCell;
use core::cell::{Ref, RefMut};
use core::ops::{Deref,DerefMut};
use crate::sync::UPSafeCell;

pub trait GeneralRef {
    type Inner;
    fn clone_strong(&self) -> StrongRef<Self::Inner>;
}

#[derive(PartialEq)]
pub struct StrongRef<T>(pub(crate) Arc<UPSafeCell<T>>);

impl<T> StrongRef<T> {
    pub fn new(x:T) -> Self{
        unsafe {
            Self {
                0: Arc::new(UPSafeCell::new(x)),
            }
        }
    }

    pub fn clone(x:&Self) -> Self {
        Self {
            0:Arc::clone(&x.0)
        }
    }

    pub fn inner(&self) -> Ref<'_,T> {
        self.0.access()
    }

    pub fn inner_mut(&self) -> RefMut<'_, T>{
        self.0.exclusive_access()
    }
}

unsafe impl<T> Sync for StrongRef<T> {}

impl<T> GeneralRef for StrongRef<T> {
    type Inner = T;
    fn clone_strong(&self) -> Self {
        Self {
            0:Arc::clone(&self.0)
        }
    }
}

impl<T> Deref for StrongRef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let ptr = self.0.as_ptr();
        unsafe {&*ptr}
    }
}

impl<T> DerefMut for StrongRef<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let ptr = self.0.as_ptr();
        unsafe {&mut *ptr}
    }
}

pub struct WeakRef<T>(Weak<UPSafeCell<T>>);

impl<T> WeakRef<T> {
    pub fn new(x:StrongRef<T>) -> Self{
        Self {
            0:Arc::downgrade(&x.0)
        }
    }
}

impl<T> WeakRef<T>{
    pub fn to_strong(&self) -> Option<StrongRef<T>> {
        match self.0.upgrade() {
            Some(inner) => Some(StrongRef {
                0:inner
            }),
            None => None
        }
    }
}

unsafe impl<T> Sync for WeakRef<T> {}