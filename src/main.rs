mod data;
mod program;
mod vm;

use crate::data::Value;
use crate::program::{Function, Instruction, Module};
use crate::vm::VM;

#[tokio::main]
async fn main() {
    use Instruction::*;

    let mut module = Module::default();

    module.add_function(
        "init",
        Function::new(
            0,
            3,
            vec![
                PushLiteral(Value::Int(20)),
                Spawn("fibproc"),
                Pop, // ignore the pid
                PushLiteral(Value::Int(10)),
                Spawn("fibproc"),
                Pop, // ignore the pid
                PushLiteral(Value::Int(2)),
                Receive,      // get an answer
                Call("show"), // show its value
                Pop,          // pop the name
                PushLiteral(Value::Int(-1)),
                Add,
                Dup(0),
                JumpIfEqual(15, Value::Int(0)),
                Jump(7),
                Pop,
                Return,
            ],
        ),
    );
    module.add_function(
        "fibproc",
        Function::new(
            1,
            3,
            vec![
                Call("fib"),
                PushLiteral(Value::Pid(0)),
                Swap(1),
                Send("result"),
                Return,
            ],
        ),
    );
    module.add_function("show", Function::new(1, 1, vec![Print, Return]));
    module.add_function(
        "fib",
        Function::new(
            1,
            5,
            vec![
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
        ),
    );

    let vm = VM::new(module);
    vm.run().await;
}
