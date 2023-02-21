use super::{PageTable, PageTableEntry, PTEFlags};
use super::{VirtPageNum, VirtAddr, PhysPageNum, PhysAddr};
use super::{FrameTracker, frame_alloc};
use super::{VPNRange, StepByOne};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use riscv::register::satp;
use alloc::sync::Arc;
use lazy_static::*;
use crate::sync::UPSafeCell;
use core::arch::{asm};
use core::borrow::{Borrow, BorrowMut};
use core::iter::Map;
use crate::config::{MEMORY_END, PAGE_SIZE, PAGE_SIZE_BITS, TRAMPOLINE, TRAP_CONTEXT, USER_STACK_SIZE};
use crate::tree::{Interval, IntervalMap};

extern "C" {
    fn stext();
    fn etext();
    fn srodata();
    fn erodata();
    fn sdata();
    fn edata();
    fn sbss_with_stack();
    fn ebss();
    fn ekernel();
    fn strampoline();
}

lazy_static! {
    pub static ref KERNEL_SPACE: Arc<UPSafeCell<MemorySet>> = Arc::new(unsafe {
        UPSafeCell::new(MemorySet::new_kernel()
    )});
}

#[derive(Copy,Clone)]
pub struct Segment {
    low:usize,
    high:usize,
}

// 为VPNRange实现Interval trait
impl Interval for Segment {
    type Item = usize;

    fn low(&self) -> Self::Item {
        self.low
    }

    fn high(&self) -> Self::Item {
        self.high
    }
}

impl Segment {
    pub fn new(l:usize,h:usize) -> Self {
        Self {
            low:l,
            high:h
        }
    }

    pub fn from_vpn(start:VirtPageNum, end:VirtPageNum) -> Self {
        Self {
            low:start.0,
            high:end.0,
        }
    }

    pub fn from_range(x:VPNRange) -> Self {
        Self {
            low:x.get_start().0,
            high:x.get_end().0 - 1,
        }
    }

    pub fn to_vpn_range(&self) -> VPNRange {
        VPNRange::new(
            self.low.into(),
            (self.high + 1).into()
        )
    }
}

pub struct MemorySet {
    page_table: PageTable,
    areas: Vec<MapArea>,
    // 新增加对映射内存的管理
    //mapped_areas:Vec<MapArea>,
    mapped_vpn_ranges:IntervalMap<Segment>,
    mapped_areas:BTreeMap<VirtPageNum,MapArea>
}

