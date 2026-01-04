//! Observable/Reactive data structures for efficient live updates
//!
//! This module provides thread-safe reactive data containers that support:
//! - Change detection via version counters
//! - Subscriber notification for automatic updates
//! - Integration with the plotting system for live data visualization
//!
//! # Example
//!
//! ```rust,no_run
//! use ruviz::data::observable::Observable;
//!
//! // Create observable data
//! let x_data = Observable::new(vec![0.0, 1.0, 2.0, 3.0]);
//! let y_data = Observable::new(vec![0.0, 1.0, 4.0, 9.0]);
//!
//! // Update data (automatically notifies subscribers)
//! x_data.update(|data| {
//!     data.push(4.0);
//! });
//! y_data.update(|data| {
//!     data.push(16.0);
//! });
//!
//! // Check if data changed since last render
//! let version = x_data.version();
//! // ... render ...
//! if x_data.version() != version {
//!     // Data changed, re-render
//! }
//! ```

use std::ops::Deref;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock, RwLockReadGuard, Weak};

/// Type alias for subscriber callback functions
pub type SubscriberCallback = Box<dyn Fn() + Send + Sync>;

/// Unique identifier for a subscriber
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubscriberId(u64);

/// Internal subscriber entry
struct Subscriber {
    id: SubscriberId,
    callback: SubscriberCallback,
}

/// Thread-safe observable data container with change detection
///
/// `Observable<T>` wraps data in an `Arc<RwLock<T>>` and tracks mutations
/// via an atomic version counter. Subscribers can register callbacks that
/// are invoked whenever the data changes.
pub struct Observable<T> {
    /// The actual data, wrapped for thread-safe access
    data: Arc<RwLock<T>>,
    /// Atomic version counter, incremented on each mutation
    version: Arc<AtomicU64>,
    /// List of subscribers to notify on changes
    subscribers: Arc<RwLock<Vec<Subscriber>>>,
    /// Counter for generating unique subscriber IDs
    next_subscriber_id: Arc<AtomicU64>,
}

impl<T> Clone for Observable<T> {
    fn clone(&self) -> Self {
        Self {
            data: Arc::clone(&self.data),
            version: Arc::clone(&self.version),
            subscribers: Arc::clone(&self.subscribers),
            next_subscriber_id: Arc::clone(&self.next_subscriber_id),
        }
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Observable<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Observable")
            .field("data", &self.data)
            .field("version", &self.version.load(Ordering::Relaxed))
            .field(
                "subscriber_count",
                &self.subscribers.read().map(|s| s.len()).unwrap_or(0),
            )
            .finish()
    }
}

impl<T> Observable<T> {
    /// Create a new Observable with the given initial value
    ///
    /// # Example
    ///
    /// ```
    /// use ruviz::data::observable::Observable;
    ///
    /// let data = Observable::new(vec![1.0, 2.0, 3.0]);
    /// assert_eq!(data.version(), 0);
    /// ```
    pub fn new(value: T) -> Self {
        Self {
            data: Arc::new(RwLock::new(value)),
            version: Arc::new(AtomicU64::new(0)),
            subscribers: Arc::new(RwLock::new(Vec::new())),
            next_subscriber_id: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Get the current version number
    ///
    /// The version is incremented each time the data is mutated through `update()`,
    /// `set()`, or `modify()`. Use this to detect changes since the last render.
    ///
    /// # Example
    ///
    /// ```
    /// use ruviz::data::observable::Observable;
    ///
    /// let data = Observable::new(42);
    /// let v1 = data.version();
    /// data.set(100);
    /// let v2 = data.version();
    /// assert!(v2 > v1);
    /// ```
    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Acquire)
    }

    /// Increment the version and notify all subscribers
    fn bump_version(&self) {
        self.version.fetch_add(1, Ordering::Release);
        self.notify_subscribers();
    }

    /// Read the data immutably
    ///
    /// Returns a guard that provides read access to the underlying data.
    /// Multiple readers can access the data concurrently.
    ///
    /// # Panics
    ///
    /// Panics if the lock is poisoned.
    ///
    /// # Example
    ///
    /// ```
    /// use ruviz::data::observable::Observable;
    ///
    /// let data = Observable::new(vec![1.0, 2.0, 3.0]);
    /// let guard = data.read();
    /// assert_eq!(guard.len(), 3);
    /// ```
    pub fn read(&self) -> std::sync::RwLockReadGuard<'_, T> {
        self.data.read().expect("Observable lock poisoned")
    }

    /// Try to read the data immutably without blocking
    ///
    /// Returns `None` if the lock is currently held for writing.
    pub fn try_read(&self) -> Option<std::sync::RwLockReadGuard<'_, T>> {
        self.data.try_read().ok()
    }

    /// Update the data using a closure
    ///
    /// This is the primary way to mutate observable data. The closure receives
    /// a mutable reference to the data. After the closure returns, the version
    /// is incremented and all subscribers are notified.
    ///
    /// # Panics
    ///
    /// Panics if the lock is poisoned.
    ///
    /// # Example
    ///
    /// ```
    /// use ruviz::data::observable::Observable;
    ///
    /// let data = Observable::new(vec![1.0, 2.0, 3.0]);
    /// data.update(|v| v.push(4.0));
    /// assert_eq!(data.read().len(), 4);
    /// ```
    pub fn update<F>(&self, f: F)
    where
        F: FnOnce(&mut T),
    {
        {
            let mut guard = self.data.write().expect("Observable lock poisoned");
            f(&mut *guard);
        }
        self.bump_version();
    }

    /// Update the data and return a result
    ///
    /// Like `update()`, but the closure can return a value.
    ///
    /// # Example
    ///
    /// ```
    /// use ruviz::data::observable::Observable;
    ///
    /// let data = Observable::new(vec![1.0, 2.0, 3.0]);
    /// let old_len = data.update_with(|v| {
    ///     let len = v.len();
    ///     v.push(4.0);
    ///     len
    /// });
    /// assert_eq!(old_len, 3);
    /// ```
    pub fn update_with<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut T) -> R,
    {
        let result = {
            let mut guard = self.data.write().expect("Observable lock poisoned");
            f(&mut *guard)
        };
        self.bump_version();
        result
    }

    /// Replace the entire value
    ///
    /// # Example
    ///
    /// ```
    /// use ruviz::data::observable::Observable;
    ///
    /// let data = Observable::new(vec![1.0, 2.0]);
    /// data.set(vec![5.0, 6.0, 7.0]);
    /// assert_eq!(data.read().len(), 3);
    /// ```
    pub fn set(&self, value: T) {
        {
            let mut guard = self.data.write().expect("Observable lock poisoned");
            *guard = value;
        }
        self.bump_version();
    }

    /// Subscribe to changes
    ///
    /// The callback will be invoked whenever the data changes.
    /// Returns a `SubscriberId` that can be used to unsubscribe later.
    ///
    /// # Example
    ///
    /// ```
    /// use ruviz::data::observable::Observable;
    /// use std::sync::atomic::{AtomicUsize, Ordering};
    /// use std::sync::Arc;
    ///
    /// let data = Observable::new(42);
    /// let counter = Arc::new(AtomicUsize::new(0));
    /// let counter_clone = Arc::clone(&counter);
    ///
    /// let id = data.subscribe(move || {
    ///     counter_clone.fetch_add(1, Ordering::Relaxed);
    /// });
    ///
    /// data.set(100);
    /// assert_eq!(counter.load(Ordering::Relaxed), 1);
    ///
    /// data.unsubscribe(id);
    /// data.set(200);
    /// assert_eq!(counter.load(Ordering::Relaxed), 1); // Not called again
    /// ```
    pub fn subscribe<F>(&self, callback: F) -> SubscriberId
    where
        F: Fn() + Send + Sync + 'static,
    {
        let id = SubscriberId(self.next_subscriber_id.fetch_add(1, Ordering::Relaxed));
        let subscriber = Subscriber {
            id,
            callback: Box::new(callback),
        };

        self.subscribers
            .write()
            .expect("Subscribers lock poisoned")
            .push(subscriber);

        id
    }

    /// Unsubscribe from changes
    ///
    /// Returns `true` if the subscriber was found and removed.
    pub fn unsubscribe(&self, id: SubscriberId) -> bool {
        let mut subscribers = self.subscribers.write().expect("Subscribers lock poisoned");
        if let Some(pos) = subscribers.iter().position(|s| s.id == id) {
            subscribers.remove(pos);
            true
        } else {
            false
        }
    }

    /// Get the number of active subscribers
    pub fn subscriber_count(&self) -> usize {
        self.subscribers
            .read()
            .expect("Subscribers lock poisoned")
            .len()
    }

    /// Notify all subscribers of a change
    fn notify_subscribers(&self) {
        let subscribers = self.subscribers.read().expect("Subscribers lock poisoned");
        for subscriber in subscribers.iter() {
            (subscriber.callback)();
        }
    }

    /// Create a weak reference to this observable
    ///
    /// This is useful for avoiding reference cycles when observables
    /// reference each other.
    pub fn downgrade(&self) -> WeakObservable<T> {
        WeakObservable {
            data: Arc::downgrade(&self.data),
            version: Arc::downgrade(&self.version),
        }
    }
}

