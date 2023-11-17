use num_cpus;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use threadpool::ThreadPool;

mod backend;
mod parser;
mod optimizer;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args[0].clone();

    if args.len() % 2 == 0 {
        println!("For each input file, there needs to be an output.");
        std::process::exit(1);
    }

    if args.len() == 1 {
        println!("{} <input file> <output file> <input file 2> <output file 2>... - Compile Brainfuck input files into executable output files.", cmd);
        println!("{} - Print this help page", cmd);
        return;
    }

    let mut pairs = vec![];
    let l = (args.len() - 1) / 2;

    for i in 0..l {
        let input = args[i * 2 + 1].clone();
        let output = args[i * 2 + 2].clone();

        pairs.push((input, output));
    }

    let cpus = num_cpus::get();
    let pool = ThreadPool::new(cpus);

    use std::sync::{Arc, Barrier};

    let barrier = Arc::new(Barrier::new(pairs.len() + 1));

    for (input, output) in pairs.iter() {
        println!("{} -> {}", input, output);
        let input = input.clone();
        let output = output.clone();
        let barrier = barrier.clone();

        pool.execute(move || {
            let mut f = OpenOptions::new()
                .read(true)
                .open(input.clone())
                .expect(&format!("Unable to open file {}", input));
            let mut code = String::new();
            f.read_to_string(&mut code)
                .expect(&format!("Unable to read file {}", input));

            let tokens = parser::tokens(&code);
            let parsed = parser::parse(&tokens);
            let optimized = optimizer::optimize(&parsed);

            let binary = backend::compile(&optimized);

            let mut out = OpenOptions::new()
                .write(true)
                .create(true)
                .open(output.clone())
                .expect(&format!("Unable to open file {}", output));

            out.write(binary.as_slice()).expect(&format!(
                "Could not write result of compilation into {}",
                output
            ));

            barrier.wait();
        });
    }

    barrier.wait();
}
