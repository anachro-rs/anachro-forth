# A4 - The Anachro Forth Compiler

Anachro Forth (A4) is a forth-inspired, bytecode-compiled
scripting language for Anachro Powerbus platform.

The compiler can be installed via cargo:

```bash
cargo install -f a4
```

## Use Case

The intended use case is to write and compile scripts on a Host PC, and to load and execute these scripts in a constrained, no_std environment, such as on embedded systems or WASM targets.

The compiler is capable of taking text-based source files (`.fth`) and converting them into compressed byte code files (`.a4`), which may be loaded on constrained environments

## Usage

```text
~ a4 --help

a4 0.0.4
A forth-inspired, bytecode-compiled scripting language for Anachro Powerbus

USAGE:
    a4 <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    compile    Compile the provided ".fth" source file into an ".a4" compiled output
    help       Prints this message or the help of the given subcommand(s)
    repl       Start an interactive "Read, Evaluate, Print, Loop" session
    run        Run a given ".fth" file, exiting after execution
```
