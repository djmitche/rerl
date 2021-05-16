//! Stack-based VM

use crate::data::{Message, Value};
use crate::program::{Function, Instruction, Module};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{
    mpsc::{channel, Receiver, Sender},
    Notify,
};

#[derive(Clone)]
pub struct VM(Arc<Mutex<VMInner>>);

pub struct VMInner {
    module: Module,

    /// next un-assigned pid
    next_pid: u64,

    /// channels to communicate with each process
    channels: HashMap<u64, Sender<Message>>,

    /// signalled when a process has exited
    process_exited: Arc<Notify>,
}

impl VM {
    pub fn new(module: Module) -> VM {
        VM(Arc::new(Mutex::new(VMInner {
            module,
            next_pid: 0,
            channels: HashMap::new(),
            process_exited: Arc::new(Notify::new()),
        })))
    }

    fn spawn_process<S: Into<String>>(&self, function_name: S, args: Vec<Value>) -> u64 {
        let receiver;
        let pid;
        {
            let mut inner = self.0.lock().unwrap();
            pid = inner.next_pid;
            inner.next_pid += 1;

            // TODO: what size should this be?
            let chan = channel(10);
            inner.channels.insert(pid, chan.0);
            receiver = chan.1;
        }
        tokio::spawn(
            self.clone()
                .run_process(function_name.into(), args, pid, receiver),
        );
        pid
    }

    async fn run_process(
        self,
        function_name: String,
        args: Vec<Value>,
        pid: u64,
        mut receiver: Receiver<Message>,
    ) {
        use Instruction::*;

        let mut function;
        {
            let inner = self.0.lock().unwrap();
            function = inner
                .module
                .get_function(function_name)
                .expect("no function with that name");
        }

        // the call stack contains all but the current frame, which is
        // broken out as local variables
        struct Frame {
            function: Function,
            next_instr: usize,
            stack: Vec<Value>,
        }
        let mut frame_stack = vec![];

        // ensure we got the number of args expected
        debug_assert_eq!(function.arg_count(), args.len());

        let mut instructions = function.instructions();
        let mut next_instr = 0_usize;
        let mut stack_size = function.stack_size();
        let mut stack = args;
        stack.reserve(stack_size);

        // TODO: tokio::task::yield_now().await sometimes?
        loop {
            let instr = &instructions[next_instr];
            println!("[{}] Execute {:?} with stack {:?}", pid, instr, stack);
            next_instr += 1;
            match *instr {
                Print => {
                    debug_assert!(!stack.is_empty());
                    let v = stack.pop().unwrap();
                    println!("{:?}", v);
                }
                PushLiteral(ref v) => {
                    debug_assert!(stack.len() < stack_size);
                    stack.push((*v).clone());
                }
                Pop => {
                    debug_assert!(!stack.is_empty());
                    stack.pop().unwrap();
                }
                Swap(i) => {
                    debug_assert!(stack.len() > i);
                    if i != 0 {
                        let l = stack.len();
                        let t = stack.pop().unwrap();
                        stack.push(stack[l - i - 1].clone());
                        stack[l - i - 1] = t;
                    }
                }
                Dup(i) => {
                    debug_assert!(stack.len() > i);
                    stack.push(stack[stack.len() - i - 1].clone());
                }
                Jump(dest) => next_instr = dest,
                JumpIfEqual(dest, ref cond) => {
                    debug_assert!(!stack.is_empty());
                    let v = stack.pop().unwrap();
                    if cond == &v {
                        next_instr = dest;
                    }
                }
                Call(name) => {
                    let child_function;
                    {
                        let inner = self.0.lock().unwrap();
                        child_function = inner
                            .module
                            .get_function(name)
                            .expect("no function with that name");
                    }
                    let arg_count = child_function.arg_count();
                    debug_assert!(stack.len() >= arg_count);
                    let child_stack = stack.split_off(stack.len() - arg_count);

                    // push the current frame onto the stack
                    frame_stack.push(Frame {
                        function,
                        next_instr,
                        stack,
                    });

                    // and set the local variables to point to the new child frame
                    function = child_function;
                    instructions = function.instructions();
                    stack_size = function.stack_size();
                    next_instr = 0;
                    stack = child_stack;
                }
                Return => {
                    let mut parent = if let Some(frame) = frame_stack.pop() {
                        frame
                    } else {
                        // return from top-level frame exits the process
                        debug_assert_eq!(stack.len(), 0);
                        {
                            let mut inner = self.0.lock().unwrap();
                            inner.channels.remove(&pid);
                            inner.process_exited.notify_one();
                        }
                        return;
                    };
                    parent.stack.append(&mut stack);

                    function = parent.function;
                    instructions = function.instructions();
                    stack_size = function.stack_size();
                    next_instr = parent.next_instr;
                    stack = parent.stack;
                }
                Add => {
                    debug_assert!(stack.len() >= 2);
                    let a = stack.pop().unwrap();
                    let b = stack.pop().unwrap();
                    stack.push(match (a, b) {
                        (Value::Int(a), Value::Int(b)) => Value::Int(a + b),
                        _ => panic!("Add only supports ints"),
                    })
                }
                Mul => {
                    debug_assert!(stack.len() >= 2);
                    let a = stack.pop().unwrap();
                    let b = stack.pop().unwrap();
                    stack.push(match (a, b) {
                        (Value::Int(a), Value::Int(b)) => Value::Int(a * b),
                        _ => panic!("Mul only supports ints"),
                    })
                }
                Spawn(name) => {
                    let arg_count = {
                        let inner = self.0.lock().unwrap();
                        let child_function = inner
                            .module
                            .get_function(name)
                            .expect("no function with that name");
                        child_function.arg_count()
                    };
                    debug_assert!(stack.len() >= arg_count);
                    let child_stack = stack.split_off(stack.len() - arg_count);
                    let pid = self.spawn_process(name, child_stack);
                    stack.push(Value::Pid(pid));
                }
                Receive => {
                    let msg = receiver.recv().await.expect("channel closed unexpectedly");
                    stack.push(Value::Str(msg.name));
                    stack.push(msg.value);
                }
                Send(name) => {
                    debug_assert!(stack.len() >= 2);
                    let value = stack.pop().unwrap();
                    let pid = if let Value::Pid(pid) = stack.pop().unwrap() {
                        pid
                    } else {
                        panic!("value on stack is not a Pid")
                    };
                    let message = Message { name, value };

                    let sender = {
                        let inner = self.0.lock().unwrap();
                        inner.channels.get(&pid).expect("no such pid").clone()
                    };
                    if let Err(_) = sender.send(message).await {
                        // XXX can't .unwrap() this
                        panic!("uhoh");
                    }
                }
            }
        }
    }

    pub async fn run(self) {
        let pid = self.spawn_process("init", vec![]);
        println!("started pid {}", pid);

        // loop until there are no processes
        let notify = self.0.lock().unwrap().process_exited.clone();
        while !self.0.lock().unwrap().channels.is_empty() {
            notify.notified().await;
        }
        println!("all processes have ended..");
    }
}
