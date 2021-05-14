mod vm;

use vm::*;

fn main() {
    use Instruction::*;

    let mut module = Module::default();

    module.functions.insert(
        "init".to_owned(),
        Function {
            arg_count: 0,
            stack_size: 3,
            instructions: vec![
                PushLiteral(Value::Int(6)),
                Call("fib"),
                Call("show"),
                Return,
            ],
        },
    );
    module.functions.insert(
        "show".to_owned(),
        Function {
            arg_count: 1,
            stack_size: 1,
            instructions: vec![Print, Return],
        },
    );
    module.functions.insert(
        "fib".to_owned(),
        Function {
            arg_count: 1,
            stack_size: 5,
            instructions: vec![
                Dup(0),
                JumpIfEqual(16, Value::Int(0)),
                Dup(0),
                JumpIfEqual(16, Value::Int(1)),
                // call fib(n-1)
                Dup(0),
                PushLiteral(Value::Int(-1)),
                Add,
                Call("fib"),
                // call fib(n-2)
                Dup(1),
                PushLiteral(Value::Int(-2)),
                Add,
                Call("fib"),
                Add,
                // stack: [n, fib(n-1) + fib(n-2)]
                Swap(1),
                Pop,
                Return,
                // return 1 (base case)
                Pop,
                PushLiteral(Value::Int(1)),
                Return,
            ],
        },
    );

    let mut exec = Execution::new(&module);
    exec.run();
}
