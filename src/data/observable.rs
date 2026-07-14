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
use std::collections::VecDeque;
use std::ops::Deref;
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
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

#[derive(Default)]
struct NotificationStatus {
    batch_depth: usize,
    dirty: bool,
    dispatching: bool,
}

#[derive(Default)]
struct NotificationState {
    status: Mutex<NotificationStatus>,
}

#[derive(Default)]
struct PairNotificationStatus {
    pending: usize,
    dispatching: bool,
}

#[derive(Default)]
struct PairNotificationState {
    status: Mutex<PairNotificationStatus>,
}

impl PairNotificationState {
    fn queue(&self) -> bool {
        let mut status = self.status.lock().expect("Pair notification lock poisoned");
        status.pending = status.pending.saturating_add(1);
        if status.dispatching {
            false
        } else {
            status.dispatching = true;
            true
        }
    }

    fn take_pending(&self) -> bool {
        let mut status = self.status.lock().expect("Pair notification lock poisoned");
        if status.pending > 0 {
            status.pending -= 1;
            true
        } else {
            status.dispatching = false;
            false
        }
    }
}

struct DispatchReset<'a> {
    notifications: &'a NotificationState,
    armed: bool,
}

impl Drop for DispatchReset<'_> {
    fn drop(&mut self) {
        if self.armed {
            self.notifications
                .status
                .lock()
                .expect("Notification lock poisoned")
                .dispatching = false;
        }
    }
}

impl NotificationState {
    fn begin_batch(&self) {
        let mut status = self.status.lock().expect("Notification lock poisoned");
        status.batch_depth = status.batch_depth.saturating_add(1);
    }

    fn end_batch(&self) {
        let mut status = self.status.lock().expect("Notification lock poisoned");
        assert!(status.batch_depth > 0, "Unbalanced observable batch");
        status.batch_depth -= 1;
    }

    fn flush(&self, subscribers: &RwLock<Vec<Subscriber>>, lock_error: &str) {
        let should_dispatch = {
            let mut status = self.status.lock().expect("Notification lock poisoned");
            if status.batch_depth == 0 && status.dirty && !status.dispatching {
                status.dispatching = true;
                true
            } else {
                false
            }
        };
        if should_dispatch {
            self.drain(subscribers, lock_error);
        }
    }

    fn request(&self, subscribers: &RwLock<Vec<Subscriber>>, lock_error: &str) {
        let should_dispatch = {
            let mut status = self.status.lock().expect("Notification lock poisoned");
            status.dirty = true;
            if status.batch_depth == 0 && !status.dispatching {
                status.dispatching = true;
                true
            } else {
                false
            }
        };
        if should_dispatch {
            self.drain(subscribers, lock_error);
        }
    }

    fn drain(&self, subscribers: &RwLock<Vec<Subscriber>>, lock_error: &str) {
        let mut reset = DispatchReset {
            notifications: self,
            armed: true,
        };
        let mut first_panic = None;
        loop {
            let should_notify = {
                let mut status = self.status.lock().expect("Notification lock poisoned");
                if status.batch_depth > 0 || !status.dirty {
                    status.dispatching = false;
                    reset.armed = false;
                    false
                } else {
                    status.dirty = false;
                    true
                }
            };
            if !should_notify {
                break;
            }

            let callbacks = collect_subscriber_callbacks(subscribers, lock_error);
            for callback in callbacks {
                if let Err(payload) = catch_unwind(AssertUnwindSafe(|| callback())) {
                    if first_panic.is_none() {
                        first_panic = Some(payload);
                    }
                }
            }
        }

        if let Some(payload) = first_panic {
            resume_unwind(payload);
        }
    }
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
    /// Shared batching and non-recursive dispatch state.
    notifications: Arc<NotificationState>,
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
            notifications: Arc::clone(&self.notifications),
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
            notifications: Arc::new(NotificationState::default()),
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

