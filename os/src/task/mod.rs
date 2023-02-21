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
use crate::tree::Interval;

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

    fn cross_current_memset(&self, start_vpn:VirtPageNum, end_vpn:VirtPageNum) -> Option<(VirtPageNum,VirtPageNum)>{
        let inner = self.inner.exclusive_access();
        let current_tcb = &inner.tasks[inner.current_task];
        let res = current_tcb.memory_set.overlap_segment(start_vpn,end_vpn);
        match res {
            Some(area) => {
                let range = area.vpn_range();
                let start_vpn = range.get_start();
                let end_vpn:VirtPageNum = (range.get_end().0 - 1).into();
                Some((start_vpn,end_vpn))
            }
            None => None
        }
    }

    /// 向当前任务的memeset中加入新的段
    fn current_memset_push(&self, start_va:VirtAddr, end_va:VirtAddr, permission:MapPermission, data:Option<&[u8]>) {
        let mut inner = self.inner.exclusive_access();
        let current_task = inner.current_task;
        let current_memset = &mut inner.tasks[current_task].memory_set;
        current_memset.mmap_push(
            MapArea::new(
                start_va, end_va,
                MapType::Framed,
                permission),
            data
        );
    }

    //
    fn current_memset_mmap(&self, start_va:VirtAddr, end_va:VirtAddr, permission:MapPermission, data: Option<&[u8]>) -> bool {
        let start_vpn:VirtPageNum = start_va.floor().into();
        let end_vpn:VirtPageNum = end_va.floor().into();
        let cross = self.cross_current_memset(start_vpn,end_vpn);
        match cross {
            Some(_) => {
                self.current_memset_push(start_va,end_va,permission,data);
                true
            }
            None => false
        }
    }

    // 尝试删除[start_va,end_va]的段
    fn current_memset_unmap(&self, start_va:VirtAddr, end_va:VirtAddr) -> bool {
        let mut inner = self.inner.exclusive_access();
        let current_task = inner.current_task;
        let current_memset = &mut inner.tasks[current_task].memory_set;
        current_memset.try_delete_range(start_va.floor().into(),
                                        end_va.floor().into())
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

pub fn current_mmap(start_va:usize, end_va:usize, prot:usize) -> bool{
    let start_va:VirtAddr = start_va.into();
    let end_va = end_va.into();
    let mut permission = MapPermission::R | MapPermission::U;
    if prot & 0x2 != 0 {
        permission |= MapPermission::W;
    }
    if prot & 0x4 != 0 {
        permission |= MapPermission::X;
    }
    TASK_MANAGER.current_memset_mmap(start_va, end_va, permission,None)
}

pub fn current_unmap(start_va:usize, end_va:usize) -> bool {
    TASK_MANAGER.current_memset_unmap(start_va.into(),end_va.into())
}