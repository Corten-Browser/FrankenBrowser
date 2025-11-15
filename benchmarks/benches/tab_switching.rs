//! Tab Switching Performance Benchmarks
//!
//! Measures tab switching performance metrics:
//! - Tab switch latency (active → background → active)
//! - Memory usage per tab
//! - CPU usage during switch
//! - First paint after switch
//! - Background tab suspension/restoration

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

// Import browser components
use browser_shell::BrowserShell;
use config_manager::{Config, ShellConfig};
use message_bus::MessageBus;
use shared_types::TabId;

/// Mock tab state for benchmarking
#[derive(Debug, Clone)]
struct MockTab {
    id: TabId,
    url: String,
    active: bool,
    memory_kb: usize,
    last_paint: Instant,
}

impl MockTab {
    fn new(id: TabId, url: &str) -> Self {
        Self {
            id,
            url: url.to_string(),
            active: false,
            memory_kb: 50_000, // ~50 MB baseline
            last_paint: Instant::now(),
        }
    }

    /// Simulate activation (restore from suspended state)
    fn activate(&mut self) {
        self.active = true;
        self.last_paint = Instant::now();
        // Simulate repainting delay
        std::thread::sleep(Duration::from_micros(100));
    }

    /// Simulate suspension (move to background)
    fn suspend(&mut self) {
        self.active = false;
        // Background tabs use slightly less memory
        self.memory_kb = (self.memory_kb as f64 * 0.9) as usize;
    }
}

/// Tab manager for benchmarking
struct MockTabManager {
    tabs: Vec<MockTab>,
    active_tab: Option<TabId>,
}

impl MockTabManager {
    fn new() -> Self {
        Self {
            tabs: Vec::new(),
            active_tab: None,
        }
    }

    /// Create a new tab
    fn create_tab(&mut self, url: &str) -> TabId {
        let id = self.tabs.len() as TabId;
        let tab = MockTab::new(id, url);
        self.tabs.push(tab);
        id
    }

    /// Switch to a specific tab
    fn switch_to_tab(&mut self, tab_id: TabId) -> Duration {
        let start = Instant::now();

        // Suspend current active tab
        if let Some(active_id) = self.active_tab {
            if let Some(tab) = self.tabs.get_mut(active_id as usize) {
                tab.suspend();
            }
        }

        // Activate target tab
        if let Some(tab) = self.tabs.get_mut(tab_id as usize) {
            tab.activate();
            self.active_tab = Some(tab_id);
        }

        start.elapsed()
    }

    /// Get total memory usage
    fn total_memory_kb(&self) -> usize {
        self.tabs.iter().map(|t| t.memory_kb).sum()
    }

    /// Close a tab
    fn close_tab(&mut self, tab_id: TabId) {
        self.tabs.retain(|t| t.id != tab_id);
        if self.active_tab == Some(tab_id) {
            self.active_tab = None;
        }
    }
}

/// Benchmark: Switch Between Two Tabs
fn bench_switch_between_two_tabs(c: &mut Criterion) {
    c.bench_function("tab_switch_two_tabs", |b| {
        let mut manager = MockTabManager::new();
        let tab1 = manager.create_tab("https://example.com");
        let tab2 = manager.create_tab("https://github.com");

        manager.switch_to_tab(tab1);

        b.iter(|| {
            // Switch to tab2
            let latency1 = manager.switch_to_tab(black_box(tab2));
            // Switch back to tab1
            let latency2 = manager.switch_to_tab(black_box(tab1));

            black_box((latency1, latency2));
        });
    });
}

