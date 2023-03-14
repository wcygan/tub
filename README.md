# Tub 🛁

A *blazingly fast* object pool for Rust.

Values are retrieved from the pool asynchronously. When the retrieved value goes out of scope, the value is returned to the pool.

## Usage

To use Tub, add this to your Cargo.toml:

```toml
[dependencies]
tub = "0.3.0"
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