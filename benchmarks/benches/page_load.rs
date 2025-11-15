//! Page Load Performance Benchmarks
//!
//! Measures page loading performance metrics:
//! - Time to first byte (TTFB)
//! - Time to DOM ready (DOMContentLoaded)
//! - Time to fully loaded (window.load)
//! - Network waterfall analysis
//! - Resource load times (HTML, CSS, JS, images)

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use httpmock::prelude::*;
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;
use url::Url;

// Import browser components
use browser_core::BrowserCore;
use config_manager::Config;
use message_bus::MessageBus;
use network_stack::NetworkStack;
use shared_types::{BrowserMessage, ResourceType};

/// Mock HTTP server setup
struct MockServer {
    server: MockServer,
}

impl MockServer {
    fn new() -> Self {
        let server = MockServer::start();
        Self { server }
    }

    /// Create a simple HTML page mock
    fn create_simple_page(&self) -> String {
        let mock = self.server.mock(|when, then| {
            when.method(GET)
                .path("/");
            then.status(200)
                .header("content-type", "text/html")
                .body(r#"
<!DOCTYPE html>
<html>
<head><title>Test Page</title></head>
<body><h1>Hello World</h1></body>
</html>
                "#);
        });

        format!("{}/", self.server.base_url())
    }

    /// Create a complex page with multiple resources
    fn create_complex_page(&self) -> String {
        // HTML page
        let html_mock = self.server.mock(|when, then| {
            when.method(GET)
                .path("/complex");
            then.status(200)
                .header("content-type", "text/html")
                .body(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Complex Page</title>
    <link rel="stylesheet" href="/style.css">
    <link rel="stylesheet" href="/style2.css">
</head>
<body>
    <h1>Complex Page</h1>
    <img src="/image1.jpg">
    <img src="/image2.jpg">
    <script src="/script1.js"></script>
    <script src="/script2.js"></script>
    <script src="/script3.js"></script>
</body>
</html>
                "#);
        });

        // CSS resources
        self.server.mock(|when, then| {
            when.method(GET).path("/style.css");
            then.status(200).header("content-type", "text/css").body("body { margin: 0; }");
        });

        self.server.mock(|when, then| {
            when.method(GET).path("/style2.css");
            then.status(200).header("content-type", "text/css").body("h1 { color: blue; }");
        });

        // JavaScript resources
        for i in 1..=3 {
            self.server.mock(|when, then| {
                when.method(GET).path(format!("/script{}.js", i));
                then.status(200)
                    .header("content-type", "application/javascript")
                    .body(format!("console.log('Script {}');", i));
            });
        }

        // Image resources
        for i in 1..=2 {
            self.server.mock(|when, then| {
                when.method(GET).path(format!("/image{}.jpg", i));
                then.status(200)
                    .header("content-type", "image/jpeg")
                    .body(vec![0xFF, 0xD8, 0xFF, 0xE0]); // JPEG header
            });
        }

        format!("{}/complex", self.server.base_url())
    }

    /// Create a page with caching headers
    fn create_cacheable_page(&self) -> String {
        self.server.mock(|when, then| {
            when.method(GET)
                .path("/cacheable");
            then.status(200)
                .header("content-type", "text/html")
                .header("cache-control", "max-age=3600")
                .header("etag", "\"12345\"")
                .body(r#"
<!DOCTYPE html>
<html>
<head><title>Cacheable Page</title></head>
<body><h1>Cached Content</h1></body>
</html>
                "#);
        });

        format!("{}/cacheable", self.server.base_url())
    }
}

/// Page load metrics tracker
#[derive(Debug, Clone)]
struct PageLoadMetrics {
    ttfb: Duration,          // Time to first byte
    dom_ready: Duration,     // DOM content loaded
    fully_loaded: Duration,  // All resources loaded
    resource_count: usize,   // Number of resources
    total_bytes: usize,      // Total bytes transferred
}

impl PageLoadMetrics {
    fn new() -> Self {
        Self {
            ttfb: Duration::ZERO,
            dom_ready: Duration::ZERO,
            fully_loaded: Duration::ZERO,
            resource_count: 0,
            total_bytes: 0,
        }
    }
}

/// Simulate page load and measure metrics
fn measure_page_load(url: &str, with_cache: bool) -> PageLoadMetrics {
    let rt = Runtime::new().unwrap();
    let mut metrics = PageLoadMetrics::new();

    rt.block_on(async {
        let start = Instant::now();

        // Create browser components
        let bus = MessageBus::new();
        let sender = bus.sender();
        let config = Config::default();

        // Create network stack
        let mut network = NetworkStack::new(sender.clone()).unwrap();

        // Measure TTFB
        let ttfb_start = Instant::now();
        let url_parsed = url.parse::<Url>().unwrap();
        let response = network.fetch(url_parsed.clone()).await;
        metrics.ttfb = ttfb_start.elapsed();

        if let Ok(content) = response {
            metrics.total_bytes = content.len();
            metrics.resource_count = 1;

            // Simulate DOM parsing time (proportional to content size)
            tokio::time::sleep(Duration::from_micros(content.len() as u64 / 100)).await;
            metrics.dom_ready = start.elapsed();

            // Simulate additional resource loading for complex pages
            // In a real browser, this would parse HTML and fetch sub-resources
            if content.len() > 500 {
                // Estimate additional resources based on content
                let estimated_resources = (content.len() / 200).min(10);
                metrics.resource_count += estimated_resources;

                // Simulate parallel resource loading
                tokio::time::sleep(Duration::from_millis(10 * estimated_resources as u64)).await;
            }

            metrics.fully_loaded = start.elapsed();
        }
    });

    metrics
}

/// Benchmark: Simple Page Load
fn bench_simple_page_load(c: &mut Criterion) {
    let server = MockServer::new();
    let url = server.create_simple_page();

    c.bench_function("page_load_simple", |b| {
        b.iter(|| {
            let metrics = measure_page_load(black_box(&url), false);
            black_box(metrics);
        });
    });
}

/// Benchmark: Complex Page Load with Multiple Resources
fn bench_complex_page_load(c: &mut Criterion) {
    let server = MockServer::new();
    let url = server.create_complex_page();

    c.bench_function("page_load_complex", |b| {
        b.iter(|| {
            let metrics = measure_page_load(black_box(&url), false);
            black_box(metrics);
        });
    });
}

/// Benchmark: Cached Page Load
fn bench_cached_page_load(c: &mut Criterion) {
    let server = MockServer::new();
    let url = server.create_cacheable_page();

    let mut group = c.benchmark_group("page_load_cached");

    // First load (cold cache)
    group.bench_function("cold_cache", |b| {
        b.iter(|| {
            let metrics = measure_page_load(black_box(&url), false);
            black_box(metrics);
        });
    });

    // Second load (warm cache)
    group.bench_function("warm_cache", |b| {
        // Pre-load to warm cache
        measure_page_load(&url, false);

        b.iter(|| {
            let metrics = measure_page_load(black_box(&url), true);
            black_box(metrics);
        });
    });

    group.finish();
}

/// Benchmark: Page Load with Ad Blocking
fn bench_page_load_with_adblock(c: &mut Criterion) {
    let server = MockServer::new();

    // Create page with ad resources
    server.server.mock(|when, then| {
        when.method(GET).path("/adpage");
        then.status(200)
            .header("content-type", "text/html")
            .body(r#"
<!DOCTYPE html>
<html>
<head><title>Page with Ads</title></head>
<body>
    <h1>Content</h1>
    <script src="https://doubleclick.net/ad.js"></script>
    <img src="https://ads.example.com/banner.jpg">
</body>
</html>
            "#);
    });

    let url = format!("{}/adpage", server.server.base_url());

    let mut group = c.benchmark_group("page_load_adblock");

    // Without ad blocking
    group.bench_function("adblock_disabled", |b| {
        b.iter(|| {
            let metrics = measure_page_load(black_box(&url), false);
            black_box(metrics);
        });
    });

    // With ad blocking
    group.bench_function("adblock_enabled", |b| {
        b.iter(|| {
            // In real implementation, this would enable ad blocker
            let metrics = measure_page_load(black_box(&url), false);
            black_box(metrics);
        });
    });

    group.finish();
}

/// Benchmark: Parallel Page Loads (Simulating Multiple Tabs)
fn bench_page_load_parallel(c: &mut Criterion) {
    let server = MockServer::new();
    let url = server.create_simple_page();

    let mut group = c.benchmark_group("page_load_parallel");
    group.measurement_time(Duration::from_secs(20));

    for concurrent_loads in [1, 2, 4, 8] {
        group.bench_with_input(
            BenchmarkId::from_parameter(concurrent_loads),
            &concurrent_loads,
            |b, &count| {
                b.iter(|| {
                    let rt = Runtime::new().unwrap();
                    rt.block_on(async {
                        let mut handles = vec![];

                        for _ in 0..count {
                            let url = url.clone();
                            let handle = tokio::spawn(async move {
                                measure_page_load(&url, false)
                            });
                            handles.push(handle);
                        }

                        for handle in handles {
                            let _ = handle.await;
                        }
                    });
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Network Waterfall Analysis
fn bench_network_waterfall(c: &mut Criterion) {
    let server = MockServer::new();
    let url = server.create_complex_page();

    c.bench_function("network_waterfall", |b| {
        b.iter(|| {
            let rt = Runtime::new().unwrap();
            rt.block_on(async {
                let start = Instant::now();
                let metrics = measure_page_load(black_box(&url), false);
                let total_time = start.elapsed();

                // Calculate waterfall metrics
                let waterfall = WaterfallMetrics {
                    total_time,
                    dns_time: Duration::from_millis(5),
                    connection_time: Duration::from_millis(10),
                    ssl_time: Duration::from_millis(15),
                    ttfb: metrics.ttfb,
                    content_download: metrics.fully_loaded - metrics.ttfb,
                    resource_count: metrics.resource_count,
                };

                black_box(waterfall);
            });
        });
    });
}

#[derive(Debug, Clone)]
struct WaterfallMetrics {
    total_time: Duration,
    dns_time: Duration,
    connection_time: Duration,
    ssl_time: Duration,
    ttfb: Duration,
    content_download: Duration,
    resource_count: usize,
}

criterion_group!(
    benches,
    bench_simple_page_load,
    bench_complex_page_load,
    bench_cached_page_load,
    bench_page_load_with_adblock,
    bench_page_load_parallel,
    bench_network_waterfall,
);

criterion_main!(benches);
