# Anachro Forth (core)

Anachro Forth is a forth-inspired, bytecode-compiled
scripting language for Anachro Powerbus platform.

## Use Case

The intended use case is to write and compile scripts on a Host PC, and to load and execute these scripts in a constrained, no_std environment, such as on embedded systems or WASM targets.

## Contents

This crate contains the core components of the language,
including:

* **The compiler** - which converts text-based source code
  into a bytecode representation. The compiler is only
  compatible with "std" platforms
* **The runtime** - which executes the compiled bytecode.
  Additionally, the runtime has two implementations:
  * The "std" runtime, which uses heap allocations for convenience
  * The "no_std" runtime, which is suitable for constrained
    environments, and does not require heap allocations
* **The Builtins** - Which are functions available to be used from the
  scripts, but are implemented in Rust
* **The Wire Format** - which is used to serialize and deserialize
  the compiled bytecode, allowing it to be sent or stored for execution
  on another device

## Stability

This project is in early, active, development. Frequent breaking changes
are expected in the near future. Please [contact me](mailto:james@onevariable.com)
if you would like to use Anachro Forth for your project or product

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  http://www.apache.org/licenses/LICENSE-2.0)

- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
