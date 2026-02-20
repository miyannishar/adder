# Adder — A Simple Expression Compiler

A small compiler for the **Adder** language: 32-bit integers and three unary operations (`add1`, `sub1`, `negate`). It demonstrates the full pipeline from source text to x86-64 assembly and executable.

**Learning guide:** [docs/CONCEPTS.md](docs/CONCEPTS.md) explains every concept used in this assignment (grammars, parsing, AST, code generation, x86-64, linking, Rust FFI, Make, etc.).  
**Run flow:** [docs/RUN_FLOW.md](docs/RUN_FLOW.md) describes what happens when you run a test: file flow and line-by-line behavior of the compiler and runtime.

---

## Setup

### Prerequisites

- **Rust and Cargo** — [Install from rust-lang.org](https://www.rust-lang.org/tools/install)
- **NASM** (Netwide Assembler) — for assembling the generated `.s` files
- **GCC or Clang** — for linking (usually pre-installed on macOS/Linux)

### Install on macOS

```bash
# Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# NASM
brew install nasm
```

### Apple Silicon (M1/M2/M3)

The compiler emits **x86-64** assembly; the runtime must be built for x86-64 so it can link with it. On Apple Silicon, add the x86-64 macOS target and the resulting `.run` binaries will run under Rosetta 2:

```bash
rustup target add x86_64-apple-darwin
```

Then `make test/37.run` (and similar) will work. Without this target, you'll see a linker error: `found architecture 'x86_64', required architecture 'arm64'`.

### System requirements

- x86-64 capable system. On Apple Silicon Macs, x86-64 runs via Rosetta 2.
- Linux, macOS, or Windows with WSL.

### Build the compiler

From the project root:

```bash
cargo build
```

---

## How to Run

### Single test: compile and run

```bash
# Compile a .snek file to a .run executable
make test/37.run

# Run it
./test/37.run
# Output: 37

# Inspect the generated assembly
cat test/37.s
```

### Run all tests and generate transcript

```bash
bash transcript_gen.sh
cat transcript.txt
```

### Clean generated files

```bash
make clean
```

---

## Compiler Pipeline

End-to-end flow from source to output:

```
Source (.snek)
     │
     ▼  sexp::parse()         ← scanning + tokenizing
 S-Expression Tree
     │
     ▼  parse_expr()           ← recursive descent parser
 AST (Expr enum)
     │
     ▼  compile_expr()        ← post-order tree walk
 Assembly String
     │
     ▼  File::create()        ← emit .s file
 .s file
     │
     ▼  nasm -f macho64       ← assembler
 .o object file
     │
     ▼  ar rcs                ← archiver (static lib)
 libour_code.a
     │
     ▼  rustc -L runtime/      ← linker
 .run executable
     │
     ▼  ./test/X.run
 Output (integer)
```

---

## The Adder Language

### Grammar (BNF)

```
<expr> ::= <number>
         | ( add1 <expr> )
         | ( sub1 <expr> )
         | ( negate <expr> )
```

`<number>` is a 32-bit signed integer.

### Semantics

- **Numbers** evaluate to themselves.
- **add1(e)** evaluates `e` and adds 1.
- **sub1(e)** evaluates `e` and subtracts 1.
- **negate(e)** evaluates `e` and multiplies by -1.

### Examples

| Source | Result |
|--------|--------|
| `37` | 37 |
| `(add1 (sub1 5))` | 5 |
| `(negate (add1 3))` | -4 |
| `(sub1 (sub1 (add1 73)))` | 72 |

---

## Component Overview

### 1. Abstract Syntax Tree (`Expr` enum)

The AST is the single intermediate representation. In Rust:

```rust
enum Expr {
    Num(i32),
    Add1(Box<Expr>),
    Sub1(Box<Expr>),
    Negate(Box<Expr>),
}
```

Recursive types need a fixed size at compile time, so we use `Box<Expr>` to put subexpressions on the heap. Without `Box`, the type would be infinitely large.

### 2. The `sexp` crate (scanning and parsing to S-expressions)

The compiler uses the `sexp` crate to turn source text into an S-expression tree. The crate:

- **Scans** the input (tokenization, maximal munch).
- **Parses** into a tree of atoms (integers, symbols, strings) and lists.

Our code then walks that tree with `parse_expr()` to build the AST. We do not implement a lexer/parser for the surface syntax ourselves; we reuse S-expression parsing.

### 3. Parser: `parse_expr()`

`parse_expr()` is a recursive-descent parser over the S-expression:

- **Atom integer** → `Expr::Num(n)` (with i32 range check).
- **List `(add1 e)`** → `Expr::Add1(Box::new(parse_expr(e)))`.
- **List `(sub1 e)`** → `Expr::Sub1(...)`.
- **List `(negate e)`** → `Expr::Negate(...)`.

One token of lookahead (the first element of the list) is enough to choose the production, so the grammar is LL(1).

### 4. Code generator: `compile_expr()`

Code generation uses an **accumulator** in a single register, `rax`:

- Every subexpression is compiled so that **its result is left in `rax`**.
- The current node’s code may use that value and again leave the result in `rax`.

This is a single bottom-up (post-order) pass: children are compiled first, then the parent emits one or more instructions. So the approach fits an S-attributed grammar: attributes flow from children to parent.

- **Num(n)** → `mov rax, n`
- **Add1(e)** → code for `e` then `add rax, 1`
- **Sub1(e)** → code for `e` then `sub rax, 1`
- **Negate(e)** → code for `e` then `neg rax`

### 5. Generated assembly: example walkthrough

For `(negate (add1 3))` we want: evaluate 3, add 1 (→ 4), negate (→ -4).

Generated instructions:

```asm
  mov rax, 3    ; load 3 into rax
  add rax, 1    ; rax = 4
  neg rax       ; rax = -4
  ret           ; return (caller reads rax)
```

The compiler emits this inside a `our_code_starts_here` function that the runtime calls. The **System V AMD64 ABI** specifies that the return value is in `rax`, which matches what we generate.

### 6. Runtime (`runtime/start.rs`)

The runtime is a small Rust program that:

1. Declares an external function with the C ABI: `our_code_starts_here() -> i64`.
2. Links to the compiled code (the `our_code` library produced from our assembly).
3. Calls that function and prints the result.

`extern "C"` means use the C calling convention (System V on Linux/macOS x86-64). The `#[link_name = "\x01our_code_starts_here"]` ensures the linker sees the exact symbol name. The call is `unsafe` because Rust cannot check the behavior of the external assembly.

### 7. Makefile

- **`test/%.s: test/%.snek src/main.rs`** — from a `.snek` file, run the compiler to produce `test/<name>.s`. The compiler is `cargo run -- $< test/$*.s`.
- **`test/%.run: test/%.s runtime/start.rs`** — from a `.s` file:
  1. `nasm -f macho64` assembles to `runtime/our_code.o`.
  2. `ar rcs runtime/libour_code.a runtime/our_code.o` builds a static library.
  3. `rustc -L runtime/ runtime/start.rs -o test/$*.run` links the runtime with that library to produce the executable.

`$<` is the first prerequisite (e.g. `test/37.snek`), `$*` is the stem (e.g. `37`). On Linux you would use `-f elf64` instead of `-f macho64`.

### 8. Calling convention (System V AMD64 ABI)

For a function that returns a single integer:

- **Arguments** (we have none) would be passed in `rdi`, `rsi`, etc.
- **Return value** is in `rax`.
- Our generated code puts the expression result in `rax` and executes `ret`, so the runtime correctly receives the value as the return value of `our_code_starts_here()`.

---

## Test Files

| File | Source | Expected output |
|------|--------|-----------------|
| `test/37.snek` | `37` | 37 |
| `test/zero.snek` | `0` | 0 |
| `test/negative.snek` | `-5` | -5 |
| `test/add.snek` | `(add1 (add1 5))` | 7 |
| `test/sub.snek` | `(sub1 10)` | 9 |
| `test/negate.snek` | `(negate (add1 3))` | -4 |
| `test/complex.snek` | `(sub1 (sub1 (add1 73)))` | 72 |
| `test/add_sub.snek` | `(add1 (sub1 5))` | 5 |
| `test/negate_negate.snek` | `(negate (negate 7))` | 7 |
| `test/nested.snek` | `(negate (add1 (sub1 (add1 10))))` | -11 |

---

## Generating the transcript

The assignment asks for a `transcript.txt` showing the compiler working on several examples. To generate it:

```bash
bash transcript_gen.sh
```

This runs each `test/*.snek` through the pipeline and appends to `transcript.txt`:

- Source code
- Make output (compile + assemble + link)
- Generated assembly
- Program output

You can then submit `transcript.txt` as part of your deliverables.

---

## Project layout

```
adder/
├── Cargo.toml          # Rust project and dependencies (sexp)
├── Makefile             # Rules to build .s from .snek and .run from .s
├── README.md            # This file
├── transcript_gen.sh    # Script to generate transcript.txt
├── runtime/
│   └── start.rs         # Runtime: calls compiled code, prints result
├── src/
│   └── main.rs          # Compiler: parse, AST, codegen, emit .s
└── test/
    ├── *.snek           # Adder source files
    ├── *.s              # Generated assembly (after make)
    └── *.run            # Executables (after make)
```

---

## Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [x86-64 quick reference (Stanford CS107)](https://web.stanford.edu/class/archive/cs/cs107/cs107.1196/guide/x86-64.html)
- [NASM documentation](https://www.nasm.us/xdoc/2.15.05/html/nasmdoc0.html)
- [sexp crate](https://crates.io/crates/sexp)
