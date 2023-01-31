use core::cell::{RefCell, RefMut};

pub struct UPSafeCell<T> {
    inner:RefCell<T>
}

unsafe impl <T> Sync for UPSafeCell<T> {}

impl <T> UPSafeCell<T> {
    pub unsafe fn new(val:T) -> Self{
        Self {inner:RefCell::new(val)}
    }
    
    pub fn exclusive_borrow(&self) -> RefMut<'_, T>{
        //缺乏阻塞手段
        //这里的borrow_mut如果没有被释放就会panic
        self.inner.borrow_mut()
    }
}