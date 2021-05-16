#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq)]
/// Data that can be handled by this VM
pub enum Value {
    Str(&'static str),
    Int(i64),
    Pid(u64),
}

pub struct Message {
    pub name: &'static str,
    pub value: Value,
}