impl MemorySet {
    pub fn new_bare() -> Self {
        Self {
            page_table: PageTable::new(),
            areas: Vec::new(),
            mapped_vpn_ranges: IntervalMap::new(),
            mapped_areas: BTreeMap::new(),
        }
    }
    pub fn token(&self) -> usize {
        self.page_table.token()
    }
    /// Assume that no conflicts.
    pub fn insert_framed_area(&mut self, start_va: VirtAddr, end_va: VirtAddr, permission: MapPermission) {
        self.push(MapArea::new(
            start_va,
            end_va,
            MapType::Framed,
            permission,
        ), None);
    }
    /// map and push into memset
    pub(crate) fn push(&mut self, mut map_area: MapArea, data: Option<&[u8]>) {
        map_area.map(&mut self.page_table);
        if let Some(data) = data {
            map_area.copy_data(&mut self.page_table, data);
        }
        self.areas.push(map_area);
    }
    /// Mention that trampoline is not collected by areas.
    fn map_trampoline(&mut self) {
        self.page_table.map(
            VirtAddr::from(TRAMPOLINE).into(),
            PhysAddr::from(strampoline as usize).into(),
            PTEFlags::R | PTEFlags::X,
        );
    }
    /// Without kernel stacks.
    pub fn new_kernel() -> Self {
        let mut memory_set = Self::new_bare();
        // map trampoline
        memory_set.map_trampoline();
        // map kernel sections
        println!(".text [{:#x}, {:#x})", stext as usize, etext as usize);
        println!(".rodata [{:#x}, {:#x})", srodata as usize, erodata as usize);
        println!(".data [{:#x}, {:#x})", sdata as usize, edata as usize);
        println!(".bss [{:#x}, {:#x})", sbss_with_stack as usize, ebss as usize);
        println!("mapping .text section");
        memory_set.push(MapArea::new(
            (stext as usize).into(),
            (etext as usize).into(),
            MapType::Identical,
            MapPermission::R | MapPermission::X,
        ), None);
        println!("mapping .rodata section");
        memory_set.push(MapArea::new(
            (srodata as usize).into(),
            (erodata as usize).into(),
            MapType::Identical,
            MapPermission::R,
        ), None);
        println!("mapping .data section");
        memory_set.push(MapArea::new(
            (sdata as usize).into(),
            (edata as usize).into(),
            MapType::Identical,
            MapPermission::R | MapPermission::W,
        ), None);
        println!("mapping .bss section");
        memory_set.push(MapArea::new(
            (sbss_with_stack as usize).into(),
            (ebss as usize).into(),
            MapType::Identical,
            MapPermission::R | MapPermission::W,
        ), None);
        println!("mapping physical memory");
        memory_set.push(MapArea::new(
            (ekernel as usize).into(),
            MEMORY_END.into(),
            MapType::Identical,
            MapPermission::R | MapPermission::W,
        ), None);
        memory_set
    }
    /// Include sections in elf and trampoline and TrapContext and user stack,
    /// also returns user_sp and entry point.
    pub fn from_elf(elf_data: &[u8]) -> (Self, usize, usize) {
        let mut memory_set = Self::new_bare();
        // map trampoline
        memory_set.map_trampoline();
        // map program headers of elf, with U flag
        let elf = xmas_elf::ElfFile::new(elf_data).unwrap();
        let elf_header = elf.header;
        let magic = elf_header.pt1.magic;
        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");
        let ph_count = elf_header.pt2.ph_count();
        let mut max_end_vpn = VirtPageNum(0);
        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
                let start_va: VirtAddr = (ph.virtual_addr() as usize).into();
                let end_va: VirtAddr = ((ph.virtual_addr() + ph.mem_size()) as usize).into();
                let mut map_perm = MapPermission::U;
                let ph_flags = ph.flags();
                if ph_flags.is_read() { map_perm |= MapPermission::R; }
                if ph_flags.is_write() { map_perm |= MapPermission::W; }
                if ph_flags.is_execute() { map_perm |= MapPermission::X; }
                let map_area = MapArea::new(
                    start_va,
                    end_va,
                    MapType::Framed,
                    map_perm,
                );
                max_end_vpn = map_area.vpn_range.get_end();
                memory_set.push(
                    map_area,
                    Some(&elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize])
                );
            }
        }
        // map user stack with U flags
        let max_end_va: VirtAddr = max_end_vpn.into();
        let mut user_stack_bottom: usize = max_end_va.into();
        // guard page
        user_stack_bottom += PAGE_SIZE;
        let user_stack_top = user_stack_bottom + USER_STACK_SIZE;
        memory_set.push(MapArea::new(
            user_stack_bottom.into(),
            user_stack_top.into(),
            MapType::Framed,
            MapPermission::R | MapPermission::W | MapPermission::U,
        ), None);
        // map TrapContext
        memory_set.push(MapArea::new(
            TRAP_CONTEXT.into(),
            TRAMPOLINE.into(),
            MapType::Framed,
            MapPermission::R | MapPermission::W,
        ), None);
        (memory_set, user_stack_top, elf.header.pt2.entry_point() as usize)
    }
    pub fn activate(&self) {
        let satp = self.page_table.token();
        unsafe {
            satp::write(satp);
            asm!("sfence.vma");
        }
    }
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.page_table.translate(vpn)
    }
}

/// 新增加的方法
impl MemorySet {
    pub fn get_page_table(&mut self) -> &mut PageTable {
        self.page_table.borrow_mut()
    }

    /// map新的虚拟空间
    pub(crate) fn mmap_push(&mut self, mut map_area: MapArea, data: Option<&[u8]>) {
        map_area.map(&mut self.page_table);
        if let Some(data) = data {
            map_area.copy_data(&mut self.page_table, data);
        }
        self.mapped_vpn_ranges.interval_insert(Segment::from_range(map_area.vpn_range));
        self.mapped_areas.insert(map_area.vpn_range.get_start(),map_area);
    }

    pub fn mmap_delete(&mut self, seg:Segment) {
        let map_area = self.mapped_areas.get_mut(&seg.low.into());
        match map_area {
            Some(area) => {
                self.mapped_vpn_ranges.interval_delete(seg);
                area.unmap(&mut self.page_table);
            },
            _ => {
                panic!("[kernel] can not delete unmapped area")
            }
        }
    }

    /// 只要虚拟段与areas的任意一个段重叠就返回重叠的段地址
    pub fn overlap_segment(&self, start:VirtPageNum, end:VirtPageNum) -> Option<&MapArea> {
        // 开闭区间问题
        let res = self.mapped_vpn_ranges.interval_search(
            Segment::from_vpn(start,end));
        match res {
            Some(seg) => {
                let res = self.search_mmap_area(
                    seg.low.into()
                );
                if res.is_none() {
                    panic!("[kernel] System Error: unmapped area")
                }
                res
            },
            None => None
        }
    }

