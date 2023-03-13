# Tub 🛁

An asynchronous pool for managing reusable values.

All values in the pool are initialized when the pool is created. Values can be retrieved from the pool asynchronously. When the retrieved out value goes out of scope, the value is returned to the pool and made available for retrieval at a later time.