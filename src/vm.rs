//! Stack-based VM

use crate::data::Value;
use crate::program::{Function, Instruction, Module};
use std::sync::{Arc, Mutex};

// process

#[derive(Debug)]
struct ProcessContext {
    pub parent: Option<Box<ProcessContext>>,
    pub function: Arc<Function>,
    pub stack: Vec<Value>,
    pub instruction: usize,
}

pub struct Process {
    vm: VM,
    call_stack: Option<Box<ProcessContext>>,
}

impl Process {
    async fn run(mut self) {
        use Instruction::*;
        loop {
            let ctxt = &mut self.call_stack.as_mut().expect("call stack is empty");
            let instr = &ctxt.function.instructions[ctxt.instruction];
            println!("Execute {:?} with stack {:?}", instr, ctxt.stack);
            ctxt.instruction += 1;
            match *instr {
                Print => {
                    debug_assert!(!ctxt.stack.is_empty());
                    let v = ctxt.stack.pop().unwrap();
                    println!("{:?}", v);
                }
                PushLiteral(ref v) => {
                    debug_assert!(ctxt.stack.len() < ctxt.function.stack_size);
                    ctxt.stack.push((*v).clone());
                }
                Pop => {
                    debug_assert!(!ctxt.stack.is_empty());
                    ctxt.stack.pop().unwrap();
                }
                Swap(i) => {
                    debug_assert!(ctxt.stack.len() > i);
                    if i != 0 {
                        let l = ctxt.stack.len();
                        let t = ctxt.stack.pop().unwrap();
                        ctxt.stack.push(ctxt.stack[l - i - 1].clone());
                        ctxt.stack[l - i - 1] = t;
                    }
                }
                Dup(i) => {
                    debug_assert!(ctxt.stack.len() > i);
                    ctxt.stack
                        .push(ctxt.stack[ctxt.stack.len() - i - 1].clone());
                }
                Jump(dest) => ctxt.instruction = dest,
                JumpIfEqual(dest, ref cond) => {
                    debug_assert!(!ctxt.stack.is_empty());
                    let v = ctxt.stack.pop().unwrap();
                    if cond == &v {
                        ctxt.instruction = dest;
                    }
                }
                Call(name) => {
                    let function = self
                        .vm
                        .0
                        .lock()
                        .unwrap()
                        .module
                        .functions
                        .get(name)
                        .expect("Unknown function")
                        .clone();
                    debug_assert!(ctxt.stack.len() >= function.arg_count);
                    let mut stack = ctxt.stack.split_off(ctxt.stack.len() - function.arg_count);
                    stack.reserve(function.stack_size);
                    self.call_stack = Some(Box::new(ProcessContext {
                        parent: self.call_stack.take(),
                        function,
                        stack,
                        instruction: 0,
                    }));
                }
                Return => {
                    let ctxt = self.call_stack.take().expect("no current context");
                    let mut parent = if let Some(parent) = ctxt.parent {
                        parent
                    } else {
                        // return from top-level function -> run is finished
                        return;
                    };
                    let mut rv = ctxt.stack;
                    debug_assert!(parent.stack.len() + rv.len() <= parent.function.stack_size);
                    parent.stack.append(&mut rv);
                    self.call_stack = Some(parent);
                }
                Add => {
                    debug_assert!(ctxt.stack.len() >= 2);
                    let a = ctxt.stack.pop().unwrap();
                    let b = ctxt.stack.pop().unwrap();
                    ctxt.stack.push(match (a, b) {
                        (Value::Int(a), Value::Int(b)) => Value::Int(a + b),
                        _ => panic!("Add only supports ints"),
                    })
                }
                Mul => {
                    debug_assert!(ctxt.stack.len() >= 2);
                    let a = ctxt.stack.pop().unwrap();
                    let b = ctxt.stack.pop().unwrap();
                    ctxt.stack.push(match (a, b) {
                        (Value::Int(a), Value::Int(b)) => Value::Int(a * b),
                        _ => panic!("Mul only supports ints"),
                    })
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct VM(Arc<Mutex<VMInner>>);

pub struct VMInner {
    module: Module,
    //next_pid: u64,
}

impl VM {
    pub fn new(module: Module) -> VM {
        VM(Arc::new(Mutex::new(VMInner {
            module,
            //next_pid: 1,
        })))
    }

    pub async fn run(&mut self) {
        let function = self
            .0
            .lock()
            .unwrap()
            .module
            .functions
            .get("init")
            .expect("no function named 'init'")
            .clone();
        debug_assert_eq!(function.arg_count, 0, "'init' must take no arguments");

        let context = Box::new(ProcessContext {
            parent: None,
            function: function.clone(),
            stack: Vec::with_capacity(function.stack_size),
            instruction: 0,
        });

        let process = Process {
            vm: self.clone(),
            call_stack: Some(context),
        };
        let jh = tokio::spawn(process.run());
        jh.await.unwrap();
    }
}