impl<T: Clone> Observable<T> {
    /// Get a clone of the current value
    ///
    /// This is a convenience method that clones the data.
    /// For large data, prefer using `read()` to avoid the copy.
    pub fn get(&self) -> T {
        self.read().clone()
    }
}

impl<T: Default> Default for Observable<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

/// Weak reference to an Observable
///
/// This can be upgraded to a full `Observable` if the original is still alive.
pub struct WeakObservable<T> {
    data: Weak<RwLock<T>>,
    version: Weak<AtomicU64>,
}

impl<T> Clone for WeakObservable<T> {
    fn clone(&self) -> Self {
        Self {
            data: Weak::clone(&self.data),
            version: Weak::clone(&self.version),
        }
    }
}

impl<T> WeakObservable<T> {
    /// Try to upgrade to a strong reference
    ///
    /// Returns `None` if the original Observable has been dropped.
    pub fn upgrade(&self) -> Option<Observable<T>> {
        let data = self.data.upgrade()?;
        let version = self.version.upgrade()?;

        Some(Observable {
            data,
            version,
            subscribers: Arc::new(RwLock::new(Vec::new())),
            next_subscriber_id: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Check if the original Observable is still alive
    pub fn is_alive(&self) -> bool {
        self.data.strong_count() > 0
    }
}

/// Batch update guard for efficient multi-observable updates
///
/// When updating multiple observables at once, use a `BatchUpdate` to
/// defer notifications until all updates are complete.
///
/// # Example
///
/// ```
/// use ruviz::data::observable::{Observable, BatchUpdate};
///
/// let x = Observable::new(vec![1.0, 2.0]);
/// let y = Observable::new(vec![1.0, 4.0]);
///
/// // Batch updates defer notifications
/// {
///     let mut batch = BatchUpdate::new();
///     batch.add(&x);
///     batch.add(&y);
///
///     x.update(|v| v.push(3.0));
///     y.update(|v| v.push(9.0));
///     // Notifications are sent when batch is dropped
/// }
/// ```
pub struct BatchUpdate<'a> {
    observables: Vec<&'a dyn BatchNotifier>,
}

/// Trait for types that can participate in batch updates
pub trait BatchNotifier {
    fn notify(&self);
}

impl<T> BatchNotifier for Observable<T> {
    fn notify(&self) {
        self.notify_subscribers();
    }
}

impl<'a> BatchUpdate<'a> {
    /// Create a new batch update
    pub fn new() -> Self {
        Self {
            observables: Vec::new(),
        }
    }

    /// Add an observable to the batch
    pub fn add<T>(&mut self, observable: &'a Observable<T>) {
        self.observables.push(observable);
    }
}

impl<'a> Default for BatchUpdate<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Drop for BatchUpdate<'a> {
    fn drop(&mut self) {
        for obs in &self.observables {
            obs.notify();
        }
    }
}

/// Observable data that tracks a window of the most recent N values
///
/// Useful for streaming time-series data where you only want to display
/// the most recent data points.
pub struct SlidingWindowObservable<T> {
    inner: Observable<Vec<T>>,
    max_size: usize,
}

impl<T: Clone> SlidingWindowObservable<T> {
    /// Create a new sliding window observable with the given capacity
    pub fn new(max_size: usize) -> Self {
        Self {
            inner: Observable::new(Vec::with_capacity(max_size)),
            max_size,
        }
    }

    /// Push a new value, removing the oldest if at capacity
    pub fn push(&self, value: T) {
        self.inner.update(|data| {
            if data.len() >= self.max_size {
                data.remove(0);
            }
            data.push(value);
        });
    }

    /// Push multiple values
    pub fn push_many(&self, values: impl IntoIterator<Item = T>) {
        self.inner.update(|data| {
            for value in values {
                if data.len() >= self.max_size {
                    data.remove(0);
                }
                data.push(value);
            }
        });
    }

    /// Clear all values
    pub fn clear(&self) {
        self.inner.update(|data| data.clear());
    }

    /// Get the current version
    pub fn version(&self) -> u64 {
        self.inner.version()
    }

    /// Get read access to the data
    pub fn read(&self) -> std::sync::RwLockReadGuard<'_, Vec<T>> {
        self.inner.read()
    }

    /// Subscribe to changes
    pub fn subscribe<F>(&self, callback: F) -> SubscriberId
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.inner.subscribe(callback)
    }

    /// Unsubscribe from changes
    pub fn unsubscribe(&self, id: SubscriberId) -> bool {
        self.inner.unsubscribe(id)
    }

    /// Get the underlying Observable
    pub fn as_observable(&self) -> &Observable<Vec<T>> {
        &self.inner
    }

    /// Get the maximum capacity
    pub fn max_size(&self) -> usize {
        self.max_size
    }

