use async_resource::PoolConfig;
use criterion::{criterion_group, BenchmarkId, Criterion};
use futures::future::join_all;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::Arc;
use std::time::Duration;

static COUNT: usize = 100_000;

pub fn tub_and_simple_pool(c: &mut Criterion) {
    let mut group = c.benchmark_group("Tub vs. Simple Pool");

    let iters = vec![
        10_000, 50_000, 100_000, 200_000, 300_000, 400_000, 500_000, 1_000_000,
    ];
    for i in iters {
        group.bench_with_input(BenchmarkId::new("tub", i), &i, |b, i| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            b.to_async(rt)
                .iter(|| run_tub(tub::Pool::from_initializer(1, || 1), *i));
        });

        group.bench_with_input(BenchmarkId::new("simple-pool", i), &i, |b, i| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            b.to_async(rt)
                .iter(|| run_simple_pool(create_simple_pool(), *i));
        });
    }

    group.finish();
}

pub fn scaled(c: &mut Criterion) {
    let mut group = c.benchmark_group("Acquire & Release (Scaled)");

    let iters = (0..19).map(|i| 2usize.pow(i as u32)).collect::<Vec<_>>();
    for i in iters {
        group.bench_with_input(BenchmarkId::new("tub", i), &i, |b, i| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            b.to_async(rt)
                .iter(|| run_tub(tub::Pool::from_initializer(1, || 1), *i));
        });

        group.bench_with_input(BenchmarkId::new("async-object-pool", i), &i, |b, i| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            b.to_async(rt)
                .iter(|| run_aop(async_object_pool::Pool::new(1), *i));
        });

        group.bench_with_input(BenchmarkId::new("simple-pool", i), &i, |b, i| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            b.to_async(rt)
                .iter(|| run_simple_pool(create_simple_pool(), *i));
        });
    }

    group.finish();
}

pub fn fixed(c: &mut Criterion) {
    let mut group = c.benchmark_group("Acquire & Release (Fixed)");

    group.bench_function(BenchmarkId::new("tub", 1), |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        b.to_async(rt)
            .iter(|| run_tub(tub::Pool::from_initializer(1, || 1), COUNT));
    });
    group.bench_function(BenchmarkId::new("async-object-pool", 2), |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        b.to_async(rt)
            .iter(|| run_aop(async_object_pool::Pool::new(1), COUNT));
    });
    group.bench_function(BenchmarkId::new("simple-pool", 3), |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        b.to_async(rt)
            .iter(|| run_simple_pool(create_simple_pool(), COUNT));
    });
    group.bench_function(BenchmarkId::new("async-resource", 4), |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        b.to_async(rt)
            .iter(|| run_async_resource_pool(create_async_resource_pool(), COUNT));
    });

    group.plot_config(
        criterion::PlotConfiguration::default().summary_scale(criterion::AxisScale::Logarithmic),
    );

    group.finish();
}

async fn run_tub(pool: tub::Pool<u32>, iters: usize) {
    let pool = Arc::new(pool);
    join_all(
        (0..iters)
            .map(|_| {
                let pool = pool.clone();
                tokio::spawn(async move {
                    let _resource = pool.acquire().await;
                })
            })
            .collect::<Vec<_>>(),
    )
    .await;
}

async fn run_aop(pool: async_object_pool::Pool<u32>, iters: usize) {
    let pool = Arc::new(pool);
    join_all(
        (0..iters)
            .map(|_| {
                let pool = pool.clone();
                tokio::spawn(async move {
                    let resource = pool.take_or_create(|| 1).await;
                    pool.put(resource).await;
                })
            })
            .collect::<Vec<_>>(),
    )
    .await;
}

async fn run_simple_pool(pool: simple_pool::ResourcePool<u32>, iters: usize) {
    let pool = Arc::new(pool);
    join_all(
        (0..iters)
            .map(|_| {
                let pool = pool.clone();
                tokio::spawn(async move {
                    let _resource = pool.get().await;
                })
            })
            .collect::<Vec<_>>(),
    )
    .await;
}

async fn run_async_resource_pool(pool: async_resource::Pool<u32, ()>, iters: usize) {
    let pool = Arc::new(pool);
    join_all(
        (0..iters)
            .map(|_| {
                let pool = pool.clone();
                tokio::spawn(async move {
                    let _resource = pool.acquire().await;
                })
            })
            .collect::<Vec<_>>(),
    )
    .await;
}

fn create_async_resource_pool() -> async_resource::Pool<u32, ()> {
    fn counter_pool_config() -> PoolConfig<u32, ()> {
        let source = Arc::new(AtomicU32::new(0));
        PoolConfig::<u32, ()>::new(move || {
            let s = source.clone();
            async move { Ok(s.fetch_add(1, SeqCst)) }
        })
    }

    counter_pool_config().build()
}

fn create_simple_pool() -> simple_pool::ResourcePool<u32> {
    let pool = simple_pool::ResourcePool::with_capacity(1);
    pool.append(1);
    pool
}

criterion_group!(
    name = fixed_group;
    config = crate::default_config();
    targets = fixed
);

criterion_group!(
    name = scaled_group;
    config = crate::default_config()
    .nresamples(50)
    .sample_size(25)
    .warm_up_time(Duration::from_millis(10))
    .measurement_time(Duration::from_millis(25));
    targets = scaled, tub_and_simple_pool
);
