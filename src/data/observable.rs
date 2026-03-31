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

use crate::core::{PlottingError, Result};
use std::ops::Deref;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock, RwLockReadGuard, Weak};

/// Type alias for subscriber callback functions
pub type SubscriberCallback = Box<dyn Fn() + Send + Sync>;

type SharedSubscriberCallback = Arc<dyn Fn() + Send + Sync>;

/// Unique identifier for a subscriber
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubscriberId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct DropHookId(u64);

/// Internal subscriber entry
struct Subscriber {
    id: SubscriberId,
    callback: SharedSubscriberCallback,
}

struct DropHookEntry {
    id: DropHookId,
    hook: Box<dyn FnOnce() + Send + 'static>,
}

struct ObservableLifecycle {
    drop_hooks: Mutex<Vec<DropHookEntry>>,
    next_drop_hook_id: AtomicU64,
}

impl ObservableLifecycle {
    fn new() -> Self {
        Self {
            drop_hooks: Mutex::new(Vec::new()),
            next_drop_hook_id: AtomicU64::new(0),
        }
    }

    fn add_drop_hook<F>(&self, hook: F) -> DropHookId
    where
        F: FnOnce() + Send + 'static,
    {
        let id = DropHookId(self.next_drop_hook_id.fetch_add(1, Ordering::Relaxed));
        self.drop_hooks
            .lock()
            .expect("Observable lifecycle lock poisoned")
            .push(DropHookEntry {
                id,
                hook: Box::new(hook),
            });
        id
    }

    fn remove_drop_hook(&self, id: DropHookId) -> bool {
        let mut hooks = self
            .drop_hooks
            .lock()
            .expect("Observable lifecycle lock poisoned");
        if let Some(pos) = hooks.iter().position(|entry| entry.id == id) {
            hooks.remove(pos);
            true
        } else {
            false
        }
    }

    #[cfg(test)]
    fn hook_count(&self) -> usize {
        self.drop_hooks
            .lock()
            .expect("Observable lifecycle lock poisoned")
            .len()
    }
}

impl Drop for ObservableLifecycle {
    fn drop(&mut self) {
        let hooks = std::mem::take(
            &mut *self
                .drop_hooks
                .lock()
                .expect("Observable lifecycle lock poisoned"),
        );
        for entry in hooks {
            (entry.hook)();
        }
    }
}

fn collect_subscriber_callbacks(
    subscribers: &RwLock<Vec<Subscriber>>,
    lock_error: &str,
) -> Vec<SharedSubscriberCallback> {
    subscribers
        .read()
        .expect(lock_error)
        .iter()
        .map(|subscriber| Arc::clone(&subscriber.callback))
        .collect()
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
    /// Internal lifecycle hooks for derived subscriptions and cleanup.
    lifecycle: Arc<ObservableLifecycle>,
}

