//! Task management implementation
//!
//! Everything about task management, like starting and switching tasks is
//! implemented here.
//!
//! A single global instance of [`TaskManager`] called `TASK_MANAGER` controls
//! all the tasks in the operating system.
//!
//! Be careful when you see `__switch` ASM function in `switch.S`. Control flow around this function
//! might not be what you expect.

mod context;
mod switch;

#[allow(clippy::module_inception)]
mod task;


use crate::config::MAX_APP_NUM;
use crate::loader::{get_num_app, init_app_cx};
use crate::sync::UPSafeCell;
use lazy_static::*;
use switch::__switch;
use task::{TaskControlBlock, TaskStatus};

pub use context::TaskContext;
use crate::timer::get_time;

/// The task manager, where all the tasks are managed.
///
/// Functions implemented on `TaskManager` deals with all task state transitions
/// and task context switching. For convenience, you can find wrappers around it
/// in the module level.
///
/// Most of `TaskManager` are hidden behind the field `inner`, to defer
/// borrowing checks to runtime. You can see examples on how to use `inner` in
/// existing functions on `TaskManager`.
pub struct TaskManager {
    /// total number of tasks
    num_app: usize,
    /// use inner value to get mutable access
    inner: UPSafeCell<TaskManagerInner>,
}

/// Inner of Task Manager
pub struct TaskManagerInner {
    /// task list
    tasks: [TaskControlBlock; MAX_APP_NUM],
    /// id of current `Running` task
    current_task: usize,
}

lazy_static! {
    /// Global variable: TASK_MANAGER
    pub static ref TASK_MANAGER: TaskManager = {
        let num_app = get_num_app();
        /*let mut tasks = [TaskControlBlock {
            task_cx: TaskContext::zero_init(),
            task_status: TaskStatus::UnInit,
        }; MAX_APP_NUM];*/
        let mut tasks = [TaskControlBlock::init(
            TaskStatus::UnInit, TaskContext::zero_init()
        ); MAX_APP_NUM];

        for (i, task) in tasks.iter_mut().enumerate() {
            task.task_cx = TaskContext::goto_restore(init_app_cx(i));
            task.task_status = TaskStatus::Ready;
        }
        TaskManager {
            num_app,
            inner: unsafe {
                UPSafeCell::new(TaskManagerInner {
                    tasks,
                    current_task: 0,
                })
            },
        }
    };
}

impl TaskManager {
    /// Run the first task in task list.
    ///
    /// Generally, the first task in task list is an idle task (we call it zero process later).
    /// But in ch3, we load apps statically, so the first task is a real app.
    fn run_first_task(&self) -> ! {
        println!("[kernel] run first task {}",0);
        //let mut inner = self.inner.exclusive_access();
        //let task0 = &mut inner.tasks[0];
        //task0.task_status = TaskStatus::Running;
        // 新增
        self.mark_current_running();
        let mut inner = self.inner.exclusive_access();
        let task0 = &mut inner.tasks[0];
        let next_task_cx_ptr = &task0.task_cx as *const TaskContext;
        drop(inner);
        let mut _unused = TaskContext::zero_init();
        // before this, we should drop local variables that must be dropped manually
        unsafe {
            __switch(&mut _unused as *mut TaskContext, next_task_cx_ptr);
        }
        panic!("unreachable in run_first_task!");
    }

    /// Change the status of current `Running` task into `Ready`.
    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        println!("[kernel] current task: {} suspended",current);
        inner.tasks[current].task_status = TaskStatus::Ready;

        //更新u mode运行时间
        let mut tracker = inner.tasks[current].task_tracker;
        tracker.u_mode_time += get_time() - tracker.last_run_moment;
    }

    /// Change the status of current `Running` task into `Exited`.
    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        println!("[kernel] current task: {} exited",current);
        inner.tasks[current].task_status = TaskStatus::Exited;

        // 打印汇总

    }

    /// Change the status of current `Ready` task into `Running`.
    fn mark_current_running(&self) {
        let mut inner = self.inner.exclusive_access();
        let current = inner.current_task;
        println!("[kernel] current task: {} is running",current);
        inner.tasks[current].task_status = TaskStatus::Running;

        // 更新时间信息
        let mut tracker = inner.tasks[current].task_tracker;
        tracker.last_run_moment = get_time();
        if tracker.start_time == 0 {
            tracker.start_time = tracker.last_run_moment;
        }
    }

    /// Find next task to run and return task id.
    ///
    /// In this case, we only return the first `Ready` task in task list.
    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    /// Switch current `Running` task to the task we have found,
    /// or there is no `Ready` task and we can exit with all applications completed
    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            println!("[kernel] find and run next task: {}",next);
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            //inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task = next;

            //新增
            drop(inner);
            self.mark_current_running();
            let mut inner = self.inner.exclusive_access();

            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
            drop(inner);
            // before this, we should drop local variables that must be dropped manually
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
            //
            
            // go back to user mode
        } else {
            println!("All applications completed!");
            use crate::board::QEMUExit;
            crate::board::QEMU_EXIT_HANDLE.exit_success();
        }
    }

    //新增一个获取当前task_id的方法
    /// get current task id
    pub fn get_current_task_id(&self) -> usize {
        self.inner.exclusive_access().current_task
    }

    /// 获取当前tcb的指针
    pub fn current_tcb_ptr(&self) -> *mut TaskControlBlock {
        // 为什么不可以返回&mut
        let current = self.get_current_task_id();
        &mut self.inner.exclusive_access().tasks[current] as *mut TaskControlBlock
    }

    /*/// 获取当前tcb的引用
    pub fn current_tcb_ref_mut(&mut self) -> &mut TaskControlBlock {
        // 为什么不可以返回&mut
        // 中间会生成局部变量无法借出
        let current = self.get_current_task_id();
        (*self.inner.exclusive_access()).tasks[current].borrow_mut()
    }*/
}

/// run first task
pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

/// rust next task
fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

/// suspend current task
fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

/// exit current task
fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

/*
/// run current task
fn mark_current_running() {
    TASK_MANAGER.mark_current_running();
}
*/

/// suspend current task, then run next task
pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

/// exit current task,  then run next task
pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}

/// update syscall time/s mode time for current task
pub fn update_task_call_time(duration:usize) {
    let tcb_ptr = TASK_MANAGER.current_tcb_ptr();
    unsafe {
        (*tcb_ptr).task_tracker.s_mode_time += duration;
    }
}
