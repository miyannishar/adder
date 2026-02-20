// =============================================================================
// Adder Runtime — runtime/start.rs
// =============================================================================
// PLP §8.2 (Calling Conventions) & §3.7 (Separate Compilation):
//
// This runtime is the "main" program that calls into the compiled assembly.
// It demonstrates several PLP concepts:
//
// 1. FOREIGN FUNCTION INTERFACE (FFI):
//    The `extern "C"` block declares a function defined in another compilation
//    unit (the generated assembly). PLP §3.7 discusses how separate compilation
//    requires symbols to be resolved at *link time*.
//
// 2. CALLING CONVENTION (PLP §8.2):
//    `extern "C"` specifies the C calling convention (System V AMD64 ABI on
//    Linux/macOS). This means:
//      - Return value is passed in register rax
//      - The `ret` instruction in our assembly returns control here
//
// 3. LINK-TIME BINDING (PLP §3.1):
//    The symbol `our_code_starts_here` is bound to a code address at link time.
//    At compile time, the Rust compiler only knows the function's *signature*;
//    the linker fills in the actual address from the assembled object file.
//
// 4. RUNTIME SYSTEM (PLP §1.4):
//    PLP notes that every compiled program needs a runtime system to bootstrap
//    execution. This file serves that role: it calls the compiled code and
//    handles I/O (printing the result).
// =============================================================================

#[link(name = "our_code")]
extern "C" {
    // The \x01 prefix is a name-mangling escape for the linker symbol.
    // This ensures the linker sees exactly "our_code_starts_here" without
    // any Rust name decoration.
    #[link_name = "\x01our_code_starts_here"]
    fn our_code_starts_here() -> i64;
}

fn main() {
    // `unsafe` is required because Rust cannot verify the safety of external
    // code. The assembly we call is correct by construction (our compiler
    // guarantees rax holds a valid i64 before `ret`).
    let i: i64 = unsafe { our_code_starts_here() };
    println!("{i}");
}