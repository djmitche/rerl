#![allow(dead_code)]

use crate::data::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// A program instruction
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

    /// Spawn a new process running the named function, consuming its arguments from the stack, and
    /// push the pid on the stack
    Spawn(&'static str),

    // TODO: include a list of allowed message names; what to do with the others?
    /// Get a message and put its name and value on the stack (TODO)
    Receive,

    /// Send a message with the given name; stack should contain [pid, value].
    Send(&'static str),
}

/// A collection of instructions that can be executed.  When begun, the stack
/// will contain `arg_count` values, and can grow to a maximum of `stack_size`
/// values.
#[derive(Debug, Clone)]
pub struct Function(Arc<FunctionInner>);

impl Function {
    pub fn new(arg_count: usize, stack_size: usize, instructions: Vec<Instruction>) -> Self {
        // TODO: calculate stack size (and ensure it's constant)
        Self(Arc::new(FunctionInner {
            arg_count,
            stack_size,
            instructions,
        }))
    }
}

#[derive(Debug)]
struct FunctionInner {
    pub arg_count: usize,
    pub stack_size: usize,
    pub instructions: Vec<Instruction>,
}

impl Function {
    pub fn arg_count(&self) -> usize {
        self.0.arg_count
    }

    pub fn stack_size(&self) -> usize {
        self.0.stack_size
    }

    pub fn instructions(&self) -> &[Instruction] {
        &self.0.instructions
    }
}

/// A module represents a set of named functions.
#[derive(Default)]
pub struct Module {
    pub functions: HashMap<String, Function>,
}

impl Module {
    /// Add a new function to this module.
    pub fn add_function<S: Into<String>>(&mut self, name: S, function: Function) {
        self.functions.insert(name.into(), function);
    }

    pub fn get_function<S: AsRef<str>>(&self, name: S) -> Option<Function> {
        self.functions.get(name.as_ref()).map(|f| f.clone())
    }
}
