//! Reactive/Observable data example
//!
//! This example demonstrates ruviz's Makie-inspired reactive data system.
//! Observables allow data to be updated dynamically with automatic change detection.
//!
//! Run with: cargo run --example reactive_example

use ruviz::data::{Observable, SlidingWindowObservable, lift, lift2};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

fn main() {
    println!("=== Ruviz Observable/Reactive Data Demo ===\n");

    // Example 1: Basic Observable
    basic_observable();

    // Example 2: Derived Observables with lift
    derived_observables();

    // Example 3: Subscriber notifications
    subscriber_notifications();

    // Example 4: Sliding window for streaming data
    sliding_window();

    // Example 5: Thread-safe concurrent updates
    concurrent_updates();

    // Example 6: Reactive plot data simulation
    reactive_plot_simulation();

    println!("\n=== All reactive examples completed! ===");
}

/// Basic Observable creation and updates
fn basic_observable() {
    println!("--- Example 1: Basic Observable ---");

    // Create an observable with initial data
    let x = Observable::new(vec![1.0, 2.0, 3.0]);

    println!("Initial data: {:?}", *x.read());
    println!("Version: {}", x.version());

    // Update the data
    x.update(|data| {
        data.push(4.0);
        data.push(5.0);
    });

    println!("After update: {:?}", *x.read());
    println!("Version: {} (incremented)", x.version());

    // Replace entirely
    x.set(vec![10.0, 20.0, 30.0]);
    println!("After set: {:?}", *x.read());
    println!("Version: {}", x.version());
    println!();
}

/// Derived observables using lift (Makie's pattern)
fn derived_observables() {
    println!("--- Example 2: Derived Observables (lift) ---");

    // Create source observable
    let x = Observable::new(3.0);

    // Create derived observable that auto-updates
    let squared = lift(&x, |v| v * v);
    let cubed = lift(&x, |v| v * v * v);

    println!("x = {}", *x.read());
    println!("x² = {} (auto-computed)", *squared.read());
    println!("x³ = {} (auto-computed)", *cubed.read());

    // Update x - derived values update automatically!
    x.set(4.0);
    println!("\nAfter x.set(4.0):");
    println!("x = {}", *x.read());
    println!("x² = {} (auto-updated)", *squared.read());
    println!("x³ = {} (auto-updated)", *cubed.read());

    // lift2 for combining two observables
    let a = Observable::new(10.0);
    let b = Observable::new(5.0);
    let sum = lift2(&a, &b, |x, y| x + y);
    let product = lift2(&a, &b, |x, y| x * y);

    println!("\nCombining two observables:");
    println!("a = {}, b = {}", *a.read(), *b.read());
    println!("a + b = {} (auto-computed)", *sum.read());
    println!("a * b = {} (auto-computed)", *product.read());

    a.set(20.0);
    println!(
        "After a.set(20.0): a + b = {}, a * b = {}",
        *sum.read(),
        *product.read()
    );
    println!();
}

/// Subscriber notification system
fn subscriber_notifications() {
    println!("--- Example 3: Subscriber Notifications ---");

    let data = Observable::new(0);
    let call_count = Arc::new(AtomicUsize::new(0));

    // Subscribe to changes
    let count_clone = Arc::clone(&call_count);
    let subscriber_id = data.subscribe(move || {
        count_clone.fetch_add(1, Ordering::Relaxed);
    });

    println!("Subscriber count: {}", data.subscriber_count());
    println!("Callback calls: {}", call_count.load(Ordering::Relaxed));

    // Trigger updates
    data.set(1);
    data.set(2);
    data.update(|v| *v += 1);

    println!(
        "After 3 updates, callback calls: {}",
        call_count.load(Ordering::Relaxed)
    );

    // Unsubscribe
    data.unsubscribe(subscriber_id);
    data.set(100);
    println!(
        "After unsubscribe + 1 update, calls still: {}",
        call_count.load(Ordering::Relaxed)
    );
    println!();
}

