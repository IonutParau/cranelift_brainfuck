# Cranelift Brainfuck Compiler

A compiler for the Brainfuck programming language, written in Rust that uses Cranelift. It performs basic optimizations.

FYI: this is not an official Cranelift example, this is just me messing around with compilers and code generation.

# How to use it

`cranelift_bfc <input file> <output file> <input file 2> <output file 2>...`. Yes, you can compile multiple files from one command in parallel. They aren't fused, they just all get compiled separately, on multiple threads.

Important Note: It compiles to *object files*, not executables. In the future it may generate executables.

# Dependencies

The object files generated depend on libc. It needs `putchar`, `getchar` and `memset`.

# Platforms supported

Currently, the compiler is hard-coded to compile to x64 linux as an ELF. Soon enough, it will ask for a target using a flag.