impl<T> Clone for Observable<T> {
    fn clone(&self) -> Self {
        Self {
            data: Arc::clone(&self.data),
            version: Arc::clone(&self.version),
            subscribers: Arc::clone(&self.subscribers),
            next_subscriber_id: Arc::clone(&self.next_subscriber_id),
            lifecycle: Arc::clone(&self.lifecycle),
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
    fn reserve_subscriber_id(&self) -> SubscriberId {
        SubscriberId(self.next_subscriber_id.fetch_add(1, Ordering::Relaxed))
    }

    fn add_subscriber_with_id(&self, id: SubscriberId, callback: SharedSubscriberCallback) {
        let subscriber = Subscriber { id, callback };
        self.subscribers
            .write()
            .expect("Subscribers lock poisoned")
            .push(subscriber);
    }

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
            lifecycle: Arc::new(ObservableLifecycle::new()),
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
        let id = self.reserve_subscriber_id();
        self.add_subscriber_with_id(id, Arc::new(callback));
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
        let callbacks =
            collect_subscriber_callbacks(&self.subscribers, "Subscribers lock poisoned");
        for callback in callbacks {
            callback();
        }
    }

    fn on_last_drop<F>(&self, hook: F) -> DropHookId
    where
        F: FnOnce() + Send + 'static,
    {
        self.lifecycle.add_drop_hook(hook)
    }

    fn remove_drop_hook(&self, id: DropHookId) -> bool {
        self.lifecycle.remove_drop_hook(id)
    }

    #[cfg(test)]
    fn lifecycle_hook_count(&self) -> usize {
        self.lifecycle.hook_count()
    }

    /// Create a weak reference to this observable
    ///
    /// This is useful for avoiding reference cycles when observables
    /// reference each other.
    pub fn downgrade(&self) -> WeakObservable<T> {
        WeakObservable {
            data: Arc::downgrade(&self.data),
            version: Arc::downgrade(&self.version),
            subscribers: Arc::downgrade(&self.subscribers),
            next_subscriber_id: Arc::downgrade(&self.next_subscriber_id),
            lifecycle: Arc::downgrade(&self.lifecycle),
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
    subscribers: Weak<RwLock<Vec<Subscriber>>>,
    next_subscriber_id: Weak<AtomicU64>,
    lifecycle: Weak<ObservableLifecycle>,
}

impl<T> Clone for WeakObservable<T> {
    fn clone(&self) -> Self {
        Self {
            data: Weak::clone(&self.data),
            version: Weak::clone(&self.version),
            subscribers: Weak::clone(&self.subscribers),
            next_subscriber_id: Weak::clone(&self.next_subscriber_id),
            lifecycle: Weak::clone(&self.lifecycle),
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
        let subscribers = self.subscribers.upgrade()?;
        let next_subscriber_id = self.next_subscriber_id.upgrade()?;
        let lifecycle = self.lifecycle.upgrade()?;

        Some(Observable {
            data,
            version,
            subscribers,
            next_subscriber_id,
            lifecycle,
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
    let f = Arc::new(f);
    let weak_source = source.downgrade();
    let weak_derived = derived.downgrade();
    let id = source.reserve_subscriber_id();
    source.add_subscriber_with_id(
        id,
        Arc::new(move || {
            let Some(source) = weak_source.upgrade() else {
                return;
            };
            let Some(derived) = weak_derived.upgrade() else {
                source.unsubscribe(id);
                return;
            };

            let new_value = f(source.get());
            {
                let mut guard = derived.data.write().expect("Lock poisoned");
                *guard = new_value;
            }
            derived.bump_version();
        }),
    );
    let weak_source_for_drop = source.downgrade();
    derived.on_last_drop(move || {
        if let Some(source) = weak_source_for_drop.upgrade() {
            source.unsubscribe(id);
        }
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
    let weak_derived = derived.downgrade();
    let weak_s1 = source1.downgrade();
    let weak_s2 = source2.downgrade();
    let source1_id = source1.reserve_subscriber_id();
    let source2_id = source2.reserve_subscriber_id();

    // Subscribe to source1
    {
        let f_clone = Arc::clone(&f);
        let weak_derived = weak_derived.clone();
        let weak_s1 = weak_s1.clone();
        let weak_s2 = weak_s2.clone();
        source1.add_subscriber_with_id(
            source1_id,
            Arc::new(move || {
                let Some(source1) = weak_s1.upgrade() else {
                    return;
                };
                let Some(source2) = weak_s2.upgrade() else {
                    source1.unsubscribe(source1_id);
                    return;
                };
                let Some(derived) = weak_derived.upgrade() else {
                    source1.unsubscribe(source1_id);
                    return;
                };

                let new_value = f_clone(source1.get(), source2.get());
                {
                    let mut guard = derived.data.write().expect("Lock poisoned");
                    *guard = new_value;
                }
                derived.bump_version();
            }),
        );
    }

    // Subscribe to source2
    {
        let f_clone = Arc::clone(&f);
        let weak_derived = weak_derived.clone();
        let weak_s1 = weak_s1.clone();
        let weak_s2 = weak_s2.clone();
        source2.add_subscriber_with_id(
            source2_id,
            Arc::new(move || {
                let Some(source1) = weak_s1.upgrade() else {
                    if let Some(source2) = weak_s2.upgrade() {
                        source2.unsubscribe(source2_id);
                    }
                    return;
                };
                let Some(source2) = weak_s2.upgrade() else {
                    return;
                };
                let Some(derived) = weak_derived.upgrade() else {
                    source2.unsubscribe(source2_id);
                    return;
                };

                let new_value = f_clone(source1.get(), source2.get());
                {
                    let mut guard = derived.data.write().expect("Lock poisoned");
                    *guard = new_value;
                }
                derived.bump_version();
            }),
        );
    }

    let weak_source1_for_source2_drop = source1.downgrade();
    let source2_drop_hook_id = source2.on_last_drop(move || {
        if let Some(source1) = weak_source1_for_source2_drop.upgrade() {
            source1.unsubscribe(source1_id);
        }
    });

    let weak_source2_for_source1_drop = source2.downgrade();
    let source1_drop_hook_id = source1.on_last_drop(move || {
        if let Some(source2) = weak_source2_for_source1_drop.upgrade() {
            source2.unsubscribe(source2_id);
        }
    });

    let weak_source1_for_derived_drop = source1.downgrade();
    let weak_source2_for_derived_drop = source2.downgrade();
    derived.on_last_drop(move || {
        if let Some(source1) = weak_source1_for_derived_drop.upgrade() {
            source1.unsubscribe(source1_id);
            source1.remove_drop_hook(source1_drop_hook_id);
        }
        if let Some(source2) = weak_source2_for_derived_drop.upgrade() {
            source2.unsubscribe(source2_id);
            source2.remove_drop_hook(source2_drop_hook_id);
        }
    });

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

/// Rendering state for the changes accumulated in a [`StreamingBuffer`] since the
/// last [`StreamingBuffer::mark_rendered`] call.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StreamingRenderState {
    /// No new visible samples have arrived since the last render mark.
    Unchanged,
    /// Only newly visible tail samples need to be appended to the existing frame.
    AppendOnly { visible_appended: usize },
    /// Existing visible samples were displaced, so the next frame must redraw.
    FullRedrawRequired,
}

impl StreamingRenderState {
    /// Returns the number of newly visible samples represented by this state.
    pub fn visible_appended(self) -> usize {
        match self {
            Self::Unchanged | Self::FullRedrawRequired => 0,
            Self::AppendOnly { visible_appended } => visible_appended,
        }
    }

    /// Returns `true` when an append-only incremental render is still correct.
    pub fn can_incrementally_render(self) -> bool {
        matches!(self, Self::AppendOnly { .. })
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
        Self::try_new(capacity).unwrap_or_else(|_| Self::with_capacity(1))
    }

    /// Try to create a new streaming buffer with validated capacity.
    pub fn try_new(capacity: usize) -> Result<Self> {
        if capacity == 0 {
            return Err(PlottingError::InvalidInput(
                "StreamingBuffer capacity must be at least 1".to_string(),
            ));
        }

        Ok(Self::with_capacity(capacity))
    }

    fn with_capacity(capacity: usize) -> Self {
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
        {
            let mut data = self.data.write().expect("Lock poisoned");
            let write_pos = self.write_pos.load(Ordering::Relaxed);
            let pos = write_pos % self.capacity;
            data[pos] = Some(value);
            self.write_pos
                .store(write_pos.wrapping_add(1), Ordering::Release);
            let total = self.total_written.load(Ordering::Relaxed);
            self.total_written
                .store(total.saturating_add(1), Ordering::Release);
            let appended = self.appended_since_render.load(Ordering::Relaxed);
            self.appended_since_render
                .store(appended.saturating_add(1), Ordering::Release);
        }
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
            let mut write_pos = self.write_pos.load(Ordering::Relaxed);
            for value in values {
                let pos = write_pos % self.capacity;
                data[pos] = Some(value);
                write_pos = write_pos.wrapping_add(1);
            }
            self.write_pos.store(write_pos, Ordering::Release);
            let total = self.total_written.load(Ordering::Relaxed);
            self.total_written
                .store(total.saturating_add(count as u64), Ordering::Release);
            let appended = self.appended_since_render.load(Ordering::Relaxed);
            self.appended_since_render
                .store(appended.saturating_add(count), Ordering::Release);
        }
        self.bump_version();
    }

    /// Get all valid data in order (oldest to newest)
    pub fn read(&self) -> Vec<T> {
        let data = self.data.read().expect("Lock poisoned");
        let total = self.total_written.load(Ordering::Acquire);
        let write_pos = self.write_pos.load(Ordering::Acquire);

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
        let appended = self.appended_since_render.load(Ordering::Acquire);
        let write_pos = self.write_pos.load(Ordering::Acquire);

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
        self.appended_since_render.load(Ordering::Acquire)
    }

    /// Mark the buffer as rendered (resets appended count)
    pub fn mark_rendered(&self) {
        self.appended_since_render.store(0, Ordering::Release);
    }

    /// Describe whether the current buffer changes can be rendered incrementally.
    pub fn render_state(&self) -> StreamingRenderState {
        let appended = self.appended_since_render.load(Ordering::Acquire);
        if appended == 0 {
            return StreamingRenderState::Unchanged;
        }

        let total_written = self.total_written.load(Ordering::Acquire);
        let visible_after = std::cmp::min(total_written as usize, self.capacity);
        let total_before = total_written.saturating_sub(appended as u64);
        let visible_before = std::cmp::min(total_before as usize, self.capacity);

        if visible_before == 0 {
            return StreamingRenderState::AppendOnly {
                visible_appended: visible_after,
            };
        }

        if visible_before.saturating_add(appended) <= self.capacity {
            return StreamingRenderState::AppendOnly {
                visible_appended: appended,
            };
        }

        StreamingRenderState::FullRedrawRequired
    }

    /// Check if partial re-render is possible.
    ///
    /// Prefer [`StreamingBuffer::render_state`] when the caller needs to
    /// distinguish append-only updates from wraparound/full-redraw cases.
    pub fn can_partial_render(&self) -> bool {
        self.render_state().can_incrementally_render()
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
        let total = self.total_written.load(Ordering::Acquire);
        std::cmp::min(total as usize, self.capacity)
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.total_written.load(Ordering::Acquire) == 0
    }

    /// Check if the buffer is full (has wrapped at least once)
    pub fn is_full(&self) -> bool {
        self.total_written.load(Ordering::Acquire) >= self.capacity as u64
    }

    /// Total number of elements ever written
    pub fn total_written(&self) -> u64 {
        self.total_written.load(Ordering::Acquire)
    }

    /// Clear all data
    pub fn clear(&self) {
        {
            let mut data = self.data.write().expect("Lock poisoned");
            for slot in data.iter_mut() {
                *slot = None;
            }
            self.write_pos.store(0, Ordering::Release);
            self.total_written.store(0, Ordering::Release);
            self.appended_since_render.store(0, Ordering::Release);
        }
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
            callback: Arc::new(callback),
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
        let callbacks = collect_subscriber_callbacks(&self.subscribers, "Lock poisoned");
        for callback in callbacks {
            callback();
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

/// Paired streaming buffers for X/Y time-series data
///
/// Provides synchronized updates and version tracking for plot integration
pub struct StreamingXY {
    x: StreamingBuffer<f64>,
    y: StreamingBuffer<f64>,
    subscribers: Arc<RwLock<Vec<Subscriber>>>,
    next_subscriber_id: Arc<AtomicU64>,
}

impl StreamingXY {
    /// Create a new paired streaming buffer
    pub fn new(capacity: usize) -> Self {
        Self {
            x: StreamingBuffer::new(capacity),
            y: StreamingBuffer::new(capacity),
            subscribers: Arc::new(RwLock::new(Vec::new())),
            next_subscriber_id: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Push a single X/Y point
    pub fn push(&self, x: f64, y: f64) {
        self.x.push(x);
        self.y.push(y);
        self.notify_subscribers();
    }

    /// Push multiple X/Y points
    pub fn push_many(&self, points: impl IntoIterator<Item = (f64, f64)>) {
        let mut pushed_any = false;
        for (x, y) in points {
            self.x.push(x);
            self.y.push(y);
            pushed_any = true;
        }

        if pushed_any {
            self.notify_subscribers();
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
    ///
    /// Prefer [`StreamingXY::render_state`] when the caller needs to distinguish
    /// append-only updates from wraparound/full-redraw cases.
    pub fn can_partial_render(&self) -> bool {
        self.render_state().can_incrementally_render()
    }

    /// Describe whether the paired buffers can be rendered incrementally.
    pub fn render_state(&self) -> StreamingRenderState {
        match (self.x.render_state(), self.y.render_state()) {
            (StreamingRenderState::Unchanged, StreamingRenderState::Unchanged) => {
                StreamingRenderState::Unchanged
            }
            (
                StreamingRenderState::AppendOnly {
                    visible_appended: x,
                },
                StreamingRenderState::AppendOnly {
                    visible_appended: y,
                },
            ) => StreamingRenderState::AppendOnly {
                visible_appended: x.min(y),
            },
            _ => StreamingRenderState::FullRedrawRequired,
        }
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
        self.notify_subscribers();
    }

    pub(crate) fn subscribe_paired<F>(&self, callback: F) -> SubscriberId
    where
        F: Fn() + Send + Sync + 'static,
    {
        let id = SubscriberId(self.next_subscriber_id.fetch_add(1, Ordering::Relaxed));
        let subscriber = Subscriber {
            id,
            callback: Arc::new(callback),
        };
        self.subscribers
            .write()
            .expect("Lock poisoned")
            .push(subscriber);
        id
    }

    pub(crate) fn unsubscribe_paired(&self, id: SubscriberId) -> bool {
        let mut subscribers = self.subscribers.write().expect("Lock poisoned");
        if let Some(pos) = subscribers
            .iter()
            .position(|subscriber| subscriber.id == id)
        {
            subscribers.remove(pos);
            true
        } else {
            false
        }
    }

    fn notify_subscribers(&self) {
        let callbacks = collect_subscriber_callbacks(&self.subscribers, "Lock poisoned");
        for callback in callbacks {
            callback();
        }
    }
}

impl Clone for StreamingXY {
    fn clone(&self) -> Self {
        Self {
            x: self.x.clone(),
            y: self.y.clone(),
            subscribers: Arc::clone(&self.subscribers),
            next_subscriber_id: Arc::clone(&self.next_subscriber_id),
        }
    }
}

impl std::fmt::Debug for StreamingXY {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StreamingXY")
            .field("len", &self.len())
            .field("version", &self.version())
            .finish()
    }
}

#[cfg(test)]
mod tests;
