//! Types related to task management

use super::TaskContext;

#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    pub task_status: TaskStatus,
    pub task_cx: TaskContext,
    // 下面是为了统计任务的执行情况而设置的属性
    pub task_tracker: TaskTracker,
}

// 新增初始化方法
impl TaskControlBlock {
    pub fn init(task_status: TaskStatus, task_cx: TaskContext) -> Self{
        TaskControlBlock {
            task_status,
            task_cx,
            task_tracker:TaskTracker::zero_init(),
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    UnInit,
    Ready,
    Running,
    Exited,
}

/// 用于追踪任务的执行情况
#[derive(Copy, Clone)]
pub struct TaskTracker {
    /// 任务开始时间
    pub start_time:usize,
    /// user mode累积运行时间
    pub u_mode_time:usize,
    /// s mode累积运行时间
    pub s_mode_time:usize,
    ///最后一次run的时刻
    pub last_run_moment:usize,
}

impl TaskTracker {
    pub fn init() -> Self {
        TaskTracker {
            start_time:0,
            u_mode_time:0,
            s_mode_time:0,
            last_run_moment:0,
        }
    }

    /// 所有字段初始化为0
    pub fn zero_init() -> Self {
        TaskTracker {
            start_time:0,
            u_mode_time:0,
            s_mode_time:0,
            last_run_moment:0,
        }
    }
}
