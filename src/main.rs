// =============================================================================
// Adder Compiler — src/main.rs
// =============================================================================
// This compiler implements the full pipeline described in PLP Section 1.6:
//   Source Text → Scanner → Parser → AST (IR) → Code Generator → Assembly
//
// PLP Concepts Used:
//   - Ch 2 (Scanning & Parsing): Tokenization, CFGs, recursive descent
//   - Ch 3 (Bindings): Compile-time binding of operations to instructions
//   - Ch 4 (Semantic Analysis): S-attributed grammar, syntax-directed translation
//   - Ch 5 (Target Architecture): x86-64 ISA, register-based computation
//   - Ch 8 (Subroutines): Calling conventions, System V ABI, return via rax
//   - Ch 15 (Code Generation): Accumulator-based single-register strategy
// =============================================================================

use sexp::*;
use sexp::Atom::*;
use std::env;
use std::fs::File;
use std::io::prelude::*;

// =============================================================================
// ABSTRACT SYNTAX TREE (PLP Ch 2 & 4)
// =============================================================================
// PLP distinguishes the *concrete syntax tree* (parse tree) from the *abstract
// syntax tree* (AST). The AST discards syntactic scaffolding like parentheses
// and keywords — their role is captured by the tree structure itself.
//
// This enum is also our sole *Intermediate Representation* (PLP §15.2).
// It is a high-level tree IR, the simplest form PLP describes.
//
// Box<Expr> is required because Rust needs a known size at compile time;
// recursive types must be heap-allocated via indirection.
// =============================================================================
enum Expr {
    Num(i32),           // Leaf: integer literal
    Add1(Box<Expr>),    // Unary: increment by 1
    Sub1(Box<Expr>),    // Unary: decrement by 1
    Negate(Box<Expr>),  // Unary: multiply by -1
}

// =============================================================================
// PARSER (PLP Ch 2: §2.1.2 CFGs, §2.3.1 Recursive Descent)
// =============================================================================
// The Adder grammar is a context-free grammar (CFG) in BNF:
//
//   <expr> ::= <number>
//            | ( add1 <expr> )
//            | ( sub1 <expr> )
//            | ( negate <expr> )
//
// Terminals: <number>, (, ), add1, sub1, negate
// Non-terminals: <expr>   (also the start symbol)
//
// PLP §2.3.1: A recursive descent parser has "a subroutine for every
// nonterminal." Since we have one nonterminal (<expr>), we have one
// parsing function: parse_expr.
//
// This is LL(1) parsing — we need only ONE token of lookahead to decide
// which production to apply:
//   - If we see a number → Num production
//   - If we see '(' → read the keyword to pick Add1/Sub1/Negate
//
// The `sexp` crate handles scanning (PLP §2.2): it tokenizes the input
// into an s-expression tree, implementing the longest-match rule (maximal
// munch) so that "add1" is one token, not "add" + "1".
// =============================================================================
fn parse_expr(s: &Sexp) -> Expr {
    match s {
        // Base case: an integer atom → Num node
        // PLP §2.1.1: The token class is "integer literal", the lexeme is
        // the specific digit string, the pattern is -?[0-9]+
        Sexp::Atom(I(n)) => {
            // Validate 32-bit range (PLP §4: semantic analysis / type checking)
            Expr::Num(i32::try_from(*n).expect("Integer overflow: value does not fit in i32"))
        }

        // Recursive case: a list like (op expr)
        // PLP §2.3.1: inspect the first element (the "lookahead" within the list)
        // to determine which production to use
        Sexp::List(vec) => {
            match &vec[..] {
                // (add1 <expr>)
                [Sexp::Atom(S(op)), e] if op == "add1" => {
                    Expr::Add1(Box::new(parse_expr(e)))
                }
                // (sub1 <expr>)
                [Sexp::Atom(S(op)), e] if op == "sub1" => {
                    Expr::Sub1(Box::new(parse_expr(e)))
                }
                // (negate <expr>)
                [Sexp::Atom(S(op)), e] if op == "negate" => {
                    Expr::Negate(Box::new(parse_expr(e)))
                }
                // No valid production matched → syntax error
                _ => panic!("Invalid expression: unrecognized form"),
            }
        }

        // Catch-all for things like floating-point atoms, strings, etc.
        _ => panic!("Invalid expression: unexpected atom type"),
    }
}

