use core::arch::asm;
use core::slice;
use lazy_static::*;
use crate::sync::UPSafeCell;
use crate::trap::TrapContext;

const MAX_APP_NUM:usize = 1024;
const APP_BASE_ADDRESS:usize = 0x80400000;
const APP_MAX_SIZE:usize = 0x20000;
const KERNEL_STACK_SIZE:usize = 4096 *2;
const USER_STACK_SIZE:usize = 4096 *2;

#[repr(align(4096))]
struct KernelStack {
    data: [u8; KERNEL_STACK_SIZE],
}

#[repr(align(4096))]
struct UserStack {
    data: [u8; USER_STACK_SIZE],
}

static KERNEL_STACK: KernelStack = KernelStack {
    data: [0; KERNEL_STACK_SIZE],
};
static USER_STACK: UserStack = UserStack {
    data: [0; USER_STACK_SIZE],
};

impl KernelStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + KERNEL_STACK_SIZE
    }

    ///压入一个TrapContext，返回它的可变引用
    pub fn push_context(&self, cx: TrapContext) -> &'static mut TrapContext {
        //在内核栈上分配一个TrapContext的空间
        let cx_ptr = (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
        unsafe {
            *cx_ptr = cx;
        }
        unsafe { cx_ptr.as_mut().unwrap() }
    }
}

impl UserStack {
    fn get_sp(&self) -> usize {
        self.data.as_ptr() as usize + USER_STACK_SIZE
    }
}

struct AppManager {
    app_num:usize,
    current_app:usize,
    app_start:[usize;MAX_APP_NUM + 1],
}



impl AppManager {
    unsafe fn new() -> AppManager {
        extern "C" {
            fn _num_app();
        }
        //无法在编译时确定数组大小,可以使用Vec
        //let mut app_starts:Vec<usize>;
        let num_app_ptr = _num_app as usize as *const usize;
        let app_num = unsafe { num_app_ptr.read_volatile() };
        let mut app_start: [usize; MAX_APP_NUM + 1] = [0; MAX_APP_NUM + 1];
        let slice = slice::from_raw_parts(num_app_ptr.add(1), app_num + 1);
        app_start[..app_num + 1].copy_from_slice(slice);
        AppManager {
            app_num,
            current_app: 0,
            app_start,
        }
    }

    unsafe fn load_app(&mut self, app_id:usize) {
        if app_id >= self.app_num {
            //panic!("app_id exceed max limit {}, invalid app_id: {}", self.app_num - 1, app_id);
            //这里需要让os自动退出而不是panic
            //qemu退出的代码已经给出了
            println!("All applications completed!");
            use crate::board::QEMUExit;
            crate::board::QEMU_EXIT_HANDLE.exit_success();
        }
        println!("[kernel] Loading app_{}", app_id);
        //下面需要把程序复制到APP_BASE_ADDRESS下
        //首先清空这个区域
        (APP_BASE_ADDRESS .. APP_MAX_SIZE + APP_MAX_SIZE).for_each(
            |byte| unsafe {(byte as *mut u8).write_volatile(0)});
        //然后复制数据到指定位置
        //data_dst必须是可变切片
        let data_src = slice::from_raw_parts(
            self.app_start[app_id] as *const u8,
            self.app_start[app_id + 1] - self.app_start[app_id]);
        let data_dst = slice::from_raw_parts_mut(
            APP_BASE_ADDRESS as *mut u8,
            data_src.len());
        data_dst.copy_from_slice(data_src);
        //最后清空指令缓存
        asm!("fence.i");
    }

    pub fn print_app_info(&self) {
        println!("[kernel] app nums: {}", self.app_num);
        for i in 0..self.app_num {
            //16进制输出
            println!("[kernel] app_{} start: {:#x}, end: {:#x}",
                     i, self.app_start[i],self.app_start[i+1]-1);
        }
    }

    pub fn get_current_app(&self) -> usize {
        self.current_app
    }

    pub fn move_to_next_app(&mut self) {
        self.current_app += 1;
    }
}

lazy_static! {
    static ref APP_MANAGER: UPSafeCell<AppManager> = unsafe{
        UPSafeCell::new(AppManager::new())
    };
}

pub fn print_app_info() {
    APP_MANAGER.exclusive_borrow().print_app_info();
}

pub fn run_next_app() -> !{
    let mut manager = APP_MANAGER.exclusive_borrow();
    let current_app = manager.get_current_app();
    //加载当前的app数据
    unsafe {
        manager.load_app(current_app);
    }

    manager.move_to_next_app();
    //需要提前drop
    drop(manager);
    extern "C" {
        //传入context的地址
        fn __restore(cx_addr: usize);
    }
    unsafe {
        __restore(KERNEL_STACK.push_context(
            TrapContext::app_init_context(
                APP_BASE_ADDRESS,
                USER_STACK.get_sp()
            )) as *const _ as usize
        );
    }
    panic!("Unreachable in batch::run_current_app!");
}

pub fn init() {
    print_app_info();
}

