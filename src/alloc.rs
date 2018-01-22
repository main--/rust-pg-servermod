use std::os::raw::{c_void, c_char};
use std::marker::PhantomData;
use std::ptr;

extern "C" {
    #[cfg(postgres = "10.0")]
    fn GenerationContextCreate(parent: *mut c_void, name: *const c_char, flags: i32, block_size: usize) -> *mut c_void;
    #[cfg(postgres = "9.5")]
    fn AllocSetContextCreate(parent: *mut c_void, name: *const c_char, min_size: usize, init_size: usize, max_size: usize) -> *mut c_void;
    fn MemoryContextDelete(context: *mut c_void);
    static mut CurrentMemoryContext: *mut c_void;
}

#[repr(C)]
#[derive(Debug)]
pub struct MemoryContext<'parent> {
    ptr: *mut c_void,
    parent: PhantomData<&'parent MemoryContext<'parent>>,
}

impl<'parent> MemoryContext<'parent> {
    pub fn create_allocset(parent: Option<&'parent MemoryContext<'parent>>,
                           // name: cstr
                           min_size: usize,
                           init_size: usize,
                           max_size: usize) -> MemoryContext<'parent> {
        unsafe {
            MemoryContext {
                ptr: AllocSetContextCreate(parent.map(|p| p.ptr).unwrap_or(ptr::null_mut()), b"\0".as_ptr() as *const _, min_size, init_size, max_size),
                parent: PhantomData,
            }
        }
    }

    pub unsafe fn set_current(&self) {
        CurrentMemoryContext = self.ptr;
    }
}

impl<'parent> Drop for MemoryContext<'parent> {
    fn drop(&mut self) {
        unsafe { MemoryContextDelete(self.ptr) }
    }
}