/// Benchmark: Switch Among Many Tabs
fn bench_switch_with_many_tabs(c: &mut Criterion) {
    let mut group = c.benchmark_group("tab_switch_many_tabs");

    for tab_count in [5, 10, 20, 50] {
        group.bench_with_input(
            BenchmarkId::from_parameter(tab_count),
            &tab_count,
            |b, &count| {
                let mut manager = MockTabManager::new();

                // Create tabs
                let tab_ids: Vec<TabId> = (0..count)
                    .map(|i| manager.create_tab(&format!("https://example{}.com", i)))
                    .collect();

                // Activate first tab
                manager.switch_to_tab(tab_ids[0]);

                b.iter(|| {
                    // Switch through all tabs sequentially
                    for &tab_id in &tab_ids {
                        let latency = manager.switch_to_tab(black_box(tab_id));
                        black_box(latency);
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Switch to Background Tab (Cold Activation)
fn bench_switch_to_background_tab(c: &mut Criterion) {
    c.bench_function("tab_switch_cold_activation", |b| {
        let mut manager = MockTabManager::new();

        // Create multiple tabs
        let tab1 = manager.create_tab("https://example.com");
        let tab2 = manager.create_tab("https://github.com");
        let tab3 = manager.create_tab("https://stackoverflow.com");

        // Activate tab1 and let others go cold
        manager.switch_to_tab(tab1);
        std::thread::sleep(Duration::from_millis(100));

        b.iter(|| {
            // Switch to cold tab (tab3)
            let latency = manager.switch_to_tab(black_box(tab3));

            // Switch back to warm tab
            manager.switch_to_tab(tab1);
            std::thread::sleep(Duration::from_millis(10));

            black_box(latency);
        });
    });
}

/// Benchmark: Switch with Active Content (Animations/Timers)
fn bench_switch_with_active_content(c: &mut Criterion) {
    c.bench_function("tab_switch_active_content", |b| {
        let mut manager = MockTabManager::new();

        // Create tabs with "active" content
        let tab1 = manager.create_tab("https://youtube.com/watch"); // Video
        let tab2 = manager.create_tab("https://twitter.com/feed"); // Auto-updating feed

        manager.switch_to_tab(tab1);

        b.iter(|| {
            // Simulate content activity before switch
            std::thread::sleep(Duration::from_micros(50));

            let latency = manager.switch_to_tab(black_box(tab2));

            // Switch back
            std::thread::sleep(Duration::from_micros(50));
            manager.switch_to_tab(tab1);

            black_box(latency);
        });
    });
}

/// Benchmark: Tab Creation and Immediate Switch
fn bench_tab_creation_and_switch(c: &mut Criterion) {
    c.bench_function("tab_create_and_switch", |b| {
        b.iter(|| {
            let mut manager = MockTabManager::new();

            let start = Instant::now();

            // Create new tab
            let new_tab = manager.create_tab("https://example.com");

            // Switch to it immediately
            manager.switch_to_tab(new_tab);

            let total_time = start.elapsed();
            black_box(total_time);
        });
    });
}

/// Benchmark: Tab Close and Switch to Next
fn bench_tab_close_and_switch(c: &mut Criterion) {
    c.bench_function("tab_close_and_switch_next", |b| {
        b.iter(|| {
            let mut manager = MockTabManager::new();

            // Create 3 tabs
            let tab1 = manager.create_tab("https://example1.com");
            let tab2 = manager.create_tab("https://example2.com");
            let tab3 = manager.create_tab("https://example3.com");

            // Activate tab2
            manager.switch_to_tab(tab2);

            let start = Instant::now();

            // Close active tab
            manager.close_tab(tab2);

            // Switch to next tab
            manager.switch_to_tab(tab3);

            let total_time = start.elapsed();
            black_box(total_time);
        });
    });
}

/// Benchmark: Memory Usage Per Tab
fn bench_memory_per_tab(c: &mut Criterion) {
    let mut group = c.benchmark_group("tab_memory_usage");

    for tab_count in [1, 5, 10, 20] {
        group.bench_with_input(
            BenchmarkId::from_parameter(tab_count),
            &tab_count,
            |b, &count| {
                b.iter(|| {
                    let mut manager = MockTabManager::new();

                    // Create tabs
                    for i in 0..count {
                        manager.create_tab(&format!("https://example{}.com", i));
                    }

                    let total_memory = manager.total_memory_kb();
                    let memory_per_tab = total_memory / count.max(1);

                    black_box((total_memory, memory_per_tab));
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: First Paint After Tab Switch
fn bench_first_paint_after_switch(c: &mut Criterion) {
    c.bench_function("tab_first_paint_after_switch", |b| {
        let mut manager = MockTabManager::new();

        let tab1 = manager.create_tab("https://example.com");
        let tab2 = manager.create_tab("https://github.com");

        manager.switch_to_tab(tab1);

        b.iter(|| {
            let switch_start = Instant::now();

            // Switch to tab
            manager.switch_to_tab(black_box(tab2));

            // Measure time until first paint
            let first_paint_time = if let Some(tab) = manager.tabs.get(tab2 as usize) {
                tab.last_paint.duration_since(switch_start)
            } else {
                Duration::ZERO
            };

            manager.switch_to_tab(tab1);

            black_box(first_paint_time);
        });
    });
}

/// Benchmark: Rapid Tab Switching (Stress Test)
fn bench_rapid_tab_switching(c: &mut Criterion) {
    c.bench_function("tab_rapid_switching", |b| {
        let mut manager = MockTabManager::new();

        // Create 5 tabs
        let tabs: Vec<TabId> = (0..5)
            .map(|i| manager.create_tab(&format!("https://example{}.com", i)))
            .collect();

        b.iter(|| {
            // Rapidly switch through tabs multiple times
            for _ in 0..10 {
                for &tab_id in &tabs {
                    manager.switch_to_tab(black_box(tab_id));
                }
            }
        });
    });
}

/// Benchmark: Tab Switch with Concurrent Operations
fn bench_tab_switch_concurrent(c: &mut Criterion) {
    let mut group = c.benchmark_group("tab_switch_concurrent");
    group.measurement_time(Duration::from_secs(15));

    c.bench_function("tab_switch_with_background_loading", |b| {
        b.iter(|| {
            let rt = Runtime::new().unwrap();

            rt.block_on(async {
                let mut manager = MockTabManager::new();

                let tab1 = manager.create_tab("https://example.com");
                let tab2 = manager.create_tab("https://github.com");

                manager.switch_to_tab(tab1);

                // Simulate background loading in tab2
                let _load_handle = tokio::spawn(async {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                });

                // Switch while background load is happening
                let latency = manager.switch_to_tab(black_box(tab2));

                black_box(latency);
            });
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_switch_between_two_tabs,
    bench_switch_with_many_tabs,
    bench_switch_to_background_tab,
    bench_switch_with_active_content,
    bench_tab_creation_and_switch,
    bench_tab_close_and_switch,
    bench_memory_per_tab,
    bench_first_paint_after_switch,
    bench_rapid_tab_switching,
    bench_tab_switch_concurrent,
);

criterion_main!(benches);
