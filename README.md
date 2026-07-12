# `no_std_tool`

A universal foundation library designed for `#![no_std]` bare-metal Rust projects.

This crate consolidates essential utilities that are frequently required in embedded, OS-development, or other resource-constrained environments where the standard library is unavailable. By centralizing these dependencies, `no_std_tool` isolates the complexity of `no_std` environments and prevents application-layer code from becoming tightly coupled with low-level hardware or environment details.

## Modules & Features

- **`sync`**: Synchronization primitives tailored for bare-metal targets.
  - `SpinMutex` & `SpinMutexGuard`: A lightweight lock relying purely on atomic operations and spin hints.
  - `CachePadded`: Eliminates false sharing by aligning structures to typical cache line boundaries (64/128 bytes).
  - `Backoff`: A spin-then-yield backoff helper for tight polling loops to prevent CPU exhaustion.
  - Full suite of standard library Atomics (`AtomicBool`, `AtomicPtr`, `AtomicU8`..`AtomicU64`, etc.).
- **`collections`**: Dynamic data structures powered by `alloc`.
  - `HashMap` and `HashSet` backed by `hashbrown`.
  - `mpsc_queue::BoundedQueue`: A lock-free, wait-free compatible bounded multi-producer single-consumer queue.
  - High-performance, non-cryptographic `ahash` hashing algorithms (faster than `SipHash` and DOS-resistant).
- **`math`**: Zero-float mathematical engine.
  - Pure integer approximations of floating-point operations like `exp_approx_q16` and `rsqrt_approx_i32`.
  - Perfect for environments lacking hardware FPU support.
- **`debug`**: Debugging and lifecycle tracking.
  - `ScopedResource`: A global atomic tracker to detect memory leaks and ensure thread drops.
- **`macros`**: Declarative macros for module scaffolding.
  - `base!()` to inject `alloc` and conditional `std` testing blocks.
  - `module!{}` to wrap and suppress common lints during `#![no_std]` integration.
  - `auto_static!`: A specialized macro to safely and effortlessly declare `#![no_std]` thread-safe global static arrays or partitioned registries.

## Usage

Simply add `no_std_tool` to your `Cargo.toml`. This crate already encapsulates and configures popular `no_std` crates like `lazy_static`, `rkyv`, `hashbrown`, and `ahash`.

```toml
[dependencies]
no_std_tool = { path = "../no_std_tool" }
```

```rust
#![no_std]

use no_std_tool::collections::HashMap;
use no_std_tool::sync::SpinMutex;
use no_std_tool::lazy_static;
use no_std_tool::rkyv;

lazy_static! {
    static ref GLOBAL_MAP: SpinMutex<HashMap<i32, &'static str>> = {
        let mut m = HashMap::new();
        m.insert(1, "Hello Bare Metal");
        SpinMutex::new(m)
    };
}
```

## Complexity Auditing

This project is configured for automated algorithmic complexity and performance auditing via `CovOpt-Analyzer`.

You can verify the algorithmic complexity of the underlying operations using:
```bash
covopt audit
```

## License

MIT
