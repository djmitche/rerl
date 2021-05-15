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
}

/// A collection of instructions that can be executed.  When begun, the stack
/// will contain `arg_count` values, and can grow to a maximum of `stack_size`
/// values.
#[derive(Debug)]
pub struct Function {
    pub arg_count: usize,
    pub stack_size: usize,
    pub instructions: Vec<Instruction>,
}

/// A module represents a set of named functions.
#[derive(Default)]
pub struct Module {
    pub functions: HashMap<String, Arc<Function>>,
}

impl Module {
    /// Add a new function to this module.
    pub fn add_function<S: Into<String>>(&mut self, name: S, function: Function) {
        self.functions.insert(name.into(), Arc::new(function));
    }
}