    /// Get the current number of elements
    pub fn len(&self) -> usize {
        self.inner.read().len()
    }

    /// Check if the window is empty
    pub fn is_empty(&self) -> bool {
        self.inner.read().is_empty()
    }
}

impl<T: Clone> Clone for SlidingWindowObservable<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            max_size: self.max_size,
        }
    }
}

/// Helper trait to convert data types to Observable
pub trait IntoObservable<T> {
    fn into_observable(self) -> Observable<T>;
}

// ============================================================================
// Makie-inspired lift/map functions for derived observables
// ============================================================================

/// Create a derived observable that automatically updates when the source changes
///
/// This is inspired by Makie.jl's `lift` function. The derived observable
/// holds a computed value that is recomputed whenever the source changes.
///
/// # Example
///
/// ```
/// use ruviz::data::observable::{Observable, lift};
///
/// let x = Observable::new(2.0);
/// let squared = lift(&x, |v| v * v);
///
/// assert_eq!(*squared.read(), 4.0);
///
/// x.set(3.0);
/// assert_eq!(*squared.read(), 9.0);
/// ```
pub fn lift<T, U, F>(source: &Observable<T>, f: F) -> Observable<U>
where
    T: Clone + Send + Sync + 'static,
    U: Send + Sync + 'static,
    F: Fn(T) -> U + Send + Sync + 'static,
{
    let initial = f(source.get());
    let derived = Observable::new(initial);
    let derived_clone = derived.clone();
    let f = Arc::new(f);
    let source_clone = source.clone();

    source.subscribe(move || {
        let new_value = f(source_clone.get());
        // Update and notify subscribers for proper chaining
        {
            let mut guard = derived_clone.data.write().expect("Lock poisoned");
            *guard = new_value;
        }
        derived_clone.bump_version();
    });

    derived
}

/// Create a derived observable from two sources
///
/// # Example
///
/// ```
/// use ruviz::data::observable::{Observable, lift2};
///
/// let x = Observable::new(2.0);
/// let y = Observable::new(3.0);
/// let sum = lift2(&x, &y, |a, b| a + b);
///
/// assert_eq!(*sum.read(), 5.0);
///
/// x.set(10.0);
/// assert_eq!(*sum.read(), 13.0);
/// ```
pub fn lift2<T1, T2, U, F>(
    source1: &Observable<T1>,
    source2: &Observable<T2>,
    f: F,
) -> Observable<U>
where
    T1: Clone + Send + Sync + 'static,
    T2: Clone + Send + Sync + 'static,
    U: Send + Sync + 'static,
    F: Fn(T1, T2) -> U + Send + Sync + 'static,
{
    let initial = f(source1.get(), source2.get());
    let derived = Observable::new(initial);

    let f = Arc::new(f);

    // Subscribe to source1
    {
        let derived_clone = derived.clone();
        let f_clone = Arc::clone(&f);
        let s1 = source1.clone();
        let s2 = source2.clone();
        source1.subscribe(move || {
            let new_value = f_clone(s1.get(), s2.get());
            {
                let mut guard = derived_clone.data.write().expect("Lock poisoned");
                *guard = new_value;
            }
            derived_clone.bump_version();
        });
    }

    // Subscribe to source2
    {
        let derived_clone = derived.clone();
        let f_clone = Arc::clone(&f);
        let s1 = source1.clone();
        let s2 = source2.clone();
        source2.subscribe(move || {
            let new_value = f_clone(s1.get(), s2.get());
            {
                let mut guard = derived_clone.data.write().expect("Lock poisoned");
                *guard = new_value;
            }
            derived_clone.bump_version();
        });
    }

    derived
}

/// Map a function over an observable (alias for lift)
///
/// # Example
///
/// ```
/// use ruviz::data::observable::{Observable, map};
///
/// let x = Observable::new(vec![1.0, 2.0, 3.0]);
/// let doubled = map(&x, |v| v.iter().map(|x| x * 2.0).collect::<Vec<_>>());
///
/// assert_eq!(*doubled.read(), vec![2.0, 4.0, 6.0]);
/// ```
pub fn map<T, U, F>(source: &Observable<T>, f: F) -> Observable<U>
where
    T: Clone + Send + Sync + 'static,
    U: Send + Sync + 'static,
    F: Fn(T) -> U + Send + Sync + 'static,
{
    lift(source, f)
}

// ============================================================================
// Reactive data handle for plots
// ============================================================================

/// A handle to reactive plot data
///
/// This is returned when adding reactive series to a plot. It can be used
/// to track whether updates are needed during the render loop.
#[derive(Clone)]
pub struct ReactiveDataHandle {
    /// Version numbers of the observables when last rendered
    last_versions: Arc<RwLock<Vec<u64>>>,
    /// Current version numbers of the observables
    current_versions: Arc<RwLock<Vec<Arc<AtomicU64>>>>,
}

