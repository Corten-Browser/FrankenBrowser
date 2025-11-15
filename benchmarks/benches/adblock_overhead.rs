//! Ad Blocking Performance Benchmarks
//!
//! Measures ad blocking overhead and effectiveness:
//! - Filter rule evaluation time
//! - Blocking decision time per request
//! - Resource count (blocked vs allowed)
//! - Memory overhead of filter engine
//! - Impact on page load time
//! - Filter list loading time

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::time::{Duration, Instant};

// Import browser components
use adblock_engine::AdBlockEngine;
use message_bus::MessageBus;
use shared_types::ResourceType;

/// Test URLs for benchmarking
struct TestUrls;

impl TestUrls {
    /// Known ad URLs that should be blocked
    fn ad_urls() -> Vec<(&'static str, ResourceType)> {
        vec![
            ("https://doubleclick.net/ad.js", ResourceType::Script),
            ("https://googleadservices.com/pagead/conversion.js", ResourceType::Script),
            ("https://ads.facebook.com/ads/tracking.gif", ResourceType::Image),
            ("https://analytics.google.com/analytics.js", ResourceType::Script),
            ("https://pagead2.googlesyndication.com/pagead/js/adsbygoogle.js", ResourceType::Script),
            ("https://www.googletagmanager.com/gtm.js", ResourceType::Script),
            ("https://connect.facebook.net/en_US/fbevents.js", ResourceType::Script),
            ("https://static.ads-twitter.com/uwt.js", ResourceType::Script),
            ("https://www.google-analytics.com/ga.js", ResourceType::Script),
            ("https://cdn.taboola.com/libtrc/publisher.js", ResourceType::Script),
        ]
    }

    /// Legitimate URLs that should NOT be blocked
    fn safe_urls() -> Vec<(&'static str, ResourceType)> {
        vec![
            ("https://example.com/index.html", ResourceType::Document),
            ("https://github.com/user/repo", ResourceType::Document),
            ("https://stackoverflow.com/questions/12345", ResourceType::Document),
            ("https://cdn.jsdelivr.net/npm/jquery@3.6.0/dist/jquery.min.js", ResourceType::Script),
            ("https://fonts.googleapis.com/css?family=Roboto", ResourceType::Stylesheet),
            ("https://images.unsplash.com/photo-12345", ResourceType::Image),
            ("https://api.github.com/users/octocat", ResourceType::XmlHttpRequest),
            ("https://www.reddit.com/r/programming", ResourceType::Document),
            ("https://news.ycombinator.com/", ResourceType::Document),
            ("https://developer.mozilla.org/en-US/docs/Web", ResourceType::Document),
        ]
    }

    /// Mixed URLs (combination of ads and safe)
    fn mixed_urls() -> Vec<(&'static str, ResourceType)> {
        let mut urls = Vec::new();
        urls.extend(Self::ad_urls().into_iter().take(5));
        urls.extend(Self::safe_urls().into_iter().take(5));
        urls
    }
}

/// Benchmark: Filter Lookup Time (Single URL)
fn bench_filter_lookup_time(c: &mut Criterion) {
    let bus = MessageBus::new();
    let sender = bus.sender();
    let engine = AdBlockEngine::new(sender).unwrap();

    let mut group = c.benchmark_group("filter_lookup");

    // Test ad URL
    group.bench_function("ad_url_lookup", |b| {
        let (url, resource_type) = TestUrls::ad_urls()[0];
        b.iter(|| {
            let should_block = engine.should_block(black_box(url), black_box(resource_type.clone()));
            black_box(should_block);
        });
    });

    // Test safe URL
    group.bench_function("safe_url_lookup", |b| {
        let (url, resource_type) = TestUrls::safe_urls()[0];
        b.iter(|| {
            let should_block = engine.should_block(black_box(url), black_box(resource_type.clone()));
            black_box(should_block);
        });
    });

    group.finish();
}

