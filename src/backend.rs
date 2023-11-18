use crate::parser::Node;

use cranelift_codegen::entity::EntityRef;
use cranelift_codegen::ir::{
    condcodes::IntCC, InstBuilder, MemFlags, StackSlotData, StackSlotKind, UserFuncName,
};
use cranelift_codegen::ir::{types::*, AbiParam, Endianness};
use cranelift_codegen::isa::CallConv;
use cranelift_codegen::Context;
use cranelift_codegen::{
    settings::{self, Configurable},
    verify_function,
};

use cranelift_frontend::FunctionBuilder;
use cranelift_frontend::FunctionBuilderContext;
use cranelift_frontend::Variable;
use cranelift_module::{FuncId, Linkage, Module};
use cranelift_object::{ObjectBuilder, ObjectModule};
use target_lexicon::{triple, BinaryFormat};

pub fn compile(nodes: &[Node]) -> Vec<u8> {
    let mut build_flags = settings::builder();
    build_flags.set("opt_level", "speed_and_size").unwrap();
    build_flags.enable("enable_alias_analysis").unwrap();

    let flags = settings::Flags::new(build_flags);

    let mut target = triple!("x86_64");
    target.binary_format = BinaryFormat::Elf;

    let isa_builder = cranelift_codegen::isa::lookup(target).unwrap();
    let isa = isa_builder.finish(flags).unwrap();

    let builder =
        ObjectBuilder::new(isa, "program", cranelift_module::default_libcall_names()).unwrap();

    let mut module = ObjectModule::new(builder);

    let mut getc_sig = module.make_signature();
    getc_sig.returns.push(AbiParam::new(I8));
    getc_sig.call_conv = CallConv::SystemV;

    let mut putc_sig = module.make_signature();
    putc_sig.params.push(AbiParam::new(I8));
    putc_sig.returns.push(AbiParam::new(I8));
    putc_sig.call_conv = CallConv::SystemV;

    let mut memset_sig = module.make_signature();
    memset_sig.params.push(AbiParam::new(R64));
    memset_sig.params.push(AbiParam::new(I32));
    memset_sig.params.push(AbiParam::new(I64));
    memset_sig.returns.push(AbiParam::new(R64));
    memset_sig.call_conv = CallConv::SystemV;

    let getc = module
        .declare_function("getchar", Linkage::Import, &getc_sig)
        .unwrap();

    let putc = module
        .declare_function("putchar", Linkage::Import, &putc_sig)
        .unwrap();

    let memset = module
        .declare_function("memset", Linkage::Import, &memset_sig)
        .unwrap();

    let main = module
        .declare_function("main", Linkage::Export, &module.make_signature())
        .unwrap();

    let mut ctx = module.make_context();

    compile_nodes(nodes, putc, getc, memset, &mut module, main, &mut ctx);

    module.define_function(main, &mut ctx).unwrap();

    module.finish().emit().unwrap()
}