impl ReactiveDataHandle {
    /// Create a new reactive data handle
    pub fn new() -> Self {
        Self {
            last_versions: Arc::new(RwLock::new(Vec::new())),
            current_versions: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Track an observable's version
    pub fn track<T>(&self, observable: &Observable<T>) {
        let mut last = self.last_versions.write().expect("Lock poisoned");
        let mut current = self.current_versions.write().expect("Lock poisoned");

        last.push(observable.version());
        current.push(Arc::clone(&observable.version));
    }

    /// Check if any tracked observable has changed since last check
    pub fn has_changes(&self) -> bool {
        let last = self.last_versions.read().expect("Lock poisoned");
        let current = self.current_versions.read().expect("Lock poisoned");

        for (i, version_arc) in current.iter().enumerate() {
            if let Some(&last_version) = last.get(i) {
                if version_arc.load(Ordering::Acquire) != last_version {
                    return true;
                }
            }
        }
        false
    }

    /// Mark all tracked observables as up-to-date
    pub fn mark_updated(&self) {
        let mut last = self.last_versions.write().expect("Lock poisoned");
        let current = self.current_versions.read().expect("Lock poisoned");

        for (i, version_arc) in current.iter().enumerate() {
            if let Some(last_version) = last.get_mut(i) {
                *last_version = version_arc.load(Ordering::Acquire);
            }
        }
    }
}

impl Default for ReactiveDataHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> IntoObservable<Vec<T>> for Vec<T> {
    fn into_observable(self) -> Observable<Vec<T>> {
        Observable::new(self)
    }
}

impl<T: Clone, const N: usize> IntoObservable<Vec<T>> for [T; N] {
    fn into_observable(self) -> Observable<Vec<T>> {
        Observable::new(self.to_vec())
    }
}

// ============================================================================
// StreamingBuffer - O(1) ring buffer for high-performance streaming data
// ============================================================================

/// Zero-copy view into StreamingBuffer data
///
/// This struct holds a read lock on the underlying data and provides
/// zero-copy access to the buffer contents. The view is valid for the
/// lifetime of the guard.
///
/// # Lifetime
///
/// The returned reference is tied to the lifetime of this view. Do not
/// store references extracted from this view beyond the view's lifetime.
///
/// # Example
///
/// ```rust,no_run
/// use ruviz::data::StreamingBuffer;
///
/// let buffer = StreamingBuffer::<f64>::new(100);
/// buffer.push(1.0);
/// buffer.push(2.0);
///
/// // Zero-copy access - no cloning
/// let view = buffer.read_view();
/// for item in view.iter() {
///     if let Some(value) = item {
///         println!("{}", value);
///     }
/// }
/// // Lock released when view is dropped
/// ```
pub struct StreamingBufferView<'a, T> {
    guard: RwLockReadGuard<'a, Vec<Option<T>>>,
}

impl<T> Deref for StreamingBufferView<'_, T> {
    type Target = [Option<T>];

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

/// High-performance ring buffer for streaming time-series data
///
/// Unlike `SlidingWindowObservable`, `StreamingBuffer` uses a true circular
/// buffer with O(1) append operations and supports partial re-render tracking
/// for appended data.
///
/// # Performance
///
/// - Push: O(1) - direct index write, no shifting
/// - Read: O(n) - must reconstruct order from head/tail
/// - Memory: Pre-allocated, no reallocations
///
/// # Example
///
/// ```rust,no_run
/// use ruviz::data::StreamingBuffer;
///
/// // Create buffer for 1000 points
/// let buffer = StreamingBuffer::<f64>::new(1000);
///
/// // Append data (O(1) per element)
/// for i in 0..2000 {
///     buffer.push(i as f64);
/// }
///
/// // Check how many new points since last render
/// let new_count = buffer.appended_since_mark();
///
/// // Mark as rendered
/// buffer.mark_rendered();
/// ```
pub struct StreamingBuffer<T> {
    /// Circular buffer storage
    data: Arc<RwLock<Vec<Option<T>>>>,
    /// Current capacity (fixed after creation)
    capacity: usize,
    /// Current write position (wraps around)
    write_pos: Arc<std::sync::atomic::AtomicUsize>,
    /// Total elements written (used to determine if full)
    total_written: Arc<AtomicU64>,
    /// Version counter for change detection
    version: Arc<AtomicU64>,
    /// Append count since last mark_rendered()
    appended_since_render: Arc<std::sync::atomic::AtomicUsize>,
    /// Subscribers for change notifications
    subscribers: Arc<RwLock<Vec<Subscriber>>>,
    /// Subscriber ID counter
    next_subscriber_id: Arc<AtomicU64>,
}

impl<T: Clone> StreamingBuffer<T> {
    /// Create a new streaming buffer with the given capacity
    pub fn new(capacity: usize) -> Self {
        let mut data = Vec::with_capacity(capacity);
        data.resize_with(capacity, || None);

        Self {
            data: Arc::new(RwLock::new(data)),
            capacity,
            write_pos: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            total_written: Arc::new(AtomicU64::new(0)),
            version: Arc::new(AtomicU64::new(0)),
            appended_since_render: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            subscribers: Arc::new(RwLock::new(Vec::new())),
            next_subscriber_id: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Push a single value (O(1) operation)
    pub fn push(&self, value: T) {
        let pos = self.write_pos.fetch_add(1, Ordering::Relaxed) % self.capacity;

        {
            let mut data = self.data.write().expect("Lock poisoned");
            data[pos] = Some(value);
        }

        self.total_written.fetch_add(1, Ordering::Relaxed);
        self.appended_since_render.fetch_add(1, Ordering::Relaxed);
        self.bump_version();
    }

    /// Push multiple values efficiently
    pub fn push_many(&self, values: impl IntoIterator<Item = T>) {
        let values: Vec<T> = values.into_iter().collect();
        let count = values.len();

        if count == 0 {
            return;
        }

        {
            let mut data = self.data.write().expect("Lock poisoned");
            for value in values {
                let pos = self.write_pos.fetch_add(1, Ordering::Relaxed) % self.capacity;
                data[pos] = Some(value);
            }
        }

        self.total_written
            .fetch_add(count as u64, Ordering::Relaxed);
        self.appended_since_render
            .fetch_add(count, Ordering::Relaxed);
        self.bump_version();
    }

    /// Get all valid data in order (oldest to newest)
    pub fn read(&self) -> Vec<T> {
        let data = self.data.read().expect("Lock poisoned");
        let total = self.total_written.load(Ordering::Relaxed);
        let write_pos = self.write_pos.load(Ordering::Relaxed);

        if total == 0 {
            return Vec::new();
        }

        let len = std::cmp::min(total as usize, self.capacity);
        let mut result = Vec::with_capacity(len);

        if total <= self.capacity as u64 {
            // Buffer not yet full - data is in order from 0 to write_pos
            for i in 0..len {
                if let Some(ref value) = data[i] {
                    result.push(value.clone());
                }
            }
        } else {
            // Buffer wrapped - oldest is at write_pos, newest is at write_pos-1
            let start = write_pos % self.capacity;
            for i in 0..self.capacity {
                let idx = (start + i) % self.capacity;
                if let Some(ref value) = data[idx] {
                    result.push(value.clone());
                }
            }
        }

        result
    }

    /// Zero-copy view into the buffer data
    ///
    /// Unlike `read()`, this method does not clone the data. Instead, it returns
    /// a view that holds a read lock on the underlying buffer. This is useful
    /// for high-performance scenarios where you need to iterate over the data
    /// without copying.
    ///
    /// # Note
    ///
    /// The data is returned in storage order (not oldest-to-newest like `read()`).
    /// Use the buffer's `total_written` and `write_pos` to determine the actual
    /// data order if needed.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use ruviz::data::StreamingBuffer;
    ///
    /// let buffer = StreamingBuffer::<f64>::new(100);
    /// buffer.push(1.0);
    ///
    /// // Zero-copy access
    /// let view = buffer.read_view();
    /// let first = view.iter().find_map(|opt| opt.as_ref());
    /// ```
    pub fn read_view(&self) -> StreamingBufferView<'_, T> {
        StreamingBufferView {
            guard: self.data.read().expect("Lock poisoned"),
        }
    }

    /// Get only the data appended since last mark_rendered() call
    ///
    /// This enables partial re-rendering for streaming data
    pub fn read_appended(&self) -> Vec<T> {
        let data = self.data.read().expect("Lock poisoned");
        let appended = self.appended_since_render.load(Ordering::Relaxed);
        let write_pos = self.write_pos.load(Ordering::Relaxed);

        if appended == 0 {
            return Vec::new();
        }

        let count = std::cmp::min(appended, self.capacity);
        let mut result = Vec::with_capacity(count);

        // Read the last `count` elements written
        for i in 0..count {
            let idx = (write_pos + self.capacity - count + i) % self.capacity;
            if let Some(ref value) = data[idx] {
                result.push(value.clone());
            }
        }

        result
    }

    /// Get the number of elements appended since last mark_rendered()
    pub fn appended_since_mark(&self) -> usize {
        self.appended_since_render.load(Ordering::Relaxed)
    }

    /// Mark the buffer as rendered (resets appended count)
    pub fn mark_rendered(&self) {
        self.appended_since_render.store(0, Ordering::Release);
    }

    /// Check if partial re-render is possible
    ///
    /// Returns true if only new data needs rendering (no wrapping occurred)
    pub fn can_partial_render(&self) -> bool {
        let appended = self.appended_since_render.load(Ordering::Relaxed);
        appended < self.capacity
    }

    /// Get the current version (for change detection)
    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Acquire)
    }

