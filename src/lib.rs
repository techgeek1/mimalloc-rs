#![feature(allocator_api)]
#![feature(nonnull_slice_from_raw_parts)]

use std::alloc::{Allocator, AllocError, Layout};
use std::ffi::c_void;
use std::ptr::{self, NonNull};

pub struct MiMalloc;

impl MiMalloc {
    /// Eagerly free memory.
    /// 
    /// # Remarks
    /// If `aggressive` is true, memory will be aggressively returned to the OS. This may
    /// be expensive!
    pub fn collect(&self, aggressive: bool) {
        unsafe { mi_collect(aggressive) }
    }
}

unsafe impl Allocator for MiMalloc {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe {
            let ptr = mi_malloc_aligned(layout.size(), layout.align())
                .cast::<u8>();

            if !ptr.is_null() {
                let ptr   = NonNull::new_unchecked(ptr);
                let slice = NonNull::slice_from_raw_parts(ptr, layout.size());

                Ok(slice)
            }
            else {
                Err(AllocError)
            }
        }
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, _layout: Layout) {
        mi_free(ptr.cast::<c_void>().as_ptr())
    }
    
    fn allocate_zeroed(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        unsafe {
            let ptr = mi_zalloc_aligned(layout.size(), layout.align())
                .cast::<u8>();

            if !ptr.is_null() {
                let ptr   = NonNull::new_unchecked(ptr);
                let slice = NonNull::slice_from_raw_parts(ptr, layout.size());

                Ok(slice)
            }
            else {
                Err(AllocError)
            }
        }
    }

    unsafe fn grow(&self, ptr: NonNull<u8>, _old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let ptr     = ptr.cast::<c_void>().as_ptr();
        let mut new = mi_expand(ptr, new_layout.size())
            .cast::<u8>();

        // Grow in place failed, reallocate
        if new.is_null() {
            new = mi_realloc_aligned(ptr, new_layout.size(), new_layout.align())
                .cast::<u8>();
        }

        if !new.is_null() {
            let ptr   = NonNull::new_unchecked(new);
            let slice = NonNull::slice_from_raw_parts(ptr, new_layout.size());

            Ok(slice)
        }
        else {
            Err(AllocError)
        }
    }

    unsafe fn grow_zeroed(&self, ptr: NonNull<u8>, old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let ptr     = ptr.cast::<c_void>().as_ptr();
        let mut new = mi_expand(ptr, new_layout.size())
            .cast::<u8>();

        // Grow in place failed, reallocate
        if new.is_null() {
            new = mi_rezalloc_aligned(ptr, new_layout.size(), new_layout.align())
                .cast::<u8>();
        }
        // Grow successful, zero the new memory
        else {
            let uninit_ptr  = ptr.add(old_layout.size());
            let uninit_size = new_layout.size() - old_layout.size();
            
            ptr::write_bytes(uninit_ptr, 0, uninit_size);
        }

        if !new.is_null() {
            let ptr   = NonNull::new_unchecked(new);
            let slice = NonNull::slice_from_raw_parts(ptr, new_layout.size());

            Ok(slice)
        }
        else {
            Err(AllocError)
        }
    }

    unsafe fn shrink(&self, ptr: NonNull<u8>, _old_layout: Layout, new_layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let ptr = ptr.cast::<c_void>().as_ptr();
        let new = mi_realloc(ptr, new_layout.size())
            .cast::<u8>();

        if !new.is_null() {
            let ptr   = NonNull::new_unchecked(new);
            let slice = NonNull::slice_from_raw_parts(ptr, new_layout.size());

            Ok(slice)
        }
        else {
            Err(AllocError)
        }
    }
}

// The mi-malloc api
//
// Documentation at https://microsoft.github.io/mimalloc/group__malloc.html
extern "C" {
    // Basic Allocation
    fn mi_expand(p: *mut c_void, newsize: usize) -> *mut c_void;
    fn mi_free(p: *mut c_void);
    fn mi_realloc(p: *mut c_void, newsize: usize) -> *mut c_void;
    
    // Extended Functions
    fn mi_collect(force: bool);

    // Aligned Allocation 
    fn mi_malloc_aligned(size: usize, alignment: usize) -> *mut c_void;
    fn mi_realloc_aligned(p: *mut c_void, newsize: usize, alignment: usize) -> *mut c_void;
    fn mi_zalloc_aligned(size: usize, alignment: usize) -> *mut c_void;

    // Heap Allocation
    // TODO: Support the heap API at some point

    // Zero initialized re-allocation
    fn mi_rezalloc_aligned(p: *mut c_void, newsize: usize, alignment: usize) -> *mut c_void;
}