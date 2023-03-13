# Tub 🛁

An asynchronous pool for managing reusable values.

All values in the pool are initialized when the pool is created. Values can be retrieved from the pool asynchronously. When the retrieved out value goes out of scope, the value is returned to the pool and made available for retrieval at a later time.

## Usage

To use Tub, add this to your Cargo.toml:

```toml
[dependencies]
tub = "0.2.2"
```

Then create and use a pool like so:

```rust
use tub::Pool;

#[tokio::main]
async fn main() {
   // Create a pool
   let pool = Pool::from_initializer(10, || Box { value: 123 });

   // Get a value from the pool
   let mut box1 = pool.acquire().await;

   // Use the value
   box1.foo();

   // Modify the value
   *box1 = Box { value: 456 };

   // Return the value to the pool
   drop(box1);
}

struct Box {
  value: u32
}

impl Box {
  fn foo(&mut self) { }
}
```