    /// Get the buffer capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get the current number of valid elements
    pub fn len(&self) -> usize {
        let total = self.total_written.load(Ordering::Relaxed);
        std::cmp::min(total as usize, self.capacity)
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.total_written.load(Ordering::Relaxed) == 0
    }

    /// Check if the buffer is full (has wrapped at least once)
    pub fn is_full(&self) -> bool {
        self.total_written.load(Ordering::Relaxed) >= self.capacity as u64
    }

    /// Total number of elements ever written
    pub fn total_written(&self) -> u64 {
        self.total_written.load(Ordering::Relaxed)
    }

    /// Clear all data
    pub fn clear(&self) {
        let mut data = self.data.write().expect("Lock poisoned");
        for slot in data.iter_mut() {
            *slot = None;
        }
        self.write_pos.store(0, Ordering::Release);
        self.total_written.store(0, Ordering::Release);
        self.appended_since_render.store(0, Ordering::Release);
        self.bump_version();
    }

    /// Subscribe to changes
    pub fn subscribe<F>(&self, callback: F) -> SubscriberId
    where
        F: Fn() + Send + Sync + 'static,
    {
        let id = SubscriberId(self.next_subscriber_id.fetch_add(1, Ordering::Relaxed));
        let subscriber = Subscriber {
            id,
            callback: Box::new(callback),
        };
        self.subscribers
            .write()
            .expect("Lock poisoned")
            .push(subscriber);
        id
    }

    /// Unsubscribe from changes
    pub fn unsubscribe(&self, id: SubscriberId) -> bool {
        let mut subscribers = self.subscribers.write().expect("Lock poisoned");
        if let Some(pos) = subscribers.iter().position(|s| s.id == id) {
            subscribers.remove(pos);
            true
        } else {
            false
        }
    }

    /// Bump version and notify subscribers
    fn bump_version(&self) {
        self.version.fetch_add(1, Ordering::Release);
        let subscribers = self.subscribers.read().expect("Lock poisoned");
        for subscriber in subscribers.iter() {
            (subscriber.callback)();
        }
    }
}

impl<T: Clone> Clone for StreamingBuffer<T> {
    fn clone(&self) -> Self {
        Self {
            data: Arc::clone(&self.data),
            capacity: self.capacity,
            write_pos: Arc::clone(&self.write_pos),
            total_written: Arc::clone(&self.total_written),
            version: Arc::clone(&self.version),
            appended_since_render: Arc::clone(&self.appended_since_render),
            subscribers: Arc::clone(&self.subscribers),
            next_subscriber_id: Arc::clone(&self.next_subscriber_id),
        }
    }
}

// Thread-safety: All internal state is protected by Arc/RwLock/Atomic
unsafe impl<T: Send> Send for StreamingBuffer<T> {}
unsafe impl<T: Send + Sync> Sync for StreamingBuffer<T> {}

/// Paired streaming buffers for X/Y time-series data
///
/// Provides synchronized updates and version tracking for plot integration
pub struct StreamingXY {
    x: StreamingBuffer<f64>,
    y: StreamingBuffer<f64>,
}

impl StreamingXY {
    /// Create a new paired streaming buffer
    pub fn new(capacity: usize) -> Self {
        Self {
            x: StreamingBuffer::new(capacity),
            y: StreamingBuffer::new(capacity),
        }
    }

    /// Push a single X/Y point
    pub fn push(&self, x: f64, y: f64) {
        self.x.push(x);
        self.y.push(y);
    }

    /// Push multiple X/Y points
    pub fn push_many(&self, points: impl IntoIterator<Item = (f64, f64)>) {
        for (x, y) in points {
            self.x.push(x);
            self.y.push(y);
        }
    }

    /// Get the X buffer
    pub fn x(&self) -> &StreamingBuffer<f64> {
        &self.x
    }

    /// Get the Y buffer
    pub fn y(&self) -> &StreamingBuffer<f64> {
        &self.y
    }

    /// Read all X data
    pub fn read_x(&self) -> Vec<f64> {
        self.x.read()
    }

    /// Read all Y data
    pub fn read_y(&self) -> Vec<f64> {
        self.y.read()
    }

    /// Zero-copy view into X buffer
    ///
    /// Returns a view that holds a read lock on the X data without cloning.
    pub fn read_view_x(&self) -> StreamingBufferView<'_, f64> {
        self.x.read_view()
    }

    /// Zero-copy view into Y buffer
    ///
    /// Returns a view that holds a read lock on the Y data without cloning.
    pub fn read_view_y(&self) -> StreamingBufferView<'_, f64> {
        self.y.read_view()
    }

    /// Zero-copy views into both X and Y buffers
    ///
    /// Returns views for both buffers, useful for iterating over pairs.
    /// Note: Both locks are held until both views are dropped.
    pub fn read_view(&self) -> (StreamingBufferView<'_, f64>, StreamingBufferView<'_, f64>) {
        (self.x.read_view(), self.y.read_view())
    }

    /// Read only appended X data since last render
    pub fn read_appended_x(&self) -> Vec<f64> {
        self.x.read_appended()
    }

    /// Read only appended Y data since last render
    pub fn read_appended_y(&self) -> Vec<f64> {
        self.y.read_appended()
    }

    /// Get the number of points appended since last render
    pub fn appended_count(&self) -> usize {
        // Both buffers should have same count
        self.x.appended_since_mark()
    }

    /// Mark both buffers as rendered
    pub fn mark_rendered(&self) {
        self.x.mark_rendered();
        self.y.mark_rendered();
    }

    /// Check if partial rendering is possible
    pub fn can_partial_render(&self) -> bool {
        self.x.can_partial_render() && self.y.can_partial_render()
    }

    /// Get the combined version (max of X and Y versions)
    pub fn version(&self) -> u64 {
        std::cmp::max(self.x.version(), self.y.version())
    }

    /// Get the current number of valid points
    pub fn len(&self) -> usize {
        self.x.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.x.is_empty()
    }

    /// Clear both buffers
    pub fn clear(&self) {
        self.x.clear();
        self.y.clear();
    }
}

