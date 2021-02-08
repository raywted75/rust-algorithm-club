use core::{mem, ptr};
use std::alloc::{alloc, dealloc, realloc, Layout};

// A double-ended queue (abbreviated to _deque_), for which elements can be
// added or remove from both back and front ends.
//
// Underneath the hood, this [`Deque`] uses a contiguous memory block as a ring
// buffer to store values.
//
// References:
//
/// - [Rust Standard Library: std::collections::VecDeque][1]
/// - [Wikipedia: Circular buffer][2]
///
/// [1]: `std::collections::VecDeque`
/// [2]: https://en.wikipedia.org/wiki/Circular_buffer
// ANCHOR: layout
pub struct Deque<T> {
    tail: usize,
    head: usize,
    ring_buf: RawVec<T>,
}
// ANCHOR_END: layout

/// For testing convenience, set default capacity to 1 in order to trigger
/// buffer expansions easily. This value must be power of 2.
const DEFAULT_CAPACITY: usize = 1;

impl<T> Deque<T> {
    /// Constructs a new, empty [`Deque<T>`].
    ///
    /// For convenience, the deque initially allocates a region of a single `T`.
    // ANCHOR: new
    pub fn new() -> Self {
        Self {
            tail: 0,
            head: 0,
            ring_buf: RawVec::with_capacity(DEFAULT_CAPACITY),
        }
    }
    // ANCHOR_END: new

    /// Prepends the given element value to the beginning of the container.
    ///
    /// # Parameters
    ///
    /// * `elem` - The element to prepend.
    ///
    /// # Complexity
    ///
    /// Constant.
    // ANCHOR: push_front
    pub fn push_front(&mut self, elem: T) {
        self.try_resize();
        self.tail = self.wrapping_sub(self.tail, 1);
        // This is safe because the offset is wrapped inside available memory by `wrap_index()`.
        unsafe { self.ptr().add(self.tail).write(elem) }
    }
    // ANCHOR_END: push_front

    /// Appends the given element value to the end of the container.
    ///
    /// # Parameters
    ///
    /// * `elem` - The element to append.
    ///
    /// # Complexity
    ///
    /// Constant.
    // ANCHOR: push_back
    pub fn push_back(&mut self, elem: T) {
        self.try_resize();
        let head = self.head;
        self.head = self.wrapping_add(self.head, 1);
        // This is safe because the offset is wrapped inside available memory by `wrap_index()`.
        unsafe { self.ptr().add(head).write(elem) }
    }
    // ANCHOR_END: push_back

