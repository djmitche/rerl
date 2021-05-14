//! Stack-based VM

use std::collections::HashMap;

// program

#[derive(Debug)]
pub enum Instruction {
    /// Print the top-most item on the stack
    Print,

    /// Push a literal value
    PushLiteral(Value),

    /// Duplicate the i'th item from the top of the stack
    Dup(usize),

    /// Discard the top item on the stack
    Pop,

    /// Swap the top item on the stack with the i'th item from the top of the stack
    Swap(usize),

    /// Jump, unconditionally, to an instruction in this function
    Jump(usize),

    /// Pop the value off the stack and jump if it is equal to the given value
    JumpIfEqual(usize, Value),

    /// Call another function, popping its arg_size arguments from the stack
    Call(&'static str),

    /// Return to the calling function, pushing this function's stack onto
    /// the caller's stack.
    Return,

    /// Add the top two values on the stack, leaving the sum
    Add,

    /// Multiply the top two values on the stack, leaving the product
    Mul,
}

#[derive(Debug)]
pub struct Function {
    pub arg_count: usize,
    pub stack_size: usize,
    pub instructions: Vec<Instruction>,
}

#[derive(Default)]
pub struct Module {
    pub functions: HashMap<String, Function>,
}

// data

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Str(&'static str),
    Int(i64),
}

// execution

#[derive(Debug)]
struct ExecutionContext<'pr> {
    pub parent: Option<Box<ExecutionContext<'pr>>>,
    pub function: &'pr Function,
    pub stack: Vec<Value>,
    pub instruction: usize,
}

pub struct Execution<'pr> {
    module: &'pr Module,
    call_stack: Option<Box<ExecutionContext<'pr>>>,
}

impl<'pr> Execution<'pr> {
    pub fn new(module: &'pr Module) -> Execution {
        let function = module
            .functions
            .get("init")
            .expect("no function named 'init'");
        debug_assert_eq!(function.arg_count, 0, "'init' must take no arguments");
        let context = Box::new(ExecutionContext {
            parent: None,
            function,
            stack: Vec::with_capacity(function.stack_size),
            instruction: 0,
        });
        Self {
            module,
            call_stack: Some(context),
        }
    }

    pub fn run(&mut self) {
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
                    let function = self.module.functions.get(name).expect("Unknown function");
                    debug_assert!(ctxt.stack.len() >= function.arg_count);
                    let mut stack = ctxt.stack.split_off(ctxt.stack.len() - function.arg_count);
                    stack.reserve(function.stack_size);
                    self.call_stack = Some(Box::new(ExecutionContext {
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
