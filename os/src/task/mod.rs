mod context;
mod switch;
mod task;

use crate::loader::{get_num_app, get_app_data};
use crate::trap::TrapContext;
use crate::sync::UPSafeCell;
use lazy_static::*;
use switch::__switch;
use task::{TaskControlBlock, TaskStatus};
use alloc::vec::Vec;
use core::borrow::{Borrow, BorrowMut};

pub use context::TaskContext;
use crate::mm::{MapArea, MapPermission, MapType, VirtAddr, VirtPageNum};

pub struct TaskManager {
    num_app: usize,
    inner: UPSafeCell<TaskManagerInner>,
}

struct TaskManagerInner {
    tasks: Vec<TaskControlBlock>,
    current_task: usize,
}

/*impl TaskManagerInner {
    pub fn get_current_tcb_const(&self) -> &TaskControlBlock {
        self.tasks[self.current_task].borrow()
    }
}*/

lazy_static! {
    pub static ref TASK_MANAGER: TaskManager = {
        println!("init TASK_MANAGER");
        let num_app = get_num_app();
        println!("num_app = {}", num_app);
        let mut tasks: Vec<TaskControlBlock> = Vec::new();
        for i in 0..num_app {
            tasks.push(TaskControlBlock::new(
                get_app_data(i),
                i,
            ));
        }
        TaskManager {
            num_app,
            inner: unsafe { UPSafeCell::new(TaskManagerInner {
                tasks,
                current_task: 0,
            })},
        }
    };
}

impl TaskManager {
    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let next_task = &mut inner.tasks[0];
        next_task.task_status = TaskStatus::Running;
        let next_task_cx_ptr = &next_task.task_cx as *const TaskContext;
        drop(inner);
        let mut _unused = TaskContext::zero_init();
        // before this, we should drop local variables that must be dropped manually
        unsafe {
            __switch(
                &mut _unused as *mut _,
                next_task_cx_ptr,
            );
        }
        panic!("unreachable in run_first_task!");
    }

    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].task_status = TaskStatus::Ready;
    }

    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].task_status = TaskStatus::Exited;
    }

    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| {
                inner.tasks[*id].task_status == TaskStatus::Ready
            })
    }

    fn get_current_token(&self) -> usize {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_user_token()
    }

    fn get_current_trap_cx(&self) -> &mut TrapContext {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_trap_cx()
    }

    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task = next;
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
            drop(inner);
            // before this, we should drop local variables that must be dropped manually
            unsafe {
                __switch(
                    current_task_cx_ptr,
                    next_task_cx_ptr,
                );
            }
            // go back to user mode
        } else {
            panic!("All applications completed!");
        }
    }
}

/// 新加入的方法
impl TaskManager {
    /*fn get_current_page_table(&self) {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].
    }*/

    fn cross_current_memset(&self, start_vpn:VirtPageNum, end_vpn:VirtPageNum) -> bool{
        let inner = self.inner.exclusive_access();
        let current_tcb = &inner.tasks[inner.current_task];
        current_tcb.memory_set.vir_seg_cross_areas(start_vpn, end_vpn)
    }


    fn in_current_memset(&self, start_vpn:VirtPageNum, end_vpn:VirtPageNum) -> Option<(usize,usize)> {
        let inner = self.inner.exclusive_access();
        let current_tcb = &inner.tasks[inner.current_task];
        current_tcb.memory_set.vir_seg_in_areas(start_vpn, end_vpn)
    }

    /// 向当前任务的memeset中加入新的段
    fn current_memset_push(&self, start_va:VirtAddr, end_va:VirtAddr, permission:MapPermission) {
        let inner = self.inner.exclusive_access();
        let current_memset = &mut inner.tasks[inner.current_task].memory_set;
        current_memset.mmap_push(
            MapArea::new(
                start_va, end_va,
                MapType::Framed,
                permission),
            None
        );
    }

    fn current_memset_unmap(&self, start_va:VirtAddr, end_va:VirtAddr) -> bool {
        let inner = self.inner.exclusive_access();
        let current_memset = &mut inner.tasks[inner.current_task].memory_set;
        let seg_pos = current_memset.vir_seg_in_areas(
            start_va.floor().into(),
            end_va.floor().into()
        );
        match seg_pos {
            Some((start_index, end_index)) => {

                true
            },
            None => false,
        }
    }
}

pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}

pub fn current_user_token() -> usize {
    TASK_MANAGER.get_current_token()
}

pub fn current_trap_cx() -> &'static mut TrapContext {
    TASK_MANAGER.get_current_trap_cx()
}

/*pub fn current_tcb_const() -> &'static TaskControlBlock {
    TASK_MANAGER.get_current_tcb_const()
}*/

pub fn cross_current_memset(start_vpn:VirtPageNum, end_vpn:VirtPageNum) -> bool {
    TASK_MANAGER.cross_current_memset(start_vpn, end_vpn)
}

pub fn in_current_memset(start_vpn:VirtPageNum, end_vpn:VirtPageNum) -> Option<&'static mut MapArea> {
    TASK_MANAGER.in_current_memset(start_vpn, end_vpn)
}

pub fn current_memset_push(start_va:VirtAddr, end_va:VirtAddr, permission:MapPermission) {
    TASK_MANAGER.current_memset_push(start_va,end_va,permission)
}