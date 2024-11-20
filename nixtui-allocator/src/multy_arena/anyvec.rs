use std::any::{Any, TypeId};
use std::{mem, ptr};
use std::alloc::{self, Layout};
use std::ptr::NonNull;

pub struct AnyVec {
    ptr: NonNull<u8>,
    len: usize,
    cap: usize,
    type_id: TypeId,
    type_size: usize,
}

impl AnyVec {

    /// # Safety
    /// Because `AnyVec` doesn't know exact type it's holding and Rust
    /// prevents anyone from accesing `Drop::drop` function, the destructor
    /// defined in `Drop::drop()` won't be run automatically.
    /// `AnyVec` provides `manually_drop()`, but the values can not be
    /// dropped during unwinding
    pub unsafe fn new_unchecked <T: Any> () -> Self {
        assert!(mem::size_of::<T>() != 0, "T must not be ZST");
        Self {
            ptr: NonNull::dangling(),
            len: 0,
            cap: 0,
            type_id: TypeId::of::<T>(),
            type_size: mem::size_of::<T>(),
        }
    }

    /// # Panics
    /// Panics, if type `T` needs drop
    pub fn new <T: Any> () -> Self {
        assert!( mem::size_of::<T>() != 0, "T must not be ZST" );
        assert!( !mem::needs_drop::<T>() );
        Self {
            ptr: NonNull::dangling(),
            len: 0,
            cap: 0,
            type_id: TypeId::of::<T>(),
            type_size: mem::size_of::<T>(),
        }
    }

    fn grow (&mut self) {
        let (new_cap, new_layout) = if self.cap == 0 {
            (self.type_size, Layout::array::<u8>(self.type_size).unwrap())
        } else {
            let new_cap = self.cap * 2;
            assert!(new_cap <= isize::MAX as usize, "Allocation too large!");
            let new_layout = Layout::array::<u8>(new_cap).unwrap();
            (new_cap, new_layout)
        };
        let new_ptr = if self.cap == 0 {
            unsafe { alloc::alloc(new_layout) }
        } else {
            let old_layout = Layout::array::<u8>(self.cap).unwrap();
            let old_ptr = self.ptr.as_ptr();
            unsafe { alloc::realloc(old_ptr, old_layout, new_layout.size())}
        };

        self.ptr = match NonNull::new(new_ptr) {
            Some(p) => p,
            None => alloc::handle_alloc_error(new_layout),
        };
        self.cap = new_cap;
    }

    pub fn push <T: Any> (&mut self, elem: T) {
        if self.type_id != elem.type_id() {
            panic!("Value of another type detected while pushing");
        }
        if self.len == self.cap { self.grow() };
        unsafe {
            ptr::write(self.ptr.as_ptr().add(self.len) as *mut T, elem);
        }
        self.len += self.type_size;
    }

    pub fn pop <T: Any> (&mut self) -> Option<T> {
        if self.type_id != TypeId::of::<T>() {
            panic!("Value of another type detected while popping");
        }
        if self.len == 0 {
            None
        }else {
            self.len -= self.type_size;
            unsafe {
                Some(ptr::read(self.ptr.as_ptr().add(self.len) as *const T))
            }
        }
    }

    pub fn manually_drop <T: Any + Drop> (mut self) {
        if self.type_id != TypeId::of::<T>() {
            panic!("Value of another type detected while dropping");
        }
        while self.pop::<T>().is_some() {}
    }

    pub fn slice <T: Any> (&self) -> &[T] {
        if self.type_id != TypeId::of::<T>() {
            panic!("Value of another type detected while dereferencing");
        }
        if self.is_empty() {
            return &[]
        }
        unsafe {
            std::slice::from_raw_parts(self.ptr.as_ptr() as *const T, self.len())
        }
    }

    pub fn slice_mut <T: Any> (&mut self) -> &mut [T] {
        if self.type_id != TypeId::of::<T>() {
            panic!("Value of another type detected while dereferencing mutably");
        }
        if self.is_empty() {
            return &mut[]
        }
        unsafe {
            std::slice::from_raw_parts_mut(self.ptr.as_ptr() as *mut T, self.len())
        }
    }

    pub fn get <T: Any> (&self, index: usize) -> Option<&T> {
        self.slice().get(index)
    }

    pub fn get_mut <T: Any> (&mut self, index: usize) -> Option<&mut T> {
        self.slice_mut().get_mut(index)
    }

    pub fn len(&self) -> usize {
        self.len / self.type_size
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

}

impl Drop for AnyVec {
    fn drop(&mut self) {
        if self.cap != 0 {
            let layout = Layout::array::<u8>(self.cap).unwrap();
            unsafe {
                alloc::dealloc(self.ptr.as_ptr(), layout);
            }
        }
    }
}



#[cfg(test)]
mod tests {
    use std::{ cell::RefCell, rc::Rc};

    use super::*;

    struct Droplet(Rc<RefCell<usize>>);
    impl Drop for Droplet {
        fn drop(&mut self) {
            let mut num = (*self.0).borrow_mut();
            *num += 1;
            println!("{}", num);
        }
    }


    #[test]
    #[should_panic]
    fn drop_isnt_allowed(){
        let mut av = AnyVec::new::<Droplet>();
        let val = Rc::new(RefCell::new(0));
        av.push(Droplet(val.clone()));
        av.push(Droplet(val));
        av.manually_drop::<Droplet>();
    }

    #[test]
    fn manual_drop(){
        let mut av = unsafe { AnyVec::new_unchecked::<Droplet>() };
        let val = Rc::new(RefCell::new(0));
        av.push(Droplet(val.clone()));
        av.push(Droplet(val.clone()));
        av.manually_drop::<Droplet>();
        assert_eq!(Rc::strong_count(&val), 1);
        assert_eq!(*val.borrow(), 2);
    }

    #[test]
    fn test_deref(){
        let mut av = AnyVec::new::<usize>();
        av.push(0_usize);
        av.push(1_usize);
        av.push(2_usize);
        let slice = av.slice::<usize>();
        assert_eq!(slice[0], 0);
        assert_eq!(slice[1], 1);
        assert_eq!(slice[2], 2);
    }

}