pub fn compile_nodes(
    nodes: &[Node],
    putc: FuncId,
    getc: FuncId,
    memset: FuncId,
    module: &mut impl Module,
    exported_func_id: FuncId,
    ctx: &mut Context,
) {
    let mut sig = module.make_signature();
    sig.returns.push(AbiParam::new(I32));
    sig.call_conv = CallConv::SystemV;
    let mut fbctx = FunctionBuilderContext::new();
    ctx.func.signature = sig;
    ctx.func.name = UserFuncName::user(0, exported_func_id.as_u32());

    let mut builder = FunctionBuilder::new(&mut ctx.func, &mut fbctx);
    let memory_size = 1 * 1024; // 4kb is probably enough

    let getc = module.declare_func_in_func(getc, &mut builder.func);
    let putc = module.declare_func_in_func(putc, &mut builder.func);
    let memset = module.declare_func_in_func(memset, &mut builder.func);

    let memory = builder
        .create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, memory_size));

    let ptr = Variable::new(0);
    builder.declare_var(ptr, R64);

    let loop_blocks = {
        let mut v = vec![];

        for node in nodes {
            match node {
                // Start of loop, inside loop, end of loop, out of loop
                Node::BeginLoop(id) => {
                    let blocks = (
                        builder.create_block(),
                        builder.create_block(),
                        builder.create_block(),
                        builder.create_block(),
                    );

                    if (*id as usize) < v.len() {
                        v[*id as usize] = blocks;
                    } else {
                        let i = *id as usize;
                        // Just spam the blocks
                        while i >= v.len() {
                            v.push(blocks);
                        }
                    }
                }
                _ => {}
            }
        }

        v
    };

    let main_block = builder.create_block();
    builder.switch_to_block(main_block); // The main block is just the random stuff
    builder.seal_block(main_block);

    let memory_address = builder.ins().stack_addr(R64, memory, 0);
    builder.def_var(ptr, memory_address);

    let flags = MemFlags::trusted(); // we trust that the developer did not corrupt the
                                     // memory

    let mut bitcast_flags = MemFlags::new();
    bitcast_flags.set_endianness(Endianness::Big);

    // A memset
    // Memory must be zero'd
    let zero = builder.ins().iconst(I32, 0);
    let memsize = builder.ins().iconst(I64, memory_size as i64);
    builder.ins().call(memset, &[memory_address, zero, memsize]);

    for node in nodes {
        match node {
            Node::Read => {
                let p = builder.use_var(ptr);
                let call = builder.ins().call(getc, &[]);
                let res = builder.inst_results(call);
                let c = res[0];
                builder.ins().store(flags, c, p, 0);
            }
            Node::Print => {
                let p = builder.use_var(ptr);
                let m = builder.ins().load(I8, flags, p, 0);
                _ = builder.ins().call(putc, &[m]);
            }
            Node::Add(n) => {
                let x = builder.ins().iconst(I8, *n as i64);
                let ptr = builder.use_var(ptr);
                let m = builder.ins().load(I8, flags, ptr, 0);
                let r = builder.ins().iadd(x, m);
                builder.ins().store(flags, r, ptr, 0);

                // Effectively:
                // *ptr = *ptr + n
            }
            Node::ShiftLeft(n) => {
                let p = builder.use_var(ptr);
                let p = builder.ins().bitcast(I64, bitcast_flags, p);
                let x = builder.ins().iconst(I64, *n as i64);
                let r = builder.ins().isub(p, x);
                let r = builder.ins().bitcast(R64, bitcast_flags, r);
                builder.def_var(ptr, r);
            }
            Node::ShiftRight(n) => {
                let p = builder.use_var(ptr);
                let p = builder.ins().bitcast(I64, bitcast_flags, p);
                let x = builder.ins().iconst(I64, *n as i64);
                let r = builder.ins().iadd(p, x);
                let r = builder.ins().bitcast(R64, bitcast_flags, r);
                builder.def_var(ptr, r);
            }
            Node::BeginLoop(n) => {
                let id = *n as usize;
                let (begin, body, _, after) = loop_blocks[id];
                builder.ins().jump(begin, &[]);
                builder.switch_to_block(begin);
                let p = builder.use_var(ptr);
                let x = builder.ins().load(I8, flags, p, 0);
                let zero = builder.ins().iconst(I8, 0);
                let c = builder.ins().icmp(IntCC::Equal, x, zero);
                builder.ins().brif(c, after, &[], body, &[]);

                builder.switch_to_block(body);
            }
            Node::EndLoop(n) => {
                let id = *n as usize;
                let (_, body, end, after) = loop_blocks[id];
                builder.ins().jump(end, &[]);
                builder.switch_to_block(end);
                let p = builder.use_var(ptr);
                let x = builder.ins().load(I8, flags, p, 0);
                let zero = builder.ins().iconst(I8, 0);
                let c = builder.ins().icmp(IntCC::NotEqual, x, zero);
                builder.ins().brif(c, body, &[], after, &[]);

                builder.switch_to_block(after);
            }
        }
    }

    builder.seal_all_blocks(); // TODO: seal blocks manually

    // Finish compilation
    let status_code = builder.ins().iconst(I32, 0);
    builder.ins().return_(&[status_code]);
    builder.finalize();

    let mut build_flags = settings::builder();
    build_flags.set("opt_level", "speed_and_size").unwrap();

    let flags = settings::Flags::new(build_flags);
    let res = verify_function(&ctx.func, &flags);

    if let Err(errors) = res {
        panic!("{}", errors);
    }
}