    pub(crate) fn shares_source(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.data, &other.data)
    }

    pub(crate) fn source_id(&self) -> usize {
        Arc::as_ptr(&self.data) as usize
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

    #[cfg(feature = "animation")]
    pub(crate) fn update_if<F>(&self, f: F) -> bool
    where
        F: FnOnce(&mut T) -> bool,
    {
        let changed = {
            let mut guard = self.data.write().expect("Observable lock poisoned");
            f(&mut *guard)
        };
        if changed {
            self.bump_version();
        }
        changed
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
        self.notifications
            .request(&self.subscribers, "Subscribers lock poisoned");
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
            notifications: Arc::downgrade(&self.notifications),
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
    notifications: Weak<NotificationState>,
    lifecycle: Weak<ObservableLifecycle>,
}

impl<T> Clone for WeakObservable<T> {
    fn clone(&self) -> Self {
        Self {
            data: Weak::clone(&self.data),
            version: Weak::clone(&self.version),
            subscribers: Weak::clone(&self.subscribers),
            next_subscriber_id: Weak::clone(&self.next_subscriber_id),
            notifications: Weak::clone(&self.notifications),
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
        let notifications = self.notifications.upgrade()?;
        let lifecycle = self.lifecycle.upgrade()?;

        Some(Observable {
            data,
            version,
            subscribers,
            next_subscriber_id,
            notifications,
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
    notification_keys: Vec<*const NotificationState>,
}

/// Trait for types that can participate in batch updates
pub trait BatchNotifier {
    fn notify(&self);

    #[doc(hidden)]
    fn begin_batch(&self) {}

    #[doc(hidden)]
    fn end_batch(&self) {}

    #[doc(hidden)]
    fn flush_batch(&self) {
        self.notify();
    }
}

impl<T> BatchNotifier for Observable<T> {
    fn notify(&self) {
        self.notify_subscribers();
    }

    fn begin_batch(&self) {
        self.notifications.begin_batch();
    }

    fn end_batch(&self) {
        self.notifications.end_batch();
    }

    fn flush_batch(&self) {
        self.notifications
            .flush(&self.subscribers, "Subscribers lock poisoned");
    }
}

impl<'a> BatchUpdate<'a> {
    /// Create a new batch update
    pub fn new() -> Self {
        Self {
            observables: Vec::new(),
            notification_keys: Vec::new(),
        }
    }

    /// Add an observable to the batch
    pub fn add<T>(&mut self, observable: &'a Observable<T>) {
        let key = Arc::as_ptr(&observable.notifications);
        if self.notification_keys.contains(&key) {
            return;
        }
        observable.begin_batch();
        self.notification_keys.push(key);
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
            obs.end_batch();
        }
        let mut first_panic = None;
        for obs in &self.observables {
            if let Err(payload) = catch_unwind(AssertUnwindSafe(|| obs.flush_batch())) {
                if first_panic.is_none() {
                    first_panic = Some(payload);
                }
            }
        }
        if !std::thread::panicking() {
            if let Some(payload) = first_panic {
                resume_unwind(payload);
            }
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
    /// Number of explicit clears, used by paired legacy-lane reconciliation.
    clear_generation: Arc<AtomicU64>,
    /// Version counter for change detection
    version: Arc<AtomicU64>,
    /// Append count since last mark_rendered()
    appended_since_render: Arc<std::sync::atomic::AtomicUsize>,
    /// Highest total-written watermark acknowledged in the current clear generation.
    rendered_through: Arc<AtomicU64>,
    /// Exact watermark captured by a frame-scoped clone.
    acknowledge_through: Option<(u64, u64)>,
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
            clear_generation: Arc::new(AtomicU64::new(0)),
            version: Arc::new(AtomicU64::new(0)),
            appended_since_render: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            rendered_through: Arc::new(AtomicU64::new(0)),
            acknowledge_through: None,
            subscribers: Arc::new(RwLock::new(Vec::new())),
            next_subscriber_id: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Push a single value (O(1) operation)
    pub fn push(&self, value: T) {
        self.push_locked(value);
        self.bump_version();
    }

    fn push_locked(&self, value: T) {
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

    /// Push multiple values efficiently
    pub fn push_many(&self, values: impl IntoIterator<Item = T>) {
        let values: Vec<T> = values.into_iter().collect();
        let count = values.len();

        if count == 0 {
            return;
        }

        self.push_many_locked(values);
        self.bump_version();
    }

    fn push_many_locked(&self, values: Vec<T>) {
        let count = values.len();
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

    /// Get all valid data in order (oldest to newest)
    pub fn read(&self) -> Vec<T> {
        self.read_locked()
    }

    fn read_locked(&self) -> Vec<T> {
        let data = self.data.read().expect("Lock poisoned");
        let total = self.total_written.load(Ordering::Acquire);
        let write_pos = self.write_pos.load(Ordering::Acquire);
        self.ordered_values(&data, total, write_pos)
    }

    fn ordered_values(&self, data: &[Option<T>], total: u64, write_pos: usize) -> Vec<T> {
        if total == 0 {
            return Vec::new();
        }

        let len = std::cmp::min(total as usize, self.capacity);
        let mut result = Vec::with_capacity(len);

        if total <= self.capacity as u64 {
            for value in data.iter().take(len).flatten() {
                result.push(value.clone());
            }
        } else {
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

    pub(crate) fn snapshot_for_render(&self) -> (Vec<T>, Self) {
        let data = self.data.read().expect("Lock poisoned");
        let generation = self.clear_generation.load(Ordering::Acquire);
        let total = self.total_written.load(Ordering::Acquire);
        let write_pos = self.write_pos.load(Ordering::Acquire);
        let values = self.ordered_values(&data, total, write_pos);
        let mut captured = self.clone();
        captured.acknowledge_through = Some((generation, total));
        (values, captured)
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
        if let Some((generation, total)) = self.acknowledge_through {
            self.mark_rendered_through(generation, total);
            return;
        }

        let _data = self.data.read().expect("Lock poisoned");
        let total = self.total_written.load(Ordering::Acquire);
        self.rendered_through.store(total, Ordering::Release);
        self.appended_since_render.store(0, Ordering::Release);
    }

    fn mark_rendered_through(&self, generation: u64, total: u64) {
        let _data = self.data.read().expect("Lock poisoned");
        if self.clear_generation.load(Ordering::Acquire) != generation {
            return;
        }

        let current_total = self.total_written.load(Ordering::Acquire);
        let acknowledged = self
            .rendered_through
            .fetch_max(total, Ordering::AcqRel)
            .max(total);
        let pending = current_total.saturating_sub(acknowledged.min(current_total));
        self.appended_since_render.store(
            usize::try_from(pending).unwrap_or(usize::MAX),
            Ordering::Release,
        );
    }

    pub(crate) fn shares_source(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.data, &other.data)
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
        self.clear_locked();
        self.bump_version();
    }

    fn clear_locked(&self) {
        let mut data = self.data.write().expect("Lock poisoned");
        for slot in data.iter_mut() {
            *slot = None;
        }
        self.write_pos.store(0, Ordering::Release);
        self.total_written.store(0, Ordering::Release);
        self.appended_since_render.store(0, Ordering::Release);
        self.rendered_through.store(0, Ordering::Release);
        self.clear_generation.fetch_add(1, Ordering::Release);
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
        self.increment_version();
        self.notify_subscribers();
    }

    fn increment_version(&self) {
        self.version.fetch_add(1, Ordering::Release);
    }

    fn notify_subscribers(&self) {
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
            clear_generation: Arc::clone(&self.clear_generation),
            version: Arc::clone(&self.version),
            appended_since_render: Arc::clone(&self.appended_since_render),
            rendered_through: Arc::clone(&self.rendered_through),
            acknowledge_through: self.acknowledge_through,
            subscribers: Arc::clone(&self.subscribers),
            next_subscriber_id: Arc::clone(&self.next_subscriber_id),
        }
    }
}

/// An owned, aligned capture of a [`StreamingXY`] pair.
#[derive(Clone, Debug)]
pub struct StreamingXYSnapshot {
    x: Vec<f64>,
    y: Vec<f64>,
    sequence: u64,
    rendered_through: u64,
    render_state: StreamingRenderState,
    appended_start: usize,
}

impl StreamingXYSnapshot {
    /// Captured X values, aligned with [`StreamingXYSnapshot::y`].
    pub fn x(&self) -> &[f64] {
        &self.x
    }

    /// Captured Y values, aligned with [`StreamingXYSnapshot::x`].
    pub fn y(&self) -> &[f64] {
        &self.y
    }

    /// Pair sequence captured with the values.
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Highest pair sequence acknowledged when this snapshot was captured.
    pub fn rendered_through(&self) -> u64 {
        self.rendered_through
    }

    /// Rendering work pending at capture time.
    pub fn render_state(&self) -> StreamingRenderState {
        self.render_state
    }

    /// Aligned visible X tail that may be appended incrementally.
    pub fn appended_x(&self) -> &[f64] {
        &self.x[self.appended_start..]
    }

    /// Aligned visible Y tail that may be appended incrementally.
    pub fn appended_y(&self) -> &[f64] {
        &self.y[self.appended_start..]
    }

    pub(crate) fn into_parts(self) -> (Vec<f64>, Vec<f64>, u64, StreamingRenderState) {
        (self.x, self.y, self.sequence, self.render_state)
    }
}

#[derive(Debug)]
struct StreamingXYProgress {
    sequence: u64,
    rendered_through: u64,
    total_written: u64,
    last_clear_sequence: Option<u64>,
    synced_x_total: u64,
    synced_y_total: u64,
    synced_x_version: u64,
    synced_y_version: u64,
    synced_x_clear_generation: u64,
    synced_y_clear_generation: u64,
}

/// Paired streaming buffers for X/Y time-series data.
///
/// Mutations through this type commit both lanes before any lane or pair
/// subscriber is notified.
pub struct StreamingXY {
    x: StreamingBuffer<f64>,
    y: StreamingBuffer<f64>,
    mutation_gate: Arc<Mutex<()>>,
    paired_data: Arc<Mutex<VecDeque<(f64, f64)>>>,
    progress: Arc<Mutex<StreamingXYProgress>>,
    subscribers: Arc<RwLock<Vec<Subscriber>>>,
    next_subscriber_id: Arc<AtomicU64>,
    notifications: Arc<NotificationState>,
    pair_notifications: Arc<PairNotificationState>,
    acknowledge_through: Option<u64>,
    #[cfg(test)]
    pair_commit_hook: Arc<Mutex<Option<SharedSubscriberCallback>>>,
}

impl StreamingXY {
    /// Create a new paired streaming buffer
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            x: StreamingBuffer::with_capacity(capacity),
            y: StreamingBuffer::with_capacity(capacity),
            mutation_gate: Arc::new(Mutex::new(())),
            paired_data: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))),
            progress: Arc::new(Mutex::new(StreamingXYProgress {
                sequence: 0,
                rendered_through: 0,
                total_written: 0,
                last_clear_sequence: None,
                synced_x_total: 0,
                synced_y_total: 0,
                synced_x_version: 0,
                synced_y_version: 0,
                synced_x_clear_generation: 0,
                synced_y_clear_generation: 0,
            })),
            subscribers: Arc::new(RwLock::new(Vec::new())),
            next_subscriber_id: Arc::new(AtomicU64::new(0)),
            notifications: Arc::new(NotificationState::default()),
            pair_notifications: Arc::new(PairNotificationState::default()),
            acknowledge_through: None,
            #[cfg(test)]
            pair_commit_hook: Arc::new(Mutex::new(None)),
        }
    }

    /// Push a single X/Y point
    pub fn push(&self, x: f64, y: f64) {
        let should_dispatch = {
            let _gate = self.mutation_gate.lock().expect("Mutation gate poisoned");
            self.reconcile_legacy_lanes_locked();
            self.x.push_locked(x);
            self.run_pair_commit_hook();
            self.y.push_locked(y);
            self.x.increment_version();
            self.y.increment_version();
            let mut paired_data = self.paired_data.lock().expect("Paired data lock poisoned");
            if paired_data.len() == self.x.capacity() {
                paired_data.pop_front();
            }
            paired_data.push_back((x, y));
            self.record_push_locked(1);
            self.pair_notifications.queue()
        };
        if should_dispatch {
            self.drain_pair_notifications();
        }
    }

    /// Push multiple X/Y points
    pub fn push_many(&self, points: impl IntoIterator<Item = (f64, f64)>) {
        let points: Vec<_> = points.into_iter().collect();
        if points.is_empty() {
            return;
        }

        let count = points.len();
        let x = points.iter().map(|(x, _)| *x).collect();
        let y = points.iter().map(|(_, y)| *y).collect();
        let should_dispatch = {
            let _gate = self.mutation_gate.lock().expect("Mutation gate poisoned");
            self.reconcile_legacy_lanes_locked();
            self.x.push_many_locked(x);
            self.run_pair_commit_hook();
            self.y.push_many_locked(y);
            self.x.increment_version();
            self.y.increment_version();
            let mut paired_data = self.paired_data.lock().expect("Paired data lock poisoned");
            for point in points {
                if paired_data.len() == self.x.capacity() {
                    paired_data.pop_front();
                }
                paired_data.push_back(point);
            }
            self.record_push_locked(count);
            self.pair_notifications.queue()
        };
        if should_dispatch {
            self.drain_pair_notifications();
        }
    }

    fn record_push_locked(&self, count: usize) {
        let mut progress = self.progress.lock().expect("StreamingXY progress poisoned");
        progress.sequence = progress.sequence.wrapping_add(count as u64);
        progress.total_written = progress.total_written.saturating_add(count as u64);
        progress.synced_x_total = self.x.total_written();
        progress.synced_y_total = self.y.total_written();
        progress.synced_x_version = self.x.version();
        progress.synced_y_version = self.y.version();
        progress.synced_x_clear_generation = self.x.clear_generation.load(Ordering::Acquire);
        progress.synced_y_clear_generation = self.y.clear_generation.load(Ordering::Acquire);
        self.sync_lane_watermarks(&progress);
    }

    fn sync_lane_watermarks(&self, progress: &StreamingXYProgress) {
        let pending_samples = Self::pending_sample_count_locked(progress);
        let pending = usize::try_from(pending_samples).unwrap_or(usize::MAX);
        self.x
            .appended_since_render
            .store(pending, Ordering::Release);
        self.y
            .appended_since_render
            .store(pending, Ordering::Release);
        let rendered_total = progress.total_written.saturating_sub(pending_samples);
        self.x
            .rendered_through
            .store(rendered_total, Ordering::Release);
        self.y
            .rendered_through
            .store(rendered_total, Ordering::Release);
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
        self.x.read_locked()
    }

    /// Read all Y data
    pub fn read_y(&self) -> Vec<f64> {
        self.y.read_locked()
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
        let gate = self.mutation_gate.lock().expect("Mutation gate poisoned");
        let x = self.x.read_view();
        let y = self.y.read_view();
        drop(gate);
        (x, y)
    }

    /// Read only appended X data since last render
    pub fn read_appended_x(&self) -> Vec<f64> {
        self.snapshot().appended_x().to_vec()
    }

    /// Read only appended Y data since last render
    pub fn read_appended_y(&self) -> Vec<f64> {
        self.snapshot().appended_y().to_vec()
    }

    /// Get the number of points appended since last render
    pub fn appended_count(&self) -> usize {
        self.refresh_legacy_lanes();
        let progress = self.progress.lock().expect("StreamingXY progress poisoned");
        let appended = Self::pending_sample_count_locked(&progress);
        usize::try_from(appended).unwrap_or(usize::MAX)
    }

    /// Mark both buffers as rendered
    pub fn mark_rendered(&self) {
        let sequence = self
            .acknowledge_through
            .unwrap_or_else(|| self.snapshot().sequence());
        self.mark_rendered_through(sequence);
    }

    /// Monotonically acknowledge rendering through an exact captured sequence.
    pub fn mark_rendered_through(&self, sequence: u64) {
        let _gate = self.mutation_gate.lock().expect("Mutation gate poisoned");
        self.reconcile_legacy_lanes_locked();
        let mut progress = self.progress.lock().expect("StreamingXY progress poisoned");
        if Self::sequence_after(sequence, progress.rendered_through)
            && !Self::sequence_after(sequence, progress.sequence)
        {
            progress.rendered_through = sequence;
        }
        self.sync_lane_watermarks(&progress);
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
        self.snapshot().render_state()
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
        self.len() == 0
    }

    /// Clear both buffers
    pub fn clear(&self) {
        let should_dispatch = {
            let _gate = self.mutation_gate.lock().expect("Mutation gate poisoned");
            self.x.clear_locked();
            self.y.clear_locked();
            let mut paired_data = self.paired_data.lock().expect("Paired data lock poisoned");
            paired_data.clear();
            let mut progress = self.progress.lock().expect("StreamingXY progress poisoned");
            self.x.increment_version();
            self.y.increment_version();
            progress.sequence = progress.sequence.wrapping_add(1);
            progress.total_written = 0;
            progress.last_clear_sequence = Some(progress.sequence);
            progress.synced_x_total = 0;
            progress.synced_y_total = 0;
            progress.synced_x_version = self.x.version();
            progress.synced_y_version = self.y.version();
            progress.synced_x_clear_generation = self.x.clear_generation.load(Ordering::Acquire);
            progress.synced_y_clear_generation = self.y.clear_generation.load(Ordering::Acquire);
            self.sync_lane_watermarks(&progress);
            self.pair_notifications.queue()
        };
        if should_dispatch {
            self.drain_pair_notifications();
        }
    }

    /// Capture aligned owned values and the exact rendering watermark state.
    pub fn snapshot(&self) -> StreamingXYSnapshot {
        let _pair_gate = self.mutation_gate.lock().expect("Mutation gate poisoned");
        let mut paired_data = self.paired_data.lock().expect("Paired data lock poisoned");
        let mut progress = self.progress.lock().expect("StreamingXY progress poisoned");
        self.reconcile_legacy_lanes(&mut paired_data, &mut progress);

        let mut x = Vec::with_capacity(paired_data.len());
        let mut y = Vec::with_capacity(paired_data.len());
        for &(x_value, y_value) in paired_data.iter() {
            x.push(x_value);
            y.push(y_value);
        }
        let render_state = Self::render_state_locked(&progress, x.len(), self.x.capacity());
        let visible_appended = usize::try_from(Self::pending_sample_count_locked(&progress))
            .unwrap_or(usize::MAX)
            .min(x.len());
        let appended_start = x.len().saturating_sub(visible_appended);
        StreamingXYSnapshot {
            x,
            y,
            sequence: progress.sequence,
            rendered_through: progress.rendered_through,
            render_state,
            appended_start,
        }
    }

    fn reconcile_legacy_lanes(
        &self,
        paired_data: &mut VecDeque<(f64, f64)>,
        progress: &mut StreamingXYProgress,
    ) {
        let x_version = self.x.version();
        let y_version = self.y.version();
        let x_total = self.x.total_written();
        let y_total = self.y.total_written();
        let x_clear_generation = self.x.clear_generation.load(Ordering::Acquire);
        let y_clear_generation = self.y.clear_generation.load(Ordering::Acquire);

        let x_changed = x_version != progress.synced_x_version
            || x_total != progress.synced_x_total
            || x_clear_generation != progress.synced_x_clear_generation;
        let y_changed = y_version != progress.synced_y_version
            || y_total != progress.synced_y_total
            || y_clear_generation != progress.synced_y_clear_generation;
        if !x_changed || !y_changed {
            return;
        }

        let x_was_cleared = x_clear_generation != progress.synced_x_clear_generation;
        let y_was_cleared = y_clear_generation != progress.synced_y_clear_generation;
        if x_was_cleared != y_was_cleared {
            return;
        }

        let x_appended = x_total.saturating_sub(progress.synced_x_total);
        let y_appended = y_total.saturating_sub(progress.synced_y_total);
        if !x_was_cleared && (x_appended == 0 || x_appended != y_appended) {
            return;
        }
        if x_was_cleared && x_total != y_total {
            return;
        }

        let x_values = self.x.read_locked();
        let y_values = self.y.read_locked();
        if self.x.version() != x_version
            || self.y.version() != y_version
            || self.x.total_written() != x_total
            || self.y.total_written() != y_total
            || self.x.clear_generation.load(Ordering::Acquire) != x_clear_generation
            || self.y.clear_generation.load(Ordering::Acquire) != y_clear_generation
        {
            return;
        }

        if x_was_cleared {
            paired_data.clear();
            for (&x_value, &y_value) in x_values.iter().zip(&y_values) {
                paired_data.push_back((x_value, y_value));
            }
            progress.sequence = progress.sequence.wrapping_add(1);
            progress.total_written = x_total;
            progress.last_clear_sequence = Some(progress.sequence);
            progress.synced_x_total = x_total;
            progress.synced_y_total = y_total;
            progress.synced_x_version = x_version;
            progress.synced_y_version = y_version;
            progress.synced_x_clear_generation = x_clear_generation;
            progress.synced_y_clear_generation = y_clear_generation;
            self.sync_lane_watermarks(progress);
            return;
        }

        let visible = usize::try_from(x_appended)
            .unwrap_or(usize::MAX)
            .min(x_values.len())
            .min(y_values.len());
        let x_start = x_values.len().saturating_sub(visible);
        let y_start = y_values.len().saturating_sub(visible);
        for (&x_value, &y_value) in x_values[x_start..].iter().zip(&y_values[y_start..]) {
            if paired_data.len() == self.x.capacity() {
                paired_data.pop_front();
            }
            paired_data.push_back((x_value, y_value));
        }

        progress.sequence = progress.sequence.wrapping_add(x_appended);
        progress.total_written = progress.total_written.saturating_add(x_appended);
        progress.synced_x_total = x_total;
        progress.synced_y_total = y_total;
        progress.synced_x_version = x_version;
        progress.synced_y_version = y_version;
        progress.synced_x_clear_generation = x_clear_generation;
        progress.synced_y_clear_generation = y_clear_generation;
        self.sync_lane_watermarks(progress);
    }

    fn reconcile_legacy_lanes_locked(&self) {
        let mut paired_data = self.paired_data.lock().expect("Paired data lock poisoned");
        let mut progress = self.progress.lock().expect("StreamingXY progress poisoned");
        self.reconcile_legacy_lanes(&mut paired_data, &mut progress);
    }

    fn sequence_after(sequence: u64, reference: u64) -> bool {
        let distance = sequence.wrapping_sub(reference);
        distance != 0 && distance < (1_u64 << 63)
    }

    fn sequence_distance(sequence: u64, reference: u64) -> u64 {
        sequence.wrapping_sub(reference)
    }

    fn render_state_locked(
        progress: &StreamingXYProgress,
        visible_after: usize,
        capacity: usize,
    ) -> StreamingRenderState {
        if progress.sequence == progress.rendered_through {
            return StreamingRenderState::Unchanged;
        }
        if progress
            .last_clear_sequence
            .is_some_and(|sequence| Self::sequence_after(sequence, progress.rendered_through))
        {
            return StreamingRenderState::FullRedrawRequired;
        }

        let appended = Self::sequence_distance(progress.sequence, progress.rendered_through);
        let appended = usize::try_from(appended).unwrap_or(usize::MAX);
        let total_before = progress.total_written.saturating_sub(appended as u64);
        let visible_before = usize::try_from(total_before)
            .unwrap_or(usize::MAX)
            .min(capacity);
        if visible_before == 0 {
            return StreamingRenderState::AppendOnly {
                visible_appended: visible_after,
            };
        }
        if visible_before.saturating_add(appended) <= capacity {
            return StreamingRenderState::AppendOnly {
                visible_appended: appended.min(visible_after),
            };
        }
        StreamingRenderState::FullRedrawRequired
    }

    fn pending_sample_count_locked(progress: &StreamingXYProgress) -> u64 {
        if progress
            .last_clear_sequence
            .is_some_and(|sequence| Self::sequence_after(sequence, progress.rendered_through))
        {
            progress.total_written
        } else {
            Self::sequence_distance(progress.sequence, progress.rendered_through)
        }
    }

    pub(crate) fn captured_through(&self, sequence: u64) -> Self {
        let mut captured = self.clone();
        captured.acknowledge_through = Some(sequence);
        captured
    }

    pub(crate) fn shares_source(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.progress, &other.progress)
    }

    pub(crate) fn refresh_legacy_lanes(&self) {
        let _pair_gate = self.mutation_gate.lock().expect("Mutation gate poisoned");
        self.reconcile_legacy_lanes_locked();
    }

    #[cfg(test)]
    fn set_pair_commit_hook<F>(&self, hook: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        *self
            .pair_commit_hook
            .lock()
            .expect("Pair commit hook lock poisoned") = Some(Arc::new(hook));
    }

    #[cfg(test)]
    fn run_pair_commit_hook(&self) {
        let hook = self
            .pair_commit_hook
            .lock()
            .expect("Pair commit hook lock poisoned")
            .clone();
        if let Some(hook) = hook {
            hook();
        }
    }

    #[cfg(not(test))]
    fn run_pair_commit_hook(&self) {}

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
        self.notifications
            .request(&self.subscribers, "Lock poisoned");
    }

    fn drain_pair_notifications(&self) {
        let mut first_panic = None;
        while self.pair_notifications.take_pending() {
            let mut run = |notify: &dyn Fn()| {
                if let Err(payload) = catch_unwind(AssertUnwindSafe(notify)) {
                    if first_panic.is_none() {
                        first_panic = Some(payload);
                    }
                }
            };
            run(&|| self.x.notify_subscribers());
            run(&|| self.y.notify_subscribers());
            run(&|| self.notify_subscribers());
        }
        if let Some(payload) = first_panic {
            resume_unwind(payload);
        }
    }
}

impl Clone for StreamingXY {
    fn clone(&self) -> Self {
        Self {
            x: self.x.clone(),
            y: self.y.clone(),
            mutation_gate: Arc::clone(&self.mutation_gate),
            paired_data: Arc::clone(&self.paired_data),
            progress: Arc::clone(&self.progress),
            subscribers: Arc::clone(&self.subscribers),
            next_subscriber_id: Arc::clone(&self.next_subscriber_id),
            notifications: Arc::clone(&self.notifications),
            pair_notifications: Arc::clone(&self.pair_notifications),
            acknowledge_through: self.acknowledge_through,
            #[cfg(test)]
            pair_commit_hook: Arc::clone(&self.pair_commit_hook),
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
