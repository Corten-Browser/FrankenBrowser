use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::time::Duration;

// Import components for benchmarking
use message_bus::{MessageBus, MessageSender};
use shared_types::{BrowserMessage, ResourceType};
use config_manager::Config;
use adblock_engine::AdBlockEngine;

/// Benchmark: Message Bus Creation and Initialization
fn bench_message_bus_creation(c: &mut Criterion) {
    c.bench_function("message_bus_creation", |b| {
        b.iter(|| {
            let bus = MessageBus::new();
            black_box(bus);
        });
    });
}

/// Benchmark: Message Bus Send Throughput
fn bench_message_bus_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_bus_throughput");

    for msg_count in [10, 100, 1000] {
        group.bench_with_input(BenchmarkId::from_parameter(msg_count), &msg_count, |b, &count| {
            let bus = MessageBus::new();
            let sender = bus.sender();

            b.iter(|| {
                for _ in 0..count {
                    sender.send(BrowserMessage::Shutdown).ok();
                }
            });
        });
    }

    group.finish();
}

/// Benchmark: Message Bus Handler Registration
fn bench_message_bus_handlers(c: &mut Criterion) {
    c.bench_function("message_bus_handler_registration", |b| {
        b.iter(|| {
            let mut bus = MessageBus::new();
            bus.register_handler(|_msg| {
                // Handler logic
            });
            black_box(bus);
        });
    });
}

/// Benchmark: Configuration Loading
fn bench_config_loading(c: &mut Criterion) {
    c.bench_function("config_load_default", |b| {
        b.iter(|| {
            let config = Config::default();
            black_box(config);
        });
    });
}

/// Benchmark: Configuration Serialization
fn bench_config_serialization(c: &mut Criterion) {
    let config = Config::default();

    c.bench_function("config_to_toml", |b| {
        b.iter(|| {
            let toml_str = toml::to_string(&config).unwrap();
            black_box(toml_str);
        });
    });
}

/// Benchmark: Ad Block Engine Initialization
fn bench_adblock_init(c: &mut Criterion) {
    let bus = MessageBus::new();
    let sender = bus.sender();

    c.bench_function("adblock_engine_init", |b| {
        b.iter(|| {
            let engine = AdBlockEngine::new(sender.clone());
            black_box(engine);
        });
    });
}

/// Benchmark: Ad Block Filter Matching
fn bench_adblock_matching(c: &mut Criterion) {
    let mut group = c.benchmark_group("adblock_filter_matching");

    let bus = MessageBus::new();
    let sender = bus.sender();
    let engine = AdBlockEngine::new(sender).unwrap();

    // Test URLs
    let test_cases = vec![
        ("https://example.com/ad.js", ResourceType::Script, "ad_script"),
        ("https://doubleclick.net/tracker.gif", ResourceType::Image, "tracker_image"),
        ("https://googleadservices.com/pagead", ResourceType::Document, "ad_document"),
        ("https://github.com/user/repo", ResourceType::Document, "safe_site"),
    ];

    for (url, resource_type, name) in test_cases {
        group.bench_with_input(BenchmarkId::from_parameter(name), &url, |b, url| {
            b.iter(|| {
                let should_block = engine.should_block(black_box(url), resource_type.clone());
                black_box(should_block);
            });
        });
    }

    group.finish();
}

/// Benchmark: Browser Message Creation
fn bench_message_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_creation");

    group.bench_function("navigate_request", |b| {
        b.iter(|| {
            let msg = BrowserMessage::NavigateRequest {
                tab_id: 1,
                url: "https://example.com".parse().unwrap(),
            };
            black_box(msg);
        });
    });

    group.bench_function("http_request", |b| {
        b.iter(|| {
            let msg = BrowserMessage::HttpRequest {
                request_id: 12345,
                url: "https://example.com".parse().unwrap(),
                headers: std::collections::HashMap::new(),
            };
            black_box(msg);
        });
    });

    group.bench_function("create_tab", |b| {
        b.iter(|| {
            let msg = BrowserMessage::CreateTab {
                parent_window: 0,
            };
            black_box(msg);
        });
    });

    group.finish();
}

/// Benchmark: Message Serialization (for IPC)
fn bench_message_serialization(c: &mut Criterion) {
    let msg = BrowserMessage::NavigateRequest {
        tab_id: 1,
        url: "https://example.com".parse().unwrap(),
    };

    c.bench_function("message_to_json", |b| {
        b.iter(|| {
            let json = serde_json::to_string(&msg).unwrap();
            black_box(json);
        });
    });

    c.bench_function("message_from_json", |b| {
        let json = serde_json::to_string(&msg).unwrap();
        b.iter(|| {
            let msg: BrowserMessage = serde_json::from_str(&json).unwrap();
            black_box(msg);
        });
    });
}

/// Benchmark: Concurrent Message Sending
fn bench_concurrent_messages(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_messages");
    group.measurement_time(Duration::from_secs(10));

    for thread_count in [1, 2, 4, 8] {
        group.bench_with_input(
            BenchmarkId::from_parameter(thread_count),
            &thread_count,
            |b, &threads| {
                let bus = MessageBus::new();
                let sender = bus.sender();

                b.iter(|| {
                    let handles: Vec<_> = (0..threads)
                        .map(|_| {
                            let sender = sender.clone();
                            std::thread::spawn(move || {
                                for _ in 0..100 {
                                    sender.send(BrowserMessage::Shutdown).ok();
                                }
                            })
                        })
                        .collect();

                    for handle in handles {
                        handle.join().unwrap();
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: ResourceType Conversions
fn bench_resource_type_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("resource_type_operations");

    group.bench_function("clone", |b| {
        let rt = ResourceType::Script;
        b.iter(|| {
            let cloned = rt.clone();
            black_box(cloned);
        });
    });

    group.bench_function("to_string", |b| {
        let rt = ResourceType::Script;
        b.iter(|| {
            let s = format!("{:?}", rt);
            black_box(s);
        });
    });

    group.finish();
}

/// Benchmark: Configuration Field Access
fn bench_config_field_access(c: &mut Criterion) {
    let config = Config::default();

    c.bench_function("config_field_access", |b| {
        b.iter(|| {
            let homepage = config.browser.homepage.clone();
            let timeout = config.network.timeout_seconds;
            let enabled = config.adblock.enabled;
            black_box((homepage, timeout, enabled));
        });
    });
}

criterion_group!(
    benches,
    bench_message_bus_creation,
    bench_message_bus_throughput,
    bench_message_bus_handlers,
    bench_config_loading,
    bench_config_serialization,
    bench_adblock_init,
    bench_adblock_matching,
    bench_message_creation,
    bench_message_serialization,
    bench_concurrent_messages,
    bench_resource_type_ops,
    bench_config_field_access,
);

criterion_main!(benches);
