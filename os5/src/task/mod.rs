//! Implementation of process management mechanism
//!
//! Here is the entry for process scheduling required by other modules
//! (such as syscall or clock interrupt).
//! By suspending or exiting the current process, you can
//! modify the process state, manage the process queue through TASK_MANAGER,
//! and switch the control flow through PROCESSOR.
//!
//! Be careful when you see [`__switch`]. Control flow around this function
//! might not be what you expect.

mod context;
mod manager;
mod pid;
mod processor;
mod switch;
#[allow(clippy::module_inception)]
mod task;

use crate::{loader::get_app_data_by_name, mm::{VirtPageNum, MapPermission, VirtAddr}, console::print};
use alloc::sync::Arc;
use lazy_static::*;
use manager::fetch_task;
use switch::__switch;
pub use task::{TaskControlBlock, TaskStatus};

pub use context::TaskContext;
pub use manager::add_task;
pub use pid::{pid_alloc, KernelStack, PidHandle};
pub use processor::{
    current_task, current_trap_cx, current_user_token, run_tasks, schedule, take_current_task,
};

/// Make current task suspended and switch to the next task
pub fn suspend_current_and_run_next() {
    // There must be an application running.
    let task = take_current_task().unwrap();

    // ---- access current TCB exclusively
    let mut task_inner = task.inner_exclusive_access();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;
    // Change status to Ready
    task_inner.task_status = TaskStatus::Ready;
    drop(task_inner);
    // ---- release current PCB

    // push back to ready queue.
    add_task(task);
    // jump to scheduling cycle
    schedule(task_cx_ptr);
}

/// Exit current task, recycle process resources and switch to the next task
pub fn exit_current_and_run_next(exit_code: i32) {
    // take from Processor
    let task = take_current_task().unwrap();
    // **** access current TCB exclusively
    let mut inner = task.inner_exclusive_access();
    // Change status to Zombie
    inner.task_status = TaskStatus::Zombie;
    // Record exit code
    inner.exit_code = exit_code;
    // do not move to its parent but under initproc

    // ++++++ access initproc TCB exclusively
    {
        let mut initproc_inner = INITPROC.inner_exclusive_access();
        for child in inner.children.iter() {
            child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
            initproc_inner.children.push(child.clone());
        }
    }
    // ++++++ release parent PCB

    inner.children.clear();
    // deallocate user space
    inner.memory_set.recycle_data_pages();
    drop(inner);
    // **** release current PCB
    // drop task manually to maintain rc correctly
    drop(task);
    // we do not have to save task context
    let mut _unused = TaskContext::zero_init();
    schedule(&mut _unused as *mut _);
}

lazy_static! {
    /// Creation of initial process
    ///
    /// the name "initproc" may be changed to any other app name like "usertests",
    /// but we have user_shell, so we don't need to change it.
    pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new(TaskControlBlock::new(
        get_app_data_by_name("ch5b_initproc").unwrap()
    ));
}

pub fn add_initproc() {
    add_task(INITPROC.clone());
}

use crate::config::PAGE_SIZE;
#[allow(dead_code, unused_variables, unused)]
pub fn mmap(start: usize, len: usize, port: usize) -> isize{
    println!("1");
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();

    println!("2");
    let start_va = VirtPageNum::from(start / PAGE_SIZE);
    let end_va = VirtPageNum::from((start+len) / PAGE_SIZE);

    println!("3");
    for vpn in start_va.0..end_va.0 {
        if inner.memory_set.find_vpn(VirtPageNum(vpn)) {
            // println!("{}  {}", start / PAGE_SIZE, (start + len) / PAGE_SIZE);
            println!("there is a overlap!!!");
            return -1;
        }
    }

    println!("4");
    let permission = MapPermission::from_bits(((port << 1) | 16) as u8);
    
    println!("5");
    inner.memory_set.insert_framed_area(VirtAddr::from(start_va), VirtAddr::from(end_va), permission.unwrap());
    // inner.memory_set.insert_framed_area(VirtAddr::from(start), VirtAddr::from(start+len), permission.unwrap());

    println!("6");
    for vpn in start_va.0..end_va.0 {
            if false == inner.memory_set.find_vpn(VirtPageNum(vpn)) {
            return -1;
        }
    }
    println!("7");
    0
}

#[allow(unused)]
pub fn unmmap(start: usize, len: usize) -> isize {
    println!("inside unmmap function !!!") ;
    println!("A1");
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();

    println!("A2");
    let start_va = VirtPageNum::from(start / PAGE_SIZE);
    let end_va = VirtPageNum::from((start+len) / PAGE_SIZE);

    println!("A");
    for vpn in start_va.0..end_va.0 { 
        if !(inner.memory_set.find_vpn(VirtPageNum(vpn))) {
            println!("it seem couldn't find the pte");
            return -1;
        }
    }
    println!("B");
    for vpn in start_va.0..end_va.0 {
        inner.memory_set.delete_pte_from(VirtPageNum(vpn));
    }
    println!("successed!!!");
    0
}

pub fn set_prio(prio: u8) {
    
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();

    inner.piro = prio;
}