# Adder

I built a small compiler for the **Adder** language: 32-bit integers and three unary ops (`add1`, `sub1`, `negate`). It goes from source text to x86-64 assembly and a runnable executable. I did this project on my **MacBook Pro with M2 Pro chip** (Apple Silicon).

---

## The language

I designed the language as S-expressions. Valid forms:

- A **number** (32-bit signed integer) — evaluates to itself.
- `(add1 <expr>)` — evaluate the expr, add 1.
- `(sub1 <expr>)` — evaluate the expr, subtract 1.
- `(negate <expr>)` — evaluate the expr, multiply by -1.

Examples I used: `37` → 37, `(add1 (sub1 5))` → 5, `(negate (add1 3))` → -4.

---

## How the compiler works

I used the **sexp** crate to turn source into an S-expression tree (it does scanning and parsing to s-exps; I didn’t write a lexer/parser for the surface syntax). Then I wrote a **recursive-descent parser** over that tree to build an **AST**: an `Expr` enum with `Num(i32)`, `Add1(Box<Expr>)`, `Sub1(Box<Expr>)`, and `Negate(Box<Expr>)`. I used `Box<Expr>` for recursive variants so the type has a fixed size.

I implemented the **code generator** as a single post-order walk over the AST. Every subexpression compiles so its result ends up in **rax**; the parent then emits one or more instructions using that value. So: `Num(n)` → `mov rax, n`; `Add1(e)` → code for `e` then `add rax, 1`; similarly for `sub1` and `negate`. I followed the System V AMD64 convention (return value in `rax`). For `(negate (add1 3))` the code I generate looks like:

```asm
  mov rax, 3
  add rax, 1
  neg rax
  ret
```

My compiler writes a `.s` file with a function `our_code_starts_here`. The **Makefile** I wrote assembles it with NASM (`macho64` on macOS), archives it into a static lib, and **rustc** links my **runtime** (`runtime/start.rs`) against that lib. The runtime declares `our_code_starts_here` as `extern "C"`, calls it, and prints the result. So the full pipeline I set up is: `.snek` → compiler → `.s` → NASM → `.o` → `ar` → `libour_code.a` → rustc with runtime → `.run`.

---

## Setup (on my M2 Pro MacBook)

I have macOS on Apple Silicon (M2 Pro). I installed **Rust**, **NASM** (`brew install nasm`), and **GCC/Clang** (Xcode). The compiler emits x86-64 assembly, so I added the x86-64 target—the `.run` binaries then run under Rosetta 2:

```bash
rustup target add x86_64-apple-darwin
```

**Build:** `cargo build`

**Run one test:**
```bash
make test/37.run
./test/37.run
```

**Run all tests and transcript:** `bash transcript_gen.sh`

**Clean:** `make clean`

---

## Project layout

- **src/main.rs** — my compiler: reads `.snek`, uses sexp + `parse_expr()` to get AST, `compile_expr()` to get assembly, writes `.s`.
- **runtime/start.rs** — my runtime: links to the compiled code, calls `our_code_starts_here()`, prints the return value.
- **Makefile** — what I use to build `.s` from `.snek` and `.run` from `.s` (nasm, ar, rustc).
- **test/*.snek** — Adder source I wrote; `make test/<name>.run` produces `test/<name>.s` and `test/<name>.run`.

More detail is in [docs/CONCEPTS.md](docs/CONCEPTS.md) and [docs/RUN_FLOW.md](docs/RUN_FLOW.md).
