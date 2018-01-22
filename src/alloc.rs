use std::os::raw::{c_void, c_char};
use std::marker::PhantomData;
use std::{ptr, slice};
use std::mem::ManuallyDrop;

extern "C" {
    #[cfg(postgres = "10.0")]
    fn GenerationContextCreate(parent: *mut c_void, name: *const c_char, flags: i32, block_size: usize) -> *mut c_void;
    #[cfg(postgres = "9.5")]
    fn AllocSetContextCreate(parent: *mut c_void, name: *const c_char, min_size: usize, init_size: usize, max_size: usize) -> *mut c_void;

    fn MemoryContextAlloc(context: *mut c_void, size: usize) -> *mut c_void;
    fn MemoryContextAllocZero(context: *mut c_void, size: usize) -> *mut c_void;

    fn MemoryContextDelete(context: *mut c_void);
    static mut CurrentMemoryContext: *mut c_void;
}

#[repr(C)]
#[derive(Debug)]
pub struct MemoryContext<'parent> {
    ptr: *mut c_void,
    parent: PhantomData<&'parent MemoryContext<'parent>>,
}

pub unsafe fn get_current_ctx() -> ManuallyDrop<MemoryContext<'static>> {
    ManuallyDrop::new(MemoryContext {
        ptr: CurrentMemoryContext,
        parent: PhantomData,
    })
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

    pub fn alloc<'a>(&'a self, size: usize) -> &'a mut [u8] {
        unsafe {
            slice::from_raw_parts_mut(MemoryContextAllocZero(self.ptr, size) as *mut u8, size)
        }
    }

    // this is safe because it only returns a ptr
    pub fn alloc_undef<'a>(&'a self, size: usize) -> *mut u8 {
        unsafe { MemoryContextAlloc(self.ptr, size) as *mut u8 }
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
