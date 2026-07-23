# no_std_tool

`no_std_tool` is the foundational utility crate for the Universal Project, designed specifically for extreme edge computing, bare-metal environments, and custom OS kernels.

## Tech Stack
- **Rust Edition 2024** (`#![no_std]` by default)
- **Zero-Allocation Data Structures**
- **Lock-Free Synchronization**: Built-in architecture-specific inline assembly for atomic operations.
- **Micro-architecture Tuning**: Integrates with the CovOpt Ontology for Zero-Entropy Tuning via `covopt_param!`.

## Features
- **Wait-Free Primitives**: `SpinMutex`, `IrqSafeMutex` for IRQ masking on `x86_64` and `aarch64`.
- **Boilerplate Macros**: `base!` and `module!` to seamlessly strip standard library dependencies.
- **CovOpt Integration**: Compile-time or runtime parameter injection for Auto-Tuning systems.

## Example

```rust
#![no_std]

use no_std_tool::sync::SpinMutex;
use no_std_tool::covopt_param;

// Define a dynamically tunable limit
const SPIN_LIMIT: u32 = covopt_param!("SPIN_LIMIT", 10_000, 100..100_000);

static GLOBAL_STATE: SpinMutex<u32> = SpinMutex::new(0);

fn main() {
    // Acquire a bounded spin-lock that prevents infinite hanging
    match GLOBAL_STATE.lock() {
        Ok(mut guard) => {
            *guard += 1;
        }
        Err(_) => {
            // Handle timeout / fallback gracefully in bare-metal
        }
    }
}
```
