// Adder Compiler — src/main.rs

use sexp::*;
use sexp::Atom::*;
use std::env;
use std::fs::File;
use std::io::prelude::*;

// ABSTRACT SYNTAX TREE

enum Expr {
    Num(i32),           // Leaf: integer literal
    Add1(Box<Expr>),    // Unary: increment by 1
    Sub1(Box<Expr>),    // Unary: decrement by 1
    Negate(Box<Expr>),  // Unary: multiply by -1
}

// PARSER

fn parse_expr(s: &Sexp) -> Expr {
    match s {
        // Base case: an integer atom → Num node
        Sexp::Atom(I(n)) => {
            Expr::Num(i32::try_from(*n).expect("Integer overflow: value does not fit in i32"))
        }

        // Recursive case: a list like (op expr)
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

// CODE GENERATOR

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

// MAIN

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

    // Phase 2: Scanning + Parsing
    let sexp = parse(&in_contents).expect("Invalid s-expression syntax");
    let expr = parse_expr(&sexp);

    // Phase 3: Code Generation
    // Post-order traversal of the AST, emitting x86-64 instructions.
    let result = compile_expr(&expr);

    // Phase 4: Emit assembly file
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