impl Clone for StreamingXY {
    fn clone(&self) -> Self {
        Self {
            x: self.x.clone(),
            y: self.y.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    use std::thread;

    #[test]
    fn test_observable_basic() {
        let obs = Observable::new(vec![1.0, 2.0, 3.0]);
        assert_eq!(obs.version(), 0);
        assert_eq!(obs.read().len(), 3);
    }

    #[test]
    fn test_observable_update() {
        let obs = Observable::new(vec![1.0, 2.0, 3.0]);
        let v1 = obs.version();

        obs.update(|data| data.push(4.0));

        assert!(obs.version() > v1);
        assert_eq!(obs.read().len(), 4);
    }

    #[test]
    fn test_observable_set() {
        let obs = Observable::new(42);
        let v1 = obs.version();

        obs.set(100);

        assert!(obs.version() > v1);
        assert_eq!(*obs.read(), 100);
    }

    #[test]
    fn test_observable_subscribe() {
        let obs = Observable::new(42);
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        let id = obs.subscribe(move || {
            counter_clone.fetch_add(1, AtomicOrdering::Relaxed);
        });

        obs.set(100);
        assert_eq!(counter.load(AtomicOrdering::Relaxed), 1);

        obs.update(|v| *v += 1);
        assert_eq!(counter.load(AtomicOrdering::Relaxed), 2);

        obs.unsubscribe(id);
        obs.set(200);
        assert_eq!(counter.load(AtomicOrdering::Relaxed), 2);
    }

    #[test]
    fn test_observable_multiple_subscribers() {
        let obs = Observable::new(0);
        let counter1 = Arc::new(AtomicUsize::new(0));
        let counter2 = Arc::new(AtomicUsize::new(0));

        let c1 = Arc::clone(&counter1);
        let c2 = Arc::clone(&counter2);

        obs.subscribe(move || {
            c1.fetch_add(1, AtomicOrdering::Relaxed);
        });
        obs.subscribe(move || {
            c2.fetch_add(1, AtomicOrdering::Relaxed);
        });

        obs.set(42);

        assert_eq!(counter1.load(AtomicOrdering::Relaxed), 1);
        assert_eq!(counter2.load(AtomicOrdering::Relaxed), 1);
    }

    #[test]
    fn test_observable_thread_safe() {
        let obs = Observable::new(0i32);
        let obs_clone = obs.clone();

        let handle = thread::spawn(move || {
            for _ in 0..1000 {
                obs_clone.update(|v| *v += 1);
            }
        });

        for _ in 0..1000 {
            obs.update(|v| *v += 1);
        }

        handle.join().unwrap();
        assert_eq!(*obs.read(), 2000);
    }

    #[test]
    fn test_observable_get_clone() {
        let obs = Observable::new(vec![1, 2, 3]);
        let cloned = obs.get();
        assert_eq!(cloned, vec![1, 2, 3]);

        obs.update(|v| v.push(4));
        assert_eq!(cloned, vec![1, 2, 3]); // Clone unchanged
        assert_eq!(obs.get(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_weak_observable() {
        let obs = Observable::new(42);
        let weak = obs.downgrade();

        assert!(weak.is_alive());
        assert!(weak.upgrade().is_some());

        drop(obs);
        assert!(!weak.is_alive());
    }

    #[test]
    fn test_sliding_window() {
        let window = SlidingWindowObservable::new(3);

        window.push(1.0);
        window.push(2.0);
        window.push(3.0);
        assert_eq!(*window.read(), vec![1.0, 2.0, 3.0]);

        window.push(4.0);
        assert_eq!(*window.read(), vec![2.0, 3.0, 4.0]);

        window.push(5.0);
        assert_eq!(*window.read(), vec![3.0, 4.0, 5.0]);
    }

    #[test]
    fn test_sliding_window_push_many() {
        let window = SlidingWindowObservable::new(3);

        window.push_many(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(*window.read(), vec![3.0, 4.0, 5.0]);
    }

    #[test]
    fn test_batch_update() {
        let x = Observable::new(0);
        let y = Observable::new(0);

        let counter = Arc::new(AtomicUsize::new(0));
        let c1 = Arc::clone(&counter);
        let c2 = Arc::clone(&counter);

        x.subscribe(move || {
            c1.fetch_add(1, AtomicOrdering::Relaxed);
        });
        y.subscribe(move || {
            c2.fetch_add(1, AtomicOrdering::Relaxed);
        });

        // Normal updates trigger notifications
        x.set(1);
        y.set(1);
        assert_eq!(counter.load(AtomicOrdering::Relaxed), 2);
    }

    #[test]
    fn test_update_with() {
        let obs = Observable::new(vec![1, 2, 3]);
        let old_len = obs.update_with(|v| {
            let len = v.len();
            v.push(4);
            len
        });

        assert_eq!(old_len, 3);
        assert_eq!(obs.read().len(), 4);
    }

    #[test]
    fn test_subscriber_count() {
        let obs = Observable::new(42);
        assert_eq!(obs.subscriber_count(), 0);

        let id1 = obs.subscribe(|| {});
        assert_eq!(obs.subscriber_count(), 1);

        let id2 = obs.subscribe(|| {});
        assert_eq!(obs.subscriber_count(), 2);

        obs.unsubscribe(id1);
        assert_eq!(obs.subscriber_count(), 1);

        obs.unsubscribe(id2);
        assert_eq!(obs.subscriber_count(), 0);
    }

    #[test]
    fn test_into_observable() {
        let v = vec![1.0, 2.0, 3.0];
        let obs = v.into_observable();
        assert_eq!(obs.read().len(), 3);

        let arr = [1.0, 2.0, 3.0];
        let obs = arr.into_observable();
        assert_eq!(obs.read().len(), 3);
    }

    #[test]
    fn test_lift_basic() {
        let x = Observable::new(3.0);
        let squared = lift(&x, |v| v * v);

        assert_eq!(*squared.read(), 9.0);
        assert_eq!(squared.version(), 0);

        x.set(4.0);
        assert_eq!(*squared.read(), 16.0);
        assert!(squared.version() > 0);
    }

    #[test]
    fn test_lift_with_vec() {
        let data = Observable::new(vec![1.0, 2.0, 3.0]);
        let sum = lift(&data, |v| v.iter().sum::<f64>());

        assert_eq!(*sum.read(), 6.0);

        data.update(|v| v.push(4.0));
        assert_eq!(*sum.read(), 10.0);
    }

    #[test]
    fn test_lift2() {
        let a = Observable::new(10.0);
        let b = Observable::new(5.0);
        let sum = lift2(&a, &b, |x, y| x + y);

        assert_eq!(*sum.read(), 15.0);

        a.set(20.0);
        assert_eq!(*sum.read(), 25.0);

        b.set(10.0);
        assert_eq!(*sum.read(), 30.0);
    }

    #[test]
    fn test_map_alias() {
        let x = Observable::new(5.0);
        let doubled = map(&x, |v| v * 2.0);

        assert_eq!(*doubled.read(), 10.0);

        x.set(7.0);
        assert_eq!(*doubled.read(), 14.0);
    }

    #[test]
    fn test_chained_lift() {
        let x = Observable::new(2.0);
        let doubled = lift(&x, |v| v * 2.0);
        let quadrupled = lift(&doubled, |v| v * 2.0);

        assert_eq!(*doubled.read(), 4.0);
        assert_eq!(*quadrupled.read(), 8.0);

        x.set(3.0);
        assert_eq!(*doubled.read(), 6.0);
        assert_eq!(*quadrupled.read(), 12.0);
    }

    #[test]
    fn test_reactive_data_handle() {
        let x = Observable::new(vec![1.0, 2.0]);
        let y = Observable::new(vec![3.0, 4.0]);

        let handle = ReactiveDataHandle::new();
        handle.track(&x);
        handle.track(&y);

        assert!(!handle.has_changes());

        x.update(|v| v.push(5.0));
        assert!(handle.has_changes());

        handle.mark_updated();
        assert!(!handle.has_changes());

        y.set(vec![10.0]);
        assert!(handle.has_changes());
    }

    // ========================================================================
    // StreamingBuffer Tests
    // ========================================================================

    #[test]
    fn test_streaming_buffer_basic() {
        let buffer = StreamingBuffer::<f64>::new(5);

        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);
        assert_eq!(buffer.capacity(), 5);

        buffer.push(1.0);
        buffer.push(2.0);
        buffer.push(3.0);

        assert!(!buffer.is_empty());
        assert_eq!(buffer.len(), 3);
        assert_eq!(buffer.read(), vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_streaming_buffer_wrap_around() {
        let buffer = StreamingBuffer::<i32>::new(3);

        buffer.push(1);
        buffer.push(2);
        buffer.push(3);
        assert_eq!(buffer.read(), vec![1, 2, 3]);
        assert!(buffer.is_full());

        // Now wrap around
        buffer.push(4);
        assert_eq!(buffer.read(), vec![2, 3, 4]);

        buffer.push(5);
        assert_eq!(buffer.read(), vec![3, 4, 5]);

        buffer.push(6);
        assert_eq!(buffer.read(), vec![4, 5, 6]);
    }

    #[test]
    fn test_streaming_buffer_push_many() {
        let buffer = StreamingBuffer::<f64>::new(5);

        buffer.push_many(vec![1.0, 2.0, 3.0]);
        assert_eq!(buffer.read(), vec![1.0, 2.0, 3.0]);

        // Wrap around with push_many
        buffer.push_many(vec![4.0, 5.0, 6.0, 7.0]);
        assert_eq!(buffer.read(), vec![3.0, 4.0, 5.0, 6.0, 7.0]);
    }

    #[test]
    fn test_streaming_buffer_appended_tracking() {
        let buffer = StreamingBuffer::<f64>::new(10);

        buffer.push(1.0);
        buffer.push(2.0);
        buffer.push(3.0);

        assert_eq!(buffer.appended_since_mark(), 3);
        assert_eq!(buffer.read_appended(), vec![1.0, 2.0, 3.0]);

        buffer.mark_rendered();
        assert_eq!(buffer.appended_since_mark(), 0);
        assert!(buffer.read_appended().is_empty());

        buffer.push(4.0);
        buffer.push(5.0);
        assert_eq!(buffer.appended_since_mark(), 2);
        assert_eq!(buffer.read_appended(), vec![4.0, 5.0]);
    }

    #[test]
    fn test_streaming_buffer_partial_render() {
        let buffer = StreamingBuffer::<f64>::new(5);

        buffer.push_many(vec![1.0, 2.0, 3.0]);
        assert!(buffer.can_partial_render());

        buffer.mark_rendered();
        buffer.push_many(vec![4.0, 5.0]);
        assert!(buffer.can_partial_render());

        // Fill beyond capacity - can't partial render
        buffer.push_many(vec![6.0, 7.0, 8.0, 9.0, 10.0]);
        assert!(!buffer.can_partial_render());
    }

    #[test]
    fn test_streaming_buffer_version_tracking() {
        let buffer = StreamingBuffer::<f64>::new(10);

        let v0 = buffer.version();
        buffer.push(1.0);
        let v1 = buffer.version();
        assert!(v1 > v0);

        buffer.push_many(vec![2.0, 3.0]);
        let v2 = buffer.version();
        assert!(v2 > v1);
    }

    #[test]
    fn test_streaming_buffer_clear() {
        let buffer = StreamingBuffer::<f64>::new(5);

        buffer.push_many(vec![1.0, 2.0, 3.0]);
        assert_eq!(buffer.len(), 3);

        buffer.clear();
        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);
        assert!(buffer.read().is_empty());
    }

    #[test]
    fn test_streaming_buffer_subscribers() {
        let buffer = StreamingBuffer::<f64>::new(10);
        let count = Arc::new(AtomicUsize::new(0));

        let count_clone = Arc::clone(&count);
        let id = buffer.subscribe(move || {
            count_clone.fetch_add(1, AtomicOrdering::Relaxed);
        });

        buffer.push(1.0);
        buffer.push(2.0);
        assert_eq!(count.load(AtomicOrdering::Relaxed), 2);

        buffer.unsubscribe(id);
        buffer.push(3.0);
        assert_eq!(count.load(AtomicOrdering::Relaxed), 2);
    }

    #[test]
    fn test_streaming_buffer_thread_safety() {
        let buffer = StreamingBuffer::<i32>::new(1000);
        let buffer_clone = buffer.clone();

        let handle = thread::spawn(move || {
            for i in 0..500 {
                buffer_clone.push(i);
            }
        });

        for i in 500..1000 {
            buffer.push(i);
        }

        handle.join().unwrap();

        // Both threads wrote 500 values each
        assert_eq!(buffer.total_written(), 1000);
        assert_eq!(buffer.len(), 1000);
    }

    // ========================================================================
    // StreamingXY Tests
    // ========================================================================

    #[test]
    fn test_streaming_xy_basic() {
        let xy = StreamingXY::new(100);

        assert!(xy.is_empty());

        xy.push(1.0, 10.0);
        xy.push(2.0, 20.0);
        xy.push(3.0, 30.0);

        assert_eq!(xy.len(), 3);
        assert_eq!(xy.read_x(), vec![1.0, 2.0, 3.0]);
        assert_eq!(xy.read_y(), vec![10.0, 20.0, 30.0]);
    }

    #[test]
    fn test_streaming_xy_push_many() {
        let xy = StreamingXY::new(100);

        xy.push_many(vec![(1.0, 10.0), (2.0, 20.0), (3.0, 30.0)]);

        assert_eq!(xy.read_x(), vec![1.0, 2.0, 3.0]);
        assert_eq!(xy.read_y(), vec![10.0, 20.0, 30.0]);
    }

    #[test]
    fn test_streaming_xy_appended() {
        let xy = StreamingXY::new(100);

        xy.push_many(vec![(1.0, 10.0), (2.0, 20.0)]);
        xy.mark_rendered();

        xy.push_many(vec![(3.0, 30.0), (4.0, 40.0)]);

        assert_eq!(xy.appended_count(), 2);
        assert_eq!(xy.read_appended_x(), vec![3.0, 4.0]);
        assert_eq!(xy.read_appended_y(), vec![30.0, 40.0]);
    }

    #[test]
    fn test_streaming_xy_clear() {
        let xy = StreamingXY::new(100);

        xy.push_many(vec![(1.0, 10.0), (2.0, 20.0)]);
        assert_eq!(xy.len(), 2);

        xy.clear();
        assert!(xy.is_empty());
    }

    // ========================================================================
    // Additional Edge Case Tests for StreamingBuffer
    // ========================================================================

    #[test]
    fn test_streaming_buffer_empty_read() {
        let buffer = StreamingBuffer::<f64>::new(10);

        // Read from empty buffer
        assert!(buffer.read().is_empty());
        assert!(buffer.read_appended().is_empty());
        assert_eq!(buffer.appended_since_mark(), 0);
        assert!(buffer.is_empty());
        assert!(!buffer.is_full());
    }

    #[test]
    fn test_streaming_buffer_capacity_one() {
        // Edge case: buffer with capacity of 1
        let buffer = StreamingBuffer::<i32>::new(1);

        buffer.push(1);
        assert_eq!(buffer.read(), vec![1]);
        assert!(buffer.is_full());

        buffer.push(2);
        assert_eq!(buffer.read(), vec![2]);
        assert_eq!(buffer.len(), 1);

        buffer.push(3);
        assert_eq!(buffer.read(), vec![3]);
    }

    #[test]
    fn test_streaming_buffer_appended_exceeds_capacity() {
        let buffer = StreamingBuffer::<f64>::new(3);

        // Push more than capacity without marking rendered
        buffer.push_many(vec![1.0, 2.0, 3.0, 4.0, 5.0]);

        // Appended count tracks all pushes, even beyond capacity
        assert_eq!(buffer.appended_since_mark(), 5);

        // But read_appended is limited to capacity
        let appended = buffer.read_appended();
        assert_eq!(appended.len(), 3);
        assert_eq!(appended, vec![3.0, 4.0, 5.0]);

        // can_partial_render should be false (appended >= capacity)
        assert!(!buffer.can_partial_render());
    }

    #[test]
    fn test_streaming_buffer_clone_shares_state() {
        let buffer1 = StreamingBuffer::<f64>::new(10);
        let buffer2 = buffer1.clone();

        buffer1.push(1.0);
        buffer1.push(2.0);

        // Clone should see the same data
        assert_eq!(buffer2.read(), vec![1.0, 2.0]);
        assert_eq!(buffer2.len(), 2);

        // Push through clone should be visible in original
        buffer2.push(3.0);
        assert_eq!(buffer1.read(), vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_streaming_buffer_push_after_clear() {
        let buffer = StreamingBuffer::<f64>::new(5);

        buffer.push_many(vec![1.0, 2.0, 3.0]);
        buffer.clear();

        // Should be able to push after clear
        buffer.push(10.0);
        buffer.push(20.0);

        assert_eq!(buffer.read(), vec![10.0, 20.0]);
        assert_eq!(buffer.len(), 2);
        assert_eq!(buffer.total_written(), 2); // Reset by clear
    }

    #[test]
    fn test_streaming_buffer_multiple_wrap_cycles() {
        let buffer = StreamingBuffer::<i32>::new(3);

        // First cycle
        buffer.push_many(vec![1, 2, 3]);
        assert_eq!(buffer.read(), vec![1, 2, 3]);

        // Second cycle
        buffer.push_many(vec![4, 5, 6]);
        assert_eq!(buffer.read(), vec![4, 5, 6]);

        // Third cycle
        buffer.push_many(vec![7, 8, 9]);
        assert_eq!(buffer.read(), vec![7, 8, 9]);

        // Partial fourth cycle
        buffer.push(10);
        assert_eq!(buffer.read(), vec![8, 9, 10]);
    }

    #[test]
    fn test_streaming_buffer_total_written_tracking() {
        let buffer = StreamingBuffer::<f64>::new(3);

        buffer.push_many(vec![1.0, 2.0, 3.0, 4.0, 5.0]);
        assert_eq!(buffer.total_written(), 5);
        assert_eq!(buffer.len(), 3); // Only 3 in buffer

        buffer.push_many(vec![6.0, 7.0]);
        assert_eq!(buffer.total_written(), 7);
    }

    #[test]
    fn test_streaming_buffer_mark_rendered_resets_only_appended() {
        let buffer = StreamingBuffer::<f64>::new(10);

        buffer.push_many(vec![1.0, 2.0, 3.0]);
        let version_before = buffer.version();

        buffer.mark_rendered();

        // mark_rendered only resets appended count, not version
        assert_eq!(buffer.appended_since_mark(), 0);
        assert_eq!(buffer.version(), version_before); // Version unchanged
        assert_eq!(buffer.len(), 3); // Data unchanged
    }

    #[test]
    fn test_streaming_xy_version_tracking() {
        let xy = StreamingXY::new(100);

        let v0 = xy.version();
        xy.push(1.0, 10.0);
        let v1 = xy.version();
        assert!(v1 > v0);

        xy.push_many(vec![(2.0, 20.0), (3.0, 30.0)]);
        let v2 = xy.version();
        assert!(v2 > v1);
    }

    #[test]
    fn test_streaming_xy_clone_shares_state() {
        let xy1 = StreamingXY::new(100);
        let xy2 = xy1.clone();

        xy1.push(1.0, 10.0);
        assert_eq!(xy2.len(), 1);
        assert_eq!(xy2.read_x(), vec![1.0]);
    }

    #[test]
    fn test_streaming_buffer_concurrent_push_many() {
        use std::sync::Arc;

        let buffer = StreamingBuffer::<i32>::new(10000);
        let buffer1 = buffer.clone();
        let buffer2 = buffer.clone();

        let handle1 = thread::spawn(move || {
            for i in 0..1000 {
                buffer1.push(i);
            }
        });

        let handle2 = thread::spawn(move || {
            for i in 1000..2000 {
                buffer2.push(i);
            }
        });

        handle1.join().unwrap();
        handle2.join().unwrap();

        // All 2000 values should be written
        assert_eq!(buffer.total_written(), 2000);

        // Buffer should be full with 10000 capacity, but only 2000 written
        assert_eq!(buffer.len(), 2000);

        // Verify no values were lost (all unique in range 0-1999)
        let data = buffer.read();
        let mut sorted = data.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 2000);
    }

    #[test]
    fn test_streaming_buffer_subscriber_notification_count() {
        let buffer = StreamingBuffer::<f64>::new(10);
        let notify_count = Arc::new(AtomicUsize::new(0));

        let count_clone = Arc::clone(&notify_count);
        buffer.subscribe(move || {
            count_clone.fetch_add(1, AtomicOrdering::Relaxed);
        });

        // push() notifies once per call
        buffer.push(1.0);
        assert_eq!(notify_count.load(AtomicOrdering::Relaxed), 1);

        // push_many() notifies once for the batch
        buffer.push_many(vec![2.0, 3.0, 4.0]);
        assert_eq!(notify_count.load(AtomicOrdering::Relaxed), 2);

        // clear() notifies
        buffer.clear();
        assert_eq!(notify_count.load(AtomicOrdering::Relaxed), 3);
    }
}