// =============================================================================
// CODE GENERATOR (PLP Ch 4: §4.2–4.3 S-Attributed Grammars, Ch 15: §15.3)
// =============================================================================
// PLP §4.2 defines an *S-attributed grammar* as one where ALL attributes are
// *synthesized* — computed from children and flowing UPWARD in the tree.
// This means evaluation can be done in a single bottom-up (post-order) pass.
//
// Our compile_expr function is exactly this: a post-order traversal where:
//   1. We recursively compile subexpressions (children first)
//   2. Then emit the current node's instruction (parent after children)
//   3. The "synthesized attribute" is the generated assembly string
//
// PLP §15.3: Code generation strategy
// We use an *accumulator-based* approach — the simplest strategy PLP describes
// in its register allocation discussion (§15.3.2). All intermediate values
// live in a single register: rax. Each node's compiled code:
//   - ASSUMES its subexpression's result is already in rax
//   - GUARANTEES its own result will also be in rax
//
// This invariant is the "contract" that makes compositional code generation work.
//
// PLP §3.1 (Binding): Each AST node is *compile-time bound* to a specific
// x86-64 instruction pattern. This early binding yields maximum efficiency
// (native code) at the cost of flexibility.
// =============================================================================
fn compile_expr(e: &Expr) -> String {
    match e {
        // Leaf: load the integer constant into the accumulator register
        // x86-64 instruction: mov rax, <immediate>
        Expr::Num(n) => format!("  mov rax, {}", *n),

        // Add1: compile subexpr (result in rax), then increment
        // x86-64 instruction: add rax, 1
        Expr::Add1(subexpr) => {
            let sub_code = compile_expr(subexpr);
            format!("{}\n  add rax, 1", sub_code)
        }

        // Sub1: compile subexpr (result in rax), then decrement
        // x86-64 instruction: sub rax, 1
        Expr::Sub1(subexpr) => {
            let sub_code = compile_expr(subexpr);
            format!("{}\n  sub rax, 1", sub_code)
        }

        // Negate: compile subexpr (result in rax), then negate
        // x86-64 instruction: neg rax  (two's complement negation)
        Expr::Negate(subexpr) => {
            let sub_code = compile_expr(subexpr);
            format!("{}\n  neg rax", sub_code)
        }
    }
}

// =============================================================================
// MAIN: Orchestrating the Pipeline (PLP §1.6)
// =============================================================================
// PLP §1.6 divides compilation into front end (analysis) and back end (synthesis).
//   Front end: read source → scan → parse → build AST
//   Back end:  traverse AST → emit assembly
//
// The output .s file is then assembled by NASM and linked with the runtime
// to produce an executable — completing the pipeline from PLP §15.6.
// =============================================================================
fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <input.snek> <output.s>", args[0]);
        std::process::exit(1);
    }

    let in_name = &args[1];
    let out_name = &args[2];

    // Phase 1: Read source file
    let mut in_file = File::open(in_name)?;
    let mut in_contents = String::new();
    in_file.read_to_string(&mut in_contents)?;

    // Phase 2: Scanning + Parsing (PLP §2.2 + §2.3)
    // The sexp crate's parse() handles scanning (tokenization) internally.
    // Our parse_expr() then performs recursive descent over the token tree.
    let sexp = parse(&in_contents).expect("Invalid s-expression syntax");
    let expr = parse_expr(&sexp);

    // Phase 3: Code Generation (PLP §15.3)
    // Post-order traversal of the AST, emitting x86-64 instructions.
    let result = compile_expr(&expr);

    // Phase 4: Emit assembly file
    // PLP §8.2 (Calling Conventions): The generated function follows the
    // System V AMD64 ABI. It takes no arguments, computes a value into
    // rax, and returns via `ret`. The caller (our Rust runtime) reads rax
    // to obtain the result.
    //
    // `global` makes the symbol visible to the linker (PLP §15.6, §3.7:
    // separate compilation and link-time binding).
    let asm_program = format!(
        "section .text
global our_code_starts_here
our_code_starts_here:
{}
  ret
",
        result
    );

    let mut out_file = File::create(out_name)?;
    out_file.write_all(asm_program.as_bytes())?;

    Ok(())
}