    /// 尝试删除一片区域，只要x中有没有被map的区域就会失败
    fn delete_segment(&mut self, x: Segment) -> bool {
        let res = self.mapped_vpn_ranges.interval_search(x);
        if res.is_none() {
            return false;
        }
        else {
            let res = res.unwrap();
            let x_l = x.low();
            let x_h = x.high();
            let res_l = res.low();
            let res_h = res.high();

            let mut new_l= x_l;
            let mut new_h = x_h;

            if x_h > res_h {
                let l= res_h + 1;
                if self.delete_segment(Segment::new(l,x_h)) {
                    new_h = res_h;
                }
                else {
                    return false;
                }
            }

            if x_l < res_l {
                let h = res_l - 1;
                if self.delete_segment(Segment::new(x_l,h)) {
                    new_l = res_l;
                }
                else {
                    return false;
                }
            }

            let new_node = Segment::new(new_l,new_h);
            /*if new_l == res_l {
                m.interval_insert(new_node);
            }
            else {
                m.interval_delete(res);
                m.interval_insert(new_node);
            }*/
            let area_to_delete = self.mapped_areas.get_mut(&res_l.into()).unwrap();
            let perm = area_to_delete.map_perm;
            let map_type = area_to_delete.map_type;
            self.mapped_vpn_ranges.interval_delete(res);
            area_to_delete.unmap(&mut self.page_table);
            self.mapped_vpn_ranges.interval_insert(new_node);
            let new_range = new_node.to_vpn_range();
            let new_area = MapArea::new(
                new_range.get_start().into(),
                new_range.get_end().into(),
                map_type,
                perm
            );
            self.mmap_push(new_area,None);
            return true;
        }
    }

    pub fn try_delete_range(&mut self, start:VirtPageNum, end:VirtPageNum) -> bool{
        let seg = Segment::from_vpn(start,end);
        self.delete_segment(seg)
    }

    pub fn search_mmap_area(&self, start_vpn:VirtPageNum) -> Option<&MapArea>{
        self.mapped_areas.get(&start_vpn)
    }
}

pub struct MapArea {
    vpn_range: VPNRange,
    data_frames: BTreeMap<VirtPageNum, FrameTracker>,
    map_type: MapType,
    map_perm: MapPermission,
}

impl MapArea {
    pub fn new(
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_type: MapType,
        map_perm: MapPermission
    ) -> Self {
        let start_vpn: VirtPageNum = start_va.floor();
        let end_vpn: VirtPageNum = end_va.ceil();
        Self {
            vpn_range: VPNRange::new(start_vpn, end_vpn),
            data_frames: BTreeMap::new(),
            map_type,
            map_perm,
        }
    }
    pub fn map_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        let ppn: PhysPageNum;
        match self.map_type {
            MapType::Identical => {
                ppn = PhysPageNum(vpn.0);
            }
            MapType::Framed => {
                let frame = frame_alloc().unwrap();
                ppn = frame.ppn;
                self.data_frames.insert(vpn, frame);
            }
        }
        let pte_flags = PTEFlags::from_bits(self.map_perm.bits).unwrap();
        page_table.map(vpn, ppn, pte_flags);
    }
    #[allow(unused)]
    pub fn unmap_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        match self.map_type {
            MapType::Framed => {
                self.data_frames.remove(&vpn);
            }
            _ => {}
        }
        page_table.unmap(vpn);
    }
    pub fn map(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.map_one(page_table, vpn);
        }
    }
    #[allow(unused)]
    pub fn unmap(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.unmap_one(page_table, vpn);
        }
    }
    /// data: start-aligned but maybe with shorter length
    /// assume that all frames were cleared before
    pub fn copy_data(&mut self, page_table: &mut PageTable, data: &[u8]) {
        assert_eq!(self.map_type, MapType::Framed);
        let mut start: usize = 0;
        let mut current_vpn = self.vpn_range.get_start();
        let len = data.len();
        loop {
            let src = &data[start..len.min(start + PAGE_SIZE)];
            let dst = &mut page_table
                .translate(current_vpn)
                .unwrap()
                .ppn()
                .get_bytes_array()[..src.len()];
            dst.copy_from_slice(src);
            start += PAGE_SIZE;
            if start >= len {
                break;
            }
            current_vpn.step();
        }
    }

    /*// range前闭后开
    fn in_vpn_range(&self, vpn:VirtPageNum) -> bool {
        vpn.0 >= self.vpn_range.get_start().0 && vpn.0 < self.vpn_range.get_end().0
    }*/

    pub fn vpn_range(&self) -> &VPNRange {
        self.vpn_range.borrow()
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MapType {
    Identical,
    Framed,
}

bitflags! {
    pub struct MapPermission: u8 {
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}

#[allow(unused)]
pub fn remap_test() {
    let mut kernel_space = KERNEL_SPACE.exclusive_access();
    let mid_text: VirtAddr = ((stext as usize + etext as usize) / 2).into();
    let mid_rodata: VirtAddr = ((srodata as usize + erodata as usize) / 2).into();
    let mid_data: VirtAddr = ((sdata as usize + edata as usize) / 2).into();
    assert_eq!(
        kernel_space.page_table.translate(mid_text.floor()).unwrap().writable(),
        false
    );
    assert_eq!(
        kernel_space.page_table.translate(mid_rodata.floor()).unwrap().writable(),
        false,
    );
    assert_eq!(
        kernel_space.page_table.translate(mid_data.floor()).unwrap().executable(),
        false,
    );
    println!("remap_test passed!");
}