    /// Removes and returns the first element of the container.
    /// If there are no elements in the container, return `None`.
    ///
    /// # Complexity
    ///
    /// Constant.
    // ANCHOR: pop_front
    pub fn pop_front(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }
        let tail = self.tail;
        self.tail = self.wrapping_add(self.tail, 1);
        // This is safe because the offset is wrapped inside available memory by `wrap_index()`.
        unsafe { Some(self.ptr().add(tail).read()) }
    }
    // ANCHOR_END: pop_front

    /// Removes and returns the last element of the container.
    /// If there are no elements in the container, return `None`.
    ///
    /// # Complexity
    ///
    /// Constant.
    // ANCHOR: push_back
    pub fn pop_back(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }
        self.head = self.wrapping_sub(self.head, 1);
        // This is safe because the offset is wrapped inside available memory by `wrap_index()`.
        unsafe { Some(self.ptr().add(self.head).read()) }
    }
    // ANCHOR_END: push_back

    /// Peeks the first element of the container.
    /// If there are no elements in the container, return `None`.
    ///
    /// # Complexity
    ///
    /// Constant.
    // ANCHOR: front
    pub fn front(&self) -> Option<&T> {
        if self.is_empty() {
            return None;
        }
        // This is safe due to the offset is wrapped inside available memory by `wrap_index()`.
        unsafe { Some(&*self.ptr().add(self.tail)) }
    }
    // ANCHOR_END: front

    /// Peeks the last element of the container.
    /// If there are no elements in the container, return `None`.
    ///
    /// # Complexity
    ///
    /// Constant.
    // ANCHOR: back
    pub fn back(&self) -> Option<&T> {
        if self.is_empty() {
            return None;
        }
        let head = self.wrapping_sub(self.head, 1);
        // This is safe due to the offset is wrapped inside available memory by `wrap_index()`.
        unsafe { Some(&*self.ptr().add(head)) }
    }
    // ANCHOR_END: back

    ///	Checks whether the container is empty.
    ///
    /// # Complexity
    ///
    /// Constant.
    // ANCHOR: is_empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    // ANCHOR_END: is_empty

    ///	Gets the number of elements in the container.
    ///
    /// # Complexity
    ///
    /// Constant.
    // ANCHOR: len
    pub fn len(&self) -> usize {
        self.head.wrapping_sub(self.tail) & self.cap() - 1
    }
    // ANCHOR_END: len

    /// Checks if underlying ring buffer is full.
    // ANCHOR: is_full
    fn is_full(&self) -> bool {
        self.cap() - self.len() == 1
    }
    // ANCHOR_END: is_full

    /// Resizes the underlying ring buffer if necessary.
    ///
    /// # Complexity
    ///
    /// Linear in the size of the container.
    ///
    // ANCHOR: try_resize
    fn try_resize(&mut self) {
        if self.is_full() {
            let old_cap = self.cap();
            self.ring_buf.grow();

            if self.tail > self.head {
                // Make the ring buffer contiguous.
                //
                // The content of ring buffer won't overlapping, so
                // `copy_nonoverlapping` is safe to called.
                //
                // Before:
                //          h   t
                // [o o o o x x o o]
                //
                // Resize:
                //          h   t
                // [o o o o x x o o | x x x x x x x x]
                //
                // Copy:
                //              t           h
                // [x x x x x x o o | o o o o x x x x]
                //  _ _ _ _           _ _ _ _
                unsafe {
                    let src = self.ptr();
                    let dst = self.ptr().add(old_cap);
                    ptr::copy_nonoverlapping(src, dst, self.head);
                }
                self.head += old_cap;
            }
        }
    }
    // ANCHOR_END: try_resize

    /// Returns the actual index of the underlying ring buffer for a given
    /// logical index + addend.
    // ANCHOR: wrapping_add
    fn wrapping_add(&self, index: usize, addend: usize) -> usize {
        wrap_index(index.wrapping_add(addend), self.cap())
    }
    // ANCHOR_END: wrapping_add

    /// Returns the actual index of the underlying ring buffer for a given
    /// logical index - subtrahend.
    // ANCHOR: wrapping_sub
    fn wrapping_sub(&self, index: usize, subtrahend: usize) -> usize {
        wrap_index(index.wrapping_sub(subtrahend), self.cap())
    }
    // ANCHOR_END: wrapping_sub

    /// An abstraction for accessing the pointer of the ring buffer.
    // ANCHOR: ptr
    #[inline]
    fn ptr(&self) -> *mut T {
        self.ring_buf.ptr
    }
    // ANCHOR_END: ptr

    /// An abstraction for accessing the capacity of the ring buffer.
    // ANCHOR: cap
    #[inline]
    fn cap(&self) -> usize {
        self.ring_buf.cap
    }
    // ANCHOR_END: cap
}

/// Returns the actual index of the underlying ring buffer for a given logical index.
///
/// To ensure all bits of `size - 1` is set to 1, here the size must always be 
/// power of two.
// ANCHOR: wrap_index
fn wrap_index(index: usize, size: usize) -> usize {
    debug_assert!(size.is_power_of_two());
    index & (size - 1)
}
// ANCHOR_END: wrap_index

/// A growable, contiguous heap memory allocation that stores homogeneous elements.
///
/// This is a simplified version of [`RawVec`] inside Rust Standard Library.
/// Use at your own risk.
///
/// [`RawVec`]: https://github.com/rust-lang/rust/blob/ff6ee2a7/library/alloc/src/raw_vec.rs
#[derive(Debug)]
// ANCHOR: RawVec
struct RawVec<T> {
    ptr: *mut T,
    cap: usize,
}
// ANCHOR_END: RawVec