/// Sliding window for streaming/time-series data
fn sliding_window() {
    println!("--- Example 4: Sliding Window Observable ---");

    // Create a sliding window that keeps only the last 5 values
    let window = SlidingWindowObservable::new(5);

    // Push values one at a time
    for i in 1..=8 {
        window.push(i as f64);
        println!("After push({}): {:?}", i, *window.read());
    }

    // Push many at once
    window.push_many(vec![100.0, 200.0, 300.0]);
    println!("After push_many([100, 200, 300]): {:?}", *window.read());

    println!(
        "Max size: {}, Current len: {}",
        window.max_size(),
        window.len()
    );
    println!();
}

/// Thread-safe concurrent updates
fn concurrent_updates() {
    println!("--- Example 5: Thread-Safe Concurrent Updates ---");

    let counter = Observable::new(0i32);
    let counter_clone = counter.clone();

    // Spawn a thread that increments
    let handle = thread::spawn(move || {
        for _ in 0..1000 {
            counter_clone.update(|v| *v += 1);
        }
    });

    // Main thread also increments
    for _ in 0..1000 {
        counter.update(|v| *v += 1);
    }

    handle.join().unwrap();

    println!("Final counter value: {} (expected: 2000)", *counter.read());
    println!();
}

/// Simulating reactive plot data updates
fn reactive_plot_simulation() {
    println!("--- Example 6: Reactive Plot Data Simulation ---");

    // Observable X and Y data for a plot
    let x_data = Observable::new(vec![0.0, 1.0, 2.0, 3.0, 4.0]);
    let y_data = Observable::new(vec![0.0, 1.0, 4.0, 9.0, 16.0]);

    // Track versions for change detection (like a render loop would)
    let mut last_x_version = x_data.version();
    let mut last_y_version = y_data.version();

    println!("Initial state:");
    println!("  X: {:?}", *x_data.read());
    println!("  Y: {:?}", *y_data.read());
    println!("  Versions: x={}, y={}", last_x_version, last_y_version);

    // Check if data changed since last render
    let needs_redraw = x_data.version() != last_x_version || y_data.version() != last_y_version;
    println!("\nChecking for changes: {}", needs_redraw);

    // Simulate data update (e.g., from a sensor or computation)
    x_data.update(|v| v.push(5.0));
    y_data.update(|v| v.push(25.0));

    println!("\nAfter adding point (5, 25):");
    let needs_redraw = x_data.version() != last_x_version || y_data.version() != last_y_version;
    println!("  Needs redraw: {}", needs_redraw);
    println!("  X: {:?}", *x_data.read());
    println!("  Y: {:?}", *y_data.read());

    // Mark as rendered
    last_x_version = x_data.version();
    last_y_version = y_data.version();

    println!("\nAfter render (versions updated):");
    let needs_redraw = x_data.version() != last_x_version || y_data.version() != last_y_version;
    println!("  Needs redraw: {}", needs_redraw);

    // Demonstrate derived statistics
    let stats = lift2(&x_data, &y_data, |x, y| {
        let x_mean: f64 = x.iter().sum::<f64>() / x.len() as f64;
        let y_mean: f64 = y.iter().sum::<f64>() / y.len() as f64;
        (x.len(), x_mean, y_mean)
    });

    println!("\nDerived statistics (auto-computed):");
    let (count, x_mean, y_mean) = *stats.read();
    println!(
        "  Points: {}, X mean: {:.2}, Y mean: {:.2}",
        count, x_mean, y_mean
    );

    // Update data - stats auto-update!
    x_data.update(|v| v.push(6.0));
    y_data.update(|v| v.push(36.0));

    let (count, x_mean, y_mean) = *stats.read();
    println!("After adding (6, 36):");
    println!(
        "  Points: {}, X mean: {:.2}, Y mean: {:.2}",
        count, x_mean, y_mean
    );
    println!();
}