/// Benchmark: Bulk Filter Lookups
fn bench_filter_bulk_lookup(c: &mut Criterion) {
    let bus = MessageBus::new();
    let sender = bus.sender();
    let engine = AdBlockEngine::new(sender).unwrap();

    let mut group = c.benchmark_group("filter_bulk_lookup");

    for url_count in [10, 100, 1000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(url_count),
            &url_count,
            |b, &count| {
                // Generate URL list
                let urls: Vec<_> = (0..count)
                    .map(|i| {
                        let test_urls = if i % 2 == 0 {
                            TestUrls::ad_urls()
                        } else {
                            TestUrls::safe_urls()
                        };
                        test_urls[i % test_urls.len()]
                    })
                    .collect();

                b.iter(|| {
                    let mut blocked_count = 0;
                    for (url, resource_type) in &urls {
                        if engine.should_block(url, resource_type.clone()) {
                            blocked_count += 1;
                        }
                    }
                    black_box(blocked_count);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Page Load with Blocking
fn bench_page_load_with_blocking(c: &mut Criterion) {
    let mut group = c.benchmark_group("page_load_blocking");

    // Simulate page with 50 resources (30 ads, 20 legitimate)
    let page_resources = {
        let mut resources = Vec::new();
        for _ in 0..30 {
            resources.push(TestUrls::ad_urls()[0]);
        }
        for _ in 0..20 {
            resources.push(TestUrls::safe_urls()[0]);
        }
        resources
    };

    group.bench_function("with_adblock", |b| {
        let bus = MessageBus::new();
        let sender = bus.sender();
        let engine = AdBlockEngine::new(sender).unwrap();

        b.iter(|| {
            let start = Instant::now();
            let mut blocked = 0;
            let mut allowed = 0;

            for (url, resource_type) in &page_resources {
                if engine.should_block(url, resource_type.clone()) {
                    blocked += 1;
                } else {
                    allowed += 1;
                    // Simulate resource download (only for allowed resources)
                    std::thread::sleep(Duration::from_micros(10));
                }
            }

            let load_time = start.elapsed();
            black_box((load_time, blocked, allowed));
        });
    });

    group.bench_function("without_adblock", |b| {
        b.iter(|| {
            let start = Instant::now();

            for _ in &page_resources {
                // Simulate resource download (all resources)
                std::thread::sleep(Duration::from_micros(10));
            }

            let load_time = start.elapsed();
            black_box(load_time);
        });
    });

    group.finish();
}

/// Benchmark: Filter Memory Overhead
fn bench_filter_memory_overhead(c: &mut Criterion) {
    c.bench_function("filter_memory_overhead", |b| {
        b.iter(|| {
            let bus = MessageBus::new();
            let sender = bus.sender();

            // Measure memory before
            let mem_before = std::mem::size_of::<AdBlockEngine>();

            // Create engine (loads filters)
            let engine = AdBlockEngine::new(sender).unwrap();

            // Estimate memory overhead
            let mem_after = std::mem::size_of_val(&engine);
            let overhead = mem_after.saturating_sub(mem_before);

            black_box((engine, overhead));
        });
    });
}

/// Benchmark: Blocking Decision Time by Resource Type
fn bench_blocking_by_resource_type(c: &mut Criterion) {
    let bus = MessageBus::new();
    let sender = bus.sender();
    let engine = AdBlockEngine::new(sender).unwrap();

    let mut group = c.benchmark_group("blocking_by_resource_type");

    let resource_types = [
        ("script", ResourceType::Script),
        ("image", ResourceType::Image),
        ("stylesheet", ResourceType::Stylesheet),
        ("document", ResourceType::Document),
        ("xhr", ResourceType::XmlHttpRequest),
    ];

    for (name, resource_type) in resource_types {
        group.bench_function(name, |b| {
            let ad_url = "https://doubleclick.net/tracker";
            b.iter(|| {
                let should_block = engine.should_block(black_box(ad_url), black_box(resource_type.clone()));
                black_box(should_block);
            });
        });
    }

    group.finish();
}

/// Benchmark: Filter Rule Complexity Impact
fn bench_filter_rule_complexity(c: &mut Criterion) {
    let bus = MessageBus::new();
    let sender = bus.sender();
    let engine = AdBlockEngine::new(sender).unwrap();

    let mut group = c.benchmark_group("filter_rule_complexity");

    // Simple pattern
    group.bench_function("simple_pattern", |b| {
        let url = "https://ads.com/banner.gif";
        b.iter(|| {
            let should_block = engine.should_block(black_box(url), ResourceType::Image);
            black_box(should_block);
        });
    });

    // Complex pattern with regex
    group.bench_function("complex_pattern", |b| {
        let url = "https://example.com/ads/tracker?id=12345&ref=homepage";
        b.iter(|| {
            let should_block = engine.should_block(black_box(url), ResourceType::Script);
            black_box(should_block);
        });
    });

    // Domain-specific pattern
    group.bench_function("domain_specific", |b| {
        let url = "https://subdomain.ads-network.com/js/tracking.js";
        b.iter(|| {
            let should_block = engine.should_block(black_box(url), ResourceType::Script);
            black_box(should_block);
        });
    });

    group.finish();
}

/// Benchmark: Concurrent Blocking Decisions
fn bench_concurrent_blocking(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_blocking");
    group.measurement_time(Duration::from_secs(15));

    for thread_count in [1, 2, 4, 8] {
        group.bench_with_input(
            BenchmarkId::from_parameter(thread_count),
            &thread_count,
            |b, &threads| {
                let bus = MessageBus::new();
                let sender = bus.sender();
                let engine = AdBlockEngine::new(sender).unwrap();

                b.iter(|| {
                    let handles: Vec<_> = (0..threads)
                        .map(|_| {
                            std::thread::spawn(|| {
                                let urls = TestUrls::mixed_urls();
                                let mut blocked = 0;

                                for (url, resource_type) in urls {
                                    if AdBlockEngine::new(MessageBus::new().sender())
                                        .unwrap()
                                        .should_block(url, resource_type)
                                    {
                                        blocked += 1;
                                    }
                                }
                                blocked
                            })
                        })
                        .collect();

                    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
                    black_box(results);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Blocking Effectiveness
fn bench_blocking_effectiveness(c: &mut Criterion) {
    let bus = MessageBus::new();
    let sender = bus.sender();
    let engine = AdBlockEngine::new(sender).unwrap();

    c.bench_function("blocking_effectiveness", |b| {
        b.iter(|| {
            let ad_urls = TestUrls::ad_urls();
            let safe_urls = TestUrls::safe_urls();

            // Count blocked ads
            let mut correctly_blocked = 0;
            for (url, resource_type) in &ad_urls {
                if engine.should_block(url, resource_type.clone()) {
                    correctly_blocked += 1;
                }
            }

            // Count false positives
            let mut false_positives = 0;
            for (url, resource_type) in &safe_urls {
                if engine.should_block(url, resource_type.clone()) {
                    false_positives += 1;
                }
            }

            // Calculate effectiveness
            let block_rate = correctly_blocked as f64 / ad_urls.len() as f64;
            let false_positive_rate = false_positives as f64 / safe_urls.len() as f64;

            black_box((block_rate, false_positive_rate, correctly_blocked, false_positives));
        });
    });
}

/// Benchmark: Filter Update Performance
fn bench_filter_update(c: &mut Criterion) {
    c.bench_function("filter_update", |b| {
        b.iter(|| {
            let bus = MessageBus::new();
            let sender = bus.sender();

            let start = Instant::now();

            // Create engine (loads filter list)
            let engine = AdBlockEngine::new(sender).unwrap();

            let load_time = start.elapsed();

            black_box((engine, load_time));
        });
    });
}

/// Benchmark: Cache Hit Rate Impact
fn bench_cache_impact(c: &mut Criterion) {
    let bus = MessageBus::new();
    let sender = bus.sender();
    let engine = AdBlockEngine::new(sender).unwrap();

    let mut group = c.benchmark_group("cache_impact");

    // Same URL repeated (high cache hit)
    group.bench_function("high_cache_hit", |b| {
        let url = TestUrls::ad_urls()[0];
        b.iter(|| {
            for _ in 0..100 {
                let should_block = engine.should_block(black_box(url.0), black_box(url.1.clone()));
                black_box(should_block);
            }
        });
    });

    // Different URLs each time (low cache hit)
    group.bench_function("low_cache_hit", |b| {
        let urls = TestUrls::mixed_urls();
        b.iter(|| {
            for (url, resource_type) in &urls {
                let should_block = engine.should_block(black_box(url), black_box(resource_type.clone()));
                black_box(should_block);
            }
        });
    });

    group.finish();
}

/// Benchmark: Blocking Latency Distribution
fn bench_blocking_latency_distribution(c: &mut Criterion) {
    let bus = MessageBus::new();
    let sender = bus.sender();
    let engine = AdBlockEngine::new(sender).unwrap();

    c.bench_function("blocking_latency_distribution", |b| {
        let urls = TestUrls::mixed_urls();

        b.iter(|| {
            let mut latencies = Vec::new();

            for (url, resource_type) in &urls {
                let start = Instant::now();
                let should_block = engine.should_block(url, resource_type.clone());
                let latency = start.elapsed();

                latencies.push(latency);
                black_box(should_block);
            }

            // Calculate statistics
            latencies.sort();
            let min = latencies.first().copied().unwrap_or_default();
            let max = latencies.last().copied().unwrap_or_default();
            let median = latencies.get(latencies.len() / 2).copied().unwrap_or_default();

            black_box((min, median, max));
        });
    });
}

criterion_group!(
    benches,
    bench_filter_lookup_time,
    bench_filter_bulk_lookup,
    bench_page_load_with_blocking,
    bench_filter_memory_overhead,
    bench_blocking_by_resource_type,
    bench_filter_rule_complexity,
    bench_concurrent_blocking,
    bench_blocking_effectiveness,
    bench_filter_update,
    bench_cache_impact,
    bench_blocking_latency_distribution,
);

criterion_main!(benches);