impl<T> RawVec<T> {
    /// Allocates on the heap with a certain capacity.
    ///
    /// Note that this does not support zero-sized allocations.
    /// For more, see [The Rustonomicon: Handling Zero-Sized Types][1].
    /// [1]: https://doc.rust-lang.org/nomicon/vec-zsts.html
    // ANCHOR: RawVec_with_capacity
    fn with_capacity(cap: usize) -> Self {
        let layout = Layout::array::<T>(cap).unwrap();
        assert!(layout.size() > 0, "Zero-sized allocation is not support");

        // This is safe because it conforms to the [safety contracts][1].
        //
        // [1] https://doc.rust-lang.org/1.49.0/alloc/alloc/trait.GlobalAlloc.html#safety-1
        let ptr = unsafe { alloc(layout).cast() };
        Self { ptr, cap }
    }
    // ANCHOR_END: RawVec_with_capacity

    // Doubles the size of the memory region to a certain capacity of elements.
    // ANCHOR: RawVec_resize
    fn grow(&mut self) {
        let new_cap = if self.cap == 0 { 1 } else { self.cap * 2 };
        let old_layout = Layout::array::<T>(self.cap).unwrap();
        // This is safe because it conforms to the [safety contracts][1].
        //
        // [1] https://doc.rust-lang.org/1.49.0/alloc/alloc/trait.GlobalAlloc.html#safety-4
        let ptr = unsafe { realloc(self.ptr.cast(), old_layout, old_layout.align() * new_cap) };
        // ...Old allocation is unusable and may be released from here.

        self.ptr = ptr.cast();
        self.cap = new_cap;
    }
    // ANCHOR_END: RawVec_resize
}

// ANCHOR: RawVec_drop
impl<T> Drop for RawVec<T> {
    /// Deallocates the underlying memory region by calculating the type layout
    /// and number of elements.
    ///
    /// This method only deallocates when containing actual sized elements.
    fn drop(&mut self) {
        let size = mem::size_of::<T>() * self.cap;
        if size > 0 {
            let align = mem::align_of::<T>();
            let layout = Layout::from_size_align(size, align).unwrap();
            // This is safe because it conforms to the [safety contracts][1].
            //
            // [1] https://doc.rust-lang.org/1.49.0/alloc/alloc/trait.GlobalAlloc.html#safety-2
            unsafe { dealloc(self.ptr.cast(), layout) }
        }
    }
}
// ANCHOR_END: RawVec_drop

#[cfg(test)]
mod deque {
    use super::Deque;

    #[test]
    fn push_pop() {
        let mut d = Deque::new();
        assert_eq!(d.len(), 0);
        assert_eq!(d.front(), None);
        assert_eq!(d.back(), None);

        d.push_back(1);
        d.push_back(2);
        // [1, 2]
        assert_eq!(d.len(), 2);
        assert_eq!(d.front(), Some(&1));
        assert_eq!(d.back(), Some(&2));

        d.push_front(3);
        d.push_front(4);
        // [4, 3, 1, 2]
        assert_eq!(d.len(), 4);
        assert_eq!(d.front(), Some(&4));
        assert_eq!(d.back(), Some(&2));

        assert_eq!(d.pop_front(), Some(4));
        assert_eq!(d.pop_front(), Some(3));
        assert_eq!(d.pop_front(), Some(1));
        assert_eq!(d.pop_front(), Some(2));
        assert_eq!(d.pop_front(), None);
        assert_eq!(d.len(), 0);
        assert_eq!(d.front(), None);
        assert_eq!(d.back(), None);

        d.push_front(5);
        d.push_front(6);
        // [6, 5]
        assert_eq!(d.len(), 2);
        assert_eq!(d.front(), Some(&6));
        assert_eq!(d.back(), Some(&5));

        assert_eq!(d.pop_back(), Some(5));
        assert_eq!(d.pop_back(), Some(6));
        assert_eq!(d.pop_back(), None);
        assert_eq!(d.len(), 0);
        assert_eq!(d.front(), None);
        assert_eq!(d.back(), None);
    }
}
