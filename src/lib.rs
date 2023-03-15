//! An asynchronous pool for managing reusable values.
//!
//! Values are retrieved from the pool asynchronously.
//! When the retrieved out value goes out of scope,
//! the value is returned to the pool.
//!
//! # Examples
//! ```
//! use tub::Pool;
//!
//! #[tokio::main]
//! async fn main() {
//!    // Create a pool
//!    let pool = Pool::from_initializer(10, || Box { value: 123 });
//!    assert_eq!(pool.remaining_capacity(), 10);
//!
//!    // Get a value from the pool
//!    let mut box1 = pool.acquire().await;
//!    assert_eq!(pool.remaining_capacity(), 9);
//!
//!    // Use the value
//!    box1.foo();
//!
//!    // Modify the value
//!    *box1 = Box { value: 456 };
//!    assert_eq!(box1.value, 456);
//!
//!    // Return the value to the pool
//!    drop(box1);
//!    assert_eq!(pool.remaining_capacity(), 10);
//! }
//!
//! struct Box {
//!   value: u32
//! }
//!
//! impl Box {
//!   fn foo(&mut self) { }
//! }
//! ```
use crossbeam_queue::ArrayQueue;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use tokio::sync::Notify;

/// A shared resource pool
#[derive(Clone)]
pub struct Pool<T> {
    inner: Arc<PoolInner<T>>,
}

struct PoolInner<T> {
    /// The queue of idle resources
    queue: ArrayQueue<T>,
    /// Notify waiting tasks
    notify: Notify,
}

/// A handle to a value from the pool
///
/// When the [`Guard`] is dropped, the value is returned to the pool
pub struct Guard<T> {
    /// A value from the pool
    /// Option is used to play nicely with borrowing rules
    value: Option<T>,
    /// A reference to the pool used to return the value when dropped
    inner: Arc<PoolInner<T>>,
}

impl<T: Default> Pool<T> {
    /// Create a new pool with a default value
    ///
    /// # Examples
    /// ```
    /// use tub::Pool;
    /// let pool: Pool<u32> = Pool::from_default(10);
    /// ```
    pub fn from_default(capacity: usize) -> Self
    where
        T: Default,
    {
        Pool::from_initializer(capacity, T::default)
    }
}

impl<T: Copy> Pool<T> {
    /// Create a new pool with a copy of a value
    ///
    /// # Examples
    /// ```
    /// use tub::Pool;
    /// let pool = Pool::from_copy(10, 123);
    /// ```
    pub fn from_copy(capacity: usize, value: T) -> Self
    where
        T: Copy,
    {
        Pool::from_initializer(capacity, move || value)
    }
}

impl<T: Clone> Pool<T> {
    /// Create a new pool with a clone of a value
    ///
    /// # Examples
    /// ```
    /// use tub::Pool;
    /// let pool = Pool::from_clone(10, &123);
    /// ```
    pub fn from_clone(capacity: usize, value: &T) -> Self
    where
        T: Clone,
    {
        Pool::from_initializer(capacity, move || value.clone())
    }
}

impl<T> Pool<T> {
    /// Create a new pool from an iterator
    ///
    /// # Examples
    /// ```
    /// use tub::Pool;
    /// let pool = Pool::from_iter(0..10);
    /// ```
    #[allow(clippy::should_implement_trait)]
    pub fn from_iter<I>(iterable: I) -> Self
    where
        I: IntoIterator<Item = T>,
    {
        Pool::from_vec(iterable.into_iter().collect())
    }

    /// Create a new pool from an initializer.
    ///
    /// The initializer is called once for each value in the pool.
    ///
    /// # Examples
    /// ```
    /// use std::sync::atomic::AtomicUsize;
    /// use std::sync::atomic::Ordering::SeqCst;
    /// use tub::Pool;
    /// let pool = Pool::from_initializer(10, || {
    ///     static COUNTER: AtomicUsize = AtomicUsize::new(0);
    ///     COUNTER.fetch_add(1, SeqCst);
    /// });
    /// ```
    pub fn from_initializer<F>(capacity: usize, init: F) -> Self
    where
        F: Fn() -> T,
    {
        let queue = ArrayQueue::new(capacity);

        for _ in 0..capacity {
            // Safety: The queue can hold every item we push
            let _ = queue.push(init());
        }

        Self {
            inner: Arc::new(PoolInner {
                queue,
                notify: Notify::new(),
            }),
        }
    }

    /// Create a new pool from a vector of values
    ///
    /// # Examples
    /// ```
    /// use tub::Pool;
    /// let pool = Pool::from_vec(vec![1, 2, 3]);
    /// ```
    pub fn from_vec(vec: Vec<T>) -> Self {
        let queue = ArrayQueue::new(vec.len());

        for item in vec {
            let _ = queue.push(item);
        }

        Self {
            inner: Arc::new(PoolInner {
                queue,
                notify: Notify::new(),
            }),
        }
    }

    /// Get the number of available values in the pool
    ///
    /// # Examples
    /// ```
    /// use tub::Pool;
    /// let pool = Pool::from_iter(0..10);
    /// assert_eq!(pool.remaining_capacity(), 10);
    /// ```
    pub fn remaining_capacity(&self) -> usize {
        self.inner.queue.len()
    }

    /// Acquire a value from the pool.
    ///
    /// The value is protected by a [`Guard`]
    ///
    /// # Examples
    /// ```
    /// use tub::Pool;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///    let pool: Pool<u32> = Pool::from_default(10);
    ///    let mut box1 = pool.acquire().await;
    ///    assert_eq!(pool.remaining_capacity(), 9);
    ///    assert_eq!(*box1, u32::default());
    /// }
    /// ```
    #[inline]
    pub async fn acquire(&self) -> Guard<T> {
        let inner = self.inner.clone();
        loop {
            if let Some(value) = inner.queue.pop() {
                return Guard {
                    value: Some(value),
                    inner,
                };
            }

            inner.notify.notified().await;
        }
    }
}

impl<T> Drop for Guard<T> {
    #[inline]
    fn drop(&mut self) {
        if let Some(value) = self.value.take() {
            // Safety: The queue will never be full when a Guard is alive
            let _ = self.inner.queue.push(value);
            self.inner.notify.notify_one();
        }
    }
}

impl<T> Deref for Guard<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // Safety: The value is always Some
        self.value.as_ref().unwrap()
    }
}

impl<T> DerefMut for Guard<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // Safety: The value is always Some
        self.value.as_mut().unwrap()
    }
}
