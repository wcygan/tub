extern crate tub;

use proptest::prelude::*;
use std::hint::black_box;
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::sync::Barrier;
use tub::Pool;

#[tokio::test]
async fn readme() {
    // Create a pool
    let pool: Pool<Box> = (0..10).map(|_| Box { _value: 123 }).into();

    // Get a value from the pool
    let mut box1 = pool.acquire().await;

    // Use the value
    box1.foo();

    // Modify the value
    *box1 = Box { _value: 456 };

    // Return the value to the pool
    drop(box1);

    struct Box {
        _value: u32,
    }

    impl Box {
        fn foo(&mut self) {}
    }
}

#[test]
fn test_new_from_vec() {
    let pool = Pool::from_vec(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    assert_eq!(pool.remaining_capacity(), 10);
}

#[test]
fn test_new_from_initializer() {
    let pool = Pool::from_initializer(10, || 1);
    assert_eq!(pool.remaining_capacity(), 10);
}

#[test]
fn test_new_from_copy() {
    let pool = Pool::from_copy(10, 1);
    assert_eq!(pool.remaining_capacity(), 10);
}

#[test]
fn test_new_from_clone() {
    let pool = Pool::from_clone(10, &1);
    assert_eq!(pool.remaining_capacity(), 10);
}

#[test]
fn test_new_from_default() {
    let pool: Pool<i32> = Pool::from_default(10);
    assert_eq!(pool.remaining_capacity(), 10);
}

#[test]
fn test_new_from_iter() {
    let pool = Pool::from_iter(0..10);
    assert_eq!(pool.remaining_capacity(), 10);
}

#[test]
fn test_from_trait_1() {
    let pool: Pool<u32> = (0..100).into();
    assert_eq!(pool.remaining_capacity(), 100);
}

#[test]
fn test_from_trait_2() {
    let pool: Pool<String> = (0..100).map(|_| String::from("hello")).into();
    assert_eq!(pool.remaining_capacity(), 100);
}

#[test]
fn test_from_trait_3() {
    let pool: Pool<Vec<u32>> = (0..10).map(|_| vec![]).into();
    assert_eq!(pool.remaining_capacity(), 10);
}

#[test]
fn test_clone_a_pool() {
    let pool = Pool::from_copy(10, 1);
    let pool2 = pool.clone();
    assert_eq!(pool.remaining_capacity(), 10);
    assert_eq!(pool2.remaining_capacity(), 10);
}

#[tokio::test]
async fn guarded_value_is_mutable() {
    let pool = Pool::from_copy(10, 1);
    let mut box1 = pool.acquire().await;
    assert_eq!(pool.remaining_capacity(), 9);
    assert_eq!(*box1, 1);
    *box1 = 2;
    assert_eq!(*box1, 2);
}

#[tokio::test]
async fn mutated_value_is_returned_to_pool() {
    let pool = Pool::from_copy(1, 1);
    let mut b = pool.acquire().await;
    assert_eq!(pool.remaining_capacity(), 0);
    assert_eq!(*b, 1);
    *b = 2;
    assert_eq!(*b, 2);
    drop(b);
    assert_eq!(pool.remaining_capacity(), 1);
    let b = pool.acquire().await;
    assert_eq!(pool.remaining_capacity(), 0);
    assert_eq!(*b, 2);
}

#[tokio::test]
async fn deadlock_check_1() {
    let pool = Pool::from_copy(1, 0);

    let futures = (0..100)
        .map(|_| {
            let pool = pool.clone();
            tokio::spawn(async move {
                let mut b = pool.acquire().await;
                *b += 1;
                drop(b);
            })
        })
        .collect::<Vec<_>>();

    for future in futures {
        future.await.unwrap();
    }

    assert_eq!(pool.remaining_capacity(), 1);
    let v = pool.acquire().await;
    assert_eq!(*v, 100);
}

#[tokio::test]
async fn deadlock_check_2() {
    let pool = Arc::new(Pool::from_copy(1, 1));
    let barrier = Arc::new(Barrier::new(2));

    let f1 = tokio::spawn({
        let pool = pool.clone();
        let barrier = barrier.clone();
        async move {
            let mut b = pool.acquire().await;
            *b = 2;
            drop(b);
            barrier.wait().await;
        }
    });

    let f2 = tokio::spawn({
        let pool = pool.clone();
        let barrier = barrier.clone();
        async move {
            barrier.wait().await;
            let mut b = pool.acquire().await;
            *b = 3;
        }
    });

    f1.await.unwrap();
    f2.await.unwrap();

    assert_eq!(pool.remaining_capacity(), 1);
    let v = pool.acquire().await;
    assert_eq!(*v, 3);
}

#[tokio::test]
async fn deadlock_check_3() {
    let pool = Arc::new(Pool::from_copy(2, 1));

    let handles: Vec<_> = (0..3)
        .map(|_| {
            tokio::spawn({
                let pool = pool.clone();
                async move {
                    let _resource = pool.acquire().await;
                }
            })
        })
        .collect();

    let handles2: Vec<_> = (0..3)
        .map(|_| {
            tokio::spawn({
                let pool = pool.clone();
                async move {
                    let _resource = pool.acquire().await;
                }
            })
        })
        .collect();

    // wait for all tasks to complete
    for handle in handles.into_iter().chain(handles2.into_iter()) {
        handle.await.unwrap();
    }
}

#[tokio::test]
async fn deadlock_check_4() {
    let pool = Arc::new(Pool::from_copy(1, 1));
    let tasks = (0..100)
        .map(|_| {
            tokio::spawn({
                let pool = pool.clone();
                async move {
                    let resource = pool.acquire().await;
                    // Sleep to increase the odds that other tasks are waiting for the pool.
                    tokio::time::sleep(Duration::from_nanos(1)).await;
                    black_box(resource);
                }
            })
        })
        .collect::<Vec<_>>();

    for task in tasks {
        task.await.unwrap();
    }
}

#[tokio::test]
async fn deadlock_check_5() {
    let pool = Arc::new(Pool::from_copy(1, 1));
    let tasks = (0..100_000)
        .map(|_| {
            tokio::spawn({
                let pool = pool.clone();
                async move {
                    let resource = pool.acquire().await;
                    black_box(resource);
                }
            })
        })
        .collect::<Vec<_>>();

    for task in tasks {
        task.await.unwrap();
    }
}

proptest! {
    #[test]
    fn new_from_vec_prop_property(vec in any::<Vec<u8>>()) {
        if !vec.is_empty() {
            let pool = Pool::from_vec(vec.clone());
            assert_eq!(pool.remaining_capacity(), vec.len());
        }
    }

    #[test]
    fn new_from_iter_prop_property(vec in any::<Vec<u8>>()) {
        if !vec.is_empty() {
            let pool = Pool::from_iter(vec.clone().into_iter());
            assert_eq!(pool.remaining_capacity(), vec.len());
        }
    }

    #[test]
    fn guard_returns_value_property(u in 0..50 as usize) {
        Runtime::new().unwrap().block_on(async {
            if u > 0 {
                let pool = Pool::from_copy(u, 1);
                let mut guards = Vec::new();
                for _ in 0..u {
                    guards.push(pool.acquire().await);
                }
                assert_eq!(pool.remaining_capacity(), 0);
                for guard in guards {
                    drop(guard);
                }
                assert_eq!(pool.remaining_capacity(), u);
            }
        });
    }

    #[test]
    fn size_property(u in any::<u8>()) {
        if u > 0 {
            let pool = Pool::from_copy(u as usize, 1);
            assert_eq!(pool.remaining_capacity(), u as usize);
        }
    }

    #[test]
    fn progress_property(_ in 0..2 as usize) {
        Runtime::new().unwrap().block_on(async {
            let pool = Arc::new(Pool::from_copy(1, 1));
            let tasks = (0..100)
                .map(|_| {
                    tokio::spawn({
                        let pool = pool.clone();
                        async move {
                            let resource = pool.acquire().await;
                            black_box(resource);
                        }
                    })
                })
                .collect::<Vec<_>>();

            for task in tasks {
                task.await.unwrap();
            }
        });
    }
}
