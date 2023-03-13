extern crate tub;

use proptest::prelude::*;
use tokio::runtime::Runtime;
use tub::Pool;

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

#[tokio::test]
async fn guarded_value_is_mutable() {
    let pool = Pool::from_copy(10, 1);
    let mut box1 = pool.get().await;
    assert_eq!(pool.remaining_capacity(), 9);
    assert_eq!(*box1, 1);
    *box1 = 2;
    assert_eq!(*box1, 2);
}

#[tokio::test]
async fn mutated_value_is_returned_to_pool() {
    let pool = Pool::from_copy(1, 1);
    let mut b = pool.get().await;
    assert_eq!(pool.remaining_capacity(), 0);
    assert_eq!(*b, 1);
    *b = 2;
    assert_eq!(*b, 2);
    drop(b);
    assert_eq!(pool.remaining_capacity(), 1);
    let b = pool.get().await;
    assert_eq!(pool.remaining_capacity(), 0);
    assert_eq!(*b, 2);
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
                    guards.push(pool.get().await);
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
}
