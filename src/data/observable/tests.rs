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
fn test_weak_observable_upgrade_preserves_subscribers() {
    let obs = Observable::new(1);
    let notifications = Arc::new(AtomicUsize::new(0));
    let notifications_clone = Arc::clone(&notifications);

    let first_id = obs.subscribe(move || {
        notifications_clone.fetch_add(1, AtomicOrdering::Relaxed);
    });

    let weak = obs.downgrade();
    let upgraded = weak.upgrade().expect("upgrade should preserve state");
    assert_eq!(upgraded.subscriber_count(), 1);

    let second_id = upgraded.subscribe(|| {});
    assert_ne!(first_id, second_id);

    upgraded.set(2);
    assert_eq!(notifications.load(AtomicOrdering::Relaxed), 1);
}

#[test]
fn test_observable_unsubscribe_within_callback_does_not_deadlock() {
    let obs = Observable::new(0);
    let callback_count = Arc::new(AtomicUsize::new(0));
    let callback_count_clone = Arc::clone(&callback_count);
    let callback_id = Arc::new(Mutex::new(None));
    let callback_id_clone = Arc::clone(&callback_id);
    let obs_clone = obs.clone();

    let id = obs.subscribe(move || {
        callback_count_clone.fetch_add(1, AtomicOrdering::Relaxed);
        if let Some(id) = *callback_id_clone.lock().expect("Lock poisoned") {
            obs_clone.unsubscribe(id);
        }
    });
    *callback_id.lock().expect("Lock poisoned") = Some(id);

    obs.set(1);
    obs.set(2);

    assert_eq!(callback_count.load(AtomicOrdering::Relaxed), 1);
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
fn test_lift_releases_source_subscription_on_drop() {
    let source = Observable::new(2.0);
    let derived = lift(&source, |v| v * 2.0);

    assert_eq!(source.subscriber_count(), 1);
    drop(derived);

    assert_eq!(source.subscriber_count(), 0);
}

#[test]
fn test_lift2_releases_remaining_source_subscription_when_other_source_drops() {
    let source2 = Observable::new(5.0);
    let derived = {
        let source1 = Observable::new(10.0);
        let derived = lift2(&source1, &source2, |x, y| x + y);
        assert_eq!(source2.subscriber_count(), 1);
        derived
    };

    source2.set(6.0);
    assert_eq!(source2.subscriber_count(), 0);
    assert_eq!(*derived.read(), 15.0);
}

#[test]
fn test_lift2_releases_source1_subscription_when_source2_drops_first() {
    let source1 = Observable::new(10.0);
    let derived = {
        let source2 = Observable::new(5.0);
        let derived = lift2(&source1, &source2, |x, y| x + y);
        assert_eq!(source1.subscriber_count(), 1);
        derived
    };

    assert_eq!(source1.subscriber_count(), 0);
    assert_eq!(*derived.read(), 15.0);
}

#[test]
fn test_lift2_does_not_accumulate_source_drop_hooks_after_drop() {
    let source1 = Observable::new(1.0);
    let source2 = Observable::new(2.0);

    for _ in 0..8 {
        let derived = lift2(&source1, &source2, |x, y| x + y);
        assert!(source1.lifecycle_hook_count() >= 1);
        assert!(source2.lifecycle_hook_count() >= 1);
        drop(derived);
        assert_eq!(source1.lifecycle_hook_count(), 0);
        assert_eq!(source2.lifecycle_hook_count(), 0);
    }
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
    assert_eq!(
        buffer.render_state(),
        StreamingRenderState::AppendOnly {
            visible_appended: 3
        }
    );

    buffer.mark_rendered();
    buffer.push_many(vec![4.0, 5.0]);
    assert!(buffer.can_partial_render());
    assert_eq!(
        buffer.render_state(),
        StreamingRenderState::AppendOnly {
            visible_appended: 2
        }
    );

    // Fill beyond capacity - can't partial render
    buffer.push_many(vec![6.0, 7.0, 8.0, 9.0, 10.0]);
    assert!(!buffer.can_partial_render());
    assert_eq!(
        buffer.render_state(),
        StreamingRenderState::FullRedrawRequired
    );
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
fn test_streaming_xy_paired_subscribers_fire_once_per_batch() {
    let xy = StreamingXY::new(100);
    let hits = Arc::new(AtomicUsize::new(0));
    let hits_for_callback = Arc::clone(&hits);
    let id = xy.subscribe_paired(move || {
        hits_for_callback.fetch_add(1, AtomicOrdering::Relaxed);
    });

    xy.push(1.0, 10.0);
    xy.push_many(vec![(2.0, 20.0), (3.0, 30.0)]);

    assert_eq!(hits.load(AtomicOrdering::Relaxed), 2);

    xy.unsubscribe_paired(id);
    xy.push(4.0, 40.0);
    assert_eq!(hits.load(AtomicOrdering::Relaxed), 2);
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
    assert_eq!(
        xy.render_state(),
        StreamingRenderState::AppendOnly {
            visible_appended: 2
        }
    );
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
fn test_streaming_buffer_try_new_rejects_zero_capacity() {
    assert!(StreamingBuffer::<i32>::try_new(0).is_err());
}

#[test]
fn test_streaming_buffer_new_zero_capacity_is_normalized() {
    let buffer = StreamingBuffer::<i32>::new(0);

    assert_eq!(buffer.capacity(), 1);
    buffer.push(7);
    buffer.push(8);
    assert_eq!(buffer.read(), vec![8]);
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

    // With no previously rendered points, the visible tail can still be appended directly.
    assert!(buffer.can_partial_render());
    assert_eq!(
        buffer.render_state(),
        StreamingRenderState::AppendOnly {
            visible_appended: 3
        }
    );
}

#[test]
fn test_streaming_buffer_render_state_requires_full_redraw_after_wrap() {
    let buffer = StreamingBuffer::<f64>::new(5);
    buffer.push_many(vec![1.0, 2.0, 3.0, 4.0]);
    buffer.mark_rendered();

    buffer.push_many(vec![5.0, 6.0]);

    assert_eq!(buffer.read(), vec![2.0, 3.0, 4.0, 5.0, 6.0]);
    assert_eq!(
        buffer.render_state(),
        StreamingRenderState::FullRedrawRequired
    );
    assert!(!buffer.can_partial_render());
}

#[test]
fn test_streaming_buffer_render_state_stays_append_only_from_empty_cache() {
    let buffer = StreamingBuffer::<f64>::new(3);
    buffer.mark_rendered();

    buffer.push_many(vec![1.0, 2.0, 3.0, 4.0, 5.0]);

    assert_eq!(buffer.read(), vec![3.0, 4.0, 5.0]);
    assert_eq!(
        buffer.render_state(),
        StreamingRenderState::AppendOnly {
            visible_appended: 3
        }
    );
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

#[test]
fn test_streaming_buffer_unsubscribe_within_callback_does_not_deadlock() {
    let buffer = StreamingBuffer::<i32>::new(4);
    let notify_count = Arc::new(AtomicUsize::new(0));
    let notify_count_clone = Arc::clone(&notify_count);
    let callback_id = Arc::new(Mutex::new(None));
    let callback_id_clone = Arc::clone(&callback_id);
    let buffer_clone = buffer.clone();

    let id = buffer.subscribe(move || {
        notify_count_clone.fetch_add(1, AtomicOrdering::Relaxed);
        if let Some(id) = *callback_id_clone.lock().expect("Lock poisoned") {
            buffer_clone.unsubscribe(id);
        }
    });
    *callback_id.lock().expect("Lock poisoned") = Some(id);

    buffer.push(1);
    buffer.push(2);

    assert_eq!(notify_count.load(AtomicOrdering::Relaxed), 1);
}
