use super::{
    BlockDevice,
    DiskInode,
    DiskInodeType,
    DirEntry,
    EasyFileSystem,
    DIRENT_SZ,
    get_block_cache,
    block_cache_sync_all,
};
use alloc::sync::Arc;
use alloc::string::String;
use alloc::vec::Vec;
use core::cmp::min;
use spin::{Mutex, MutexGuard};

pub struct Inode {
    block_id: usize,
    block_offset: usize,
    fs: Arc<Mutex<EasyFileSystem>>,
    block_device: Arc<dyn BlockDevice>,
}

impl Inode {
    /// We should not acquire efs lock here.
    pub fn new(
        block_id: u32,
        block_offset: usize,
        fs: Arc<Mutex<EasyFileSystem>>,
        block_device: Arc<dyn BlockDevice>,
    ) -> Self {
        Self {
            block_id: block_id as usize,
            block_offset,
            fs,
            block_device,
        }
    }

    // 权限变更为公有
    pub fn read_disk_inode<V>(&self, f: impl FnOnce(&DiskInode) -> V) -> V {
        get_block_cache(
            self.block_id,
            Arc::clone(&self.block_device)
        ).lock().read(self.block_offset, f)
    }

    fn modify_disk_inode<V>(&self, f: impl FnOnce(&mut DiskInode) -> V) -> V {
        get_block_cache(
            self.block_id,
            Arc::clone(&self.block_device)
        ).lock().modify(self.block_offset, f)
    }

    // 不仅要返回inode number还要返回在目录中的位置,即(pos,inode)
    fn __find_inode(
        &self,
        name: &str,
        disk_inode: &DiskInode,
    ) -> Option<(usize,u32)> {
        // assert it is a directory
        assert!(disk_inode.is_dir());
        let file_count = (disk_inode.size as usize) / DIRENT_SZ;
        let mut dirent = DirEntry::empty();
        for i in 0..file_count {
            assert_eq!(
                disk_inode.read_at(
                    DIRENT_SZ * i,
                    dirent.as_bytes_mut(),
                    &self.block_device,
                ),
                DIRENT_SZ,
            );
            if dirent.name() == name {
                return Some((i,dirent.inode_number() as u32));
            }
        }
        None
    }

    // 只返回inode_id，不返回位置
    #[inline]
    fn find_inode_id(
        &self,
        name: &str,
        disk_inode: &DiskInode,
    ) -> Option<u32> {
        match self.__find_inode(name,disk_inode) {
            Some((_,inode)) => Some(inode),
            None => None
        }
    }

    // 对外暴露的在磁盘块上查询inode number的接口
    #[inline]
    pub fn find_inode_number(&self, name: &str) -> Result<Option<u32>,&'static str>{
        self.read_disk_inode(|disk_inode|{
            if disk_inode.is_dir() {
                Ok(self.find_inode_id(name,disk_inode))
            }
            else {
                Err("Not a file")
            }
        })
    }

    // 找到目录下文件名为name的目录项的位置并生成指向Inode的指针
    fn find_inode_pos(&self, name: &str) -> Option<(usize, Arc<Inode>)>{
        let fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            self.__find_inode(name, disk_inode)
                .map(|(pos,inode_id)| {
                    let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id);
                    (pos,
                     Arc::new(Self::new(
                        block_id,
                        block_offset,
                        self.fs.clone(),
                        self.block_device.clone(),
                    )))
                })
        })
    }

    // 只返回目录下文件名为name的目录或文件的Inode指针
    pub fn find(&self, name: &str) -> Option<Arc<Inode>> {
        let fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            self.find_inode_id(name, disk_inode)
            .map(|inode_id| {
                let (block_id, block_offset) = fs.get_disk_inode_pos(inode_id);
                Arc::new(Self::new(
                    block_id,
                    block_offset,
                    self.fs.clone(),
                    self.block_device.clone(),
                ))
            })
        })
    }

    fn increase_size(
        &self,
        new_size: u32,
        disk_inode: &mut DiskInode,
        fs: &mut MutexGuard<EasyFileSystem>,
    ) {
        if new_size < disk_inode.size {
            return;
        }
        let blocks_needed = disk_inode.blocks_num_needed(new_size);
        let mut v: Vec<u32> = Vec::new();
        for _ in 0..blocks_needed {
            v.push(fs.alloc_data());
        }
        disk_inode.increase_size(new_size, v, &self.block_device);
    }

    // 封装了increase_self_size,使得函数更方便使用
    fn increase_self_size(
        &self,
        new_size: u32,
    ) {
        self.modify_disk_inode(|disk_inode|{
            if new_size < disk_inode.size {
                return;
            }
            let blocks_needed = disk_inode.blocks_num_needed(new_size);
            let mut v: Vec<u32> = Vec::new();
            let fs = &mut self.fs.lock();
            for _ in 0..blocks_needed {
                v.push(fs.alloc_data());
            }
            disk_inode.increase_size(new_size, v, &self.block_device);
        })
    }

    // 与increase_self_size对应的减小Inode文件块的方法
    fn decrease_self_size(
        &self,
        new_size: u32,
    ) {
        self.modify_disk_inode(|disk_inode|{
            if new_size > disk_inode.size {
                return;
            }
            let data_blocks_dealloc = disk_inode.decrease_size(new_size, &self.block_device);
            for data_block in data_blocks_dealloc.into_iter() {
                self.fs.lock().dealloc_data(data_block);
            }
        });
        block_cache_sync_all();
    }

    // 创建文件和目录的通用接口
    // 修改函数，可以指定inode_number
    fn __create(&self, name: &str, inode_number: Option<u32>, inode_type:DiskInodeType) -> Option<Arc<Inode>> {
        let mut fs = self.fs.lock();
        if self.modify_disk_inode(|root_inode| {
            // assert it is a directory
            assert!(root_inode.is_dir());
            // has the file been created?
            self.find_inode_id(name, root_inode)
        }).is_some() {
            return None;
        }
        // create a new file
        // alloc a inode with an indirect block
        // 修改new_inode_id的初始化方式
        let new_inode_id = if inode_number.is_some()
            {inode_number.unwrap()} else {fs.alloc_inode()};
        // initialize inode
        let (new_inode_block_id, new_inode_block_offset) 
            = fs.get_disk_inode_pos(new_inode_id);
        let parent_ino = self.get_ino();
        get_block_cache(
            new_inode_block_id as usize,
            Arc::clone(&self.block_device)
        ).lock().modify(new_inode_block_offset, |new_inode: &mut DiskInode| {
            if inode_number.is_none() {
                // 没有指定inode,直接初始化
                new_inode.initialize(
                    inode_type,
                    new_inode_block_id, 
                    parent_ino
                );
            }
            // nlink + 1
            new_inode.add_nlink();
        });
        self.modify_disk_inode(|root_inode| {
            // append file in the dirent
            let file_count = (root_inode.size as usize) / DIRENT_SZ;
            let new_size = (file_count + 1) * DIRENT_SZ;
            // increase size
            self.increase_size(new_size as u32, root_inode, &mut fs);
            // write dirent
            let dirent = DirEntry::new(name, new_inode_id);
            root_inode.write_at(
                file_count * DIRENT_SZ,
                dirent.as_bytes(),
                &self.block_device,
            );
        });

        let (block_id, block_offset) = fs.get_disk_inode_pos(new_inode_id);
        block_cache_sync_all();
        // return inode
        Some(Arc::new(Self::new(
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        )))
        // release efs lock automatically by compiler
    }

    // 不改变原有接口，创建文件
    #[inline]
    pub fn create(&self, name: &str) -> Option<Arc<Inode>> {
        self.__create(name,None,DiskInodeType::File)
    }

    fn check_name(&self, name: &str) -> bool{
        !(name.starts_with("./") || name.starts_with("../"))
    }

    // 创建目录
    #[inline]
    pub fn create_dir(&self, name: &str) -> Option<Arc<Inode>> {
        assert!(self.check_name(name));
        self.__create(name,None,DiskInodeType::Directory)
    }

    // 根据已有inode创建文件
    #[inline]
    pub fn create_by_inode(&self, name: &str,inode_number:u32) -> Option<Arc<Inode>> {
        assert!(self.check_name(name));
        self.__create(name,Some(inode_number),DiskInodeType::File)
    }

    pub fn ls(&self) -> Vec<String> {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            let file_count = (disk_inode.size as usize) / DIRENT_SZ;
            let mut v: Vec<String> = Vec::new();
            for i in 0..file_count {
                let mut dirent = DirEntry::empty();
                assert_eq!(
                    disk_inode.read_at(
                        i * DIRENT_SZ,
                        dirent.as_bytes_mut(),
                        &self.block_device,
                    ),
                    DIRENT_SZ,
                );
                v.push(String::from(dirent.name()));
            }
            v
        })
    }

    // 新增加get_dents方法读取目录项
    pub fn get_dents(&self, count:usize) -> Vec<DirEntry> {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            let file_count = (disk_inode.size as usize) / DIRENT_SZ;
            let mut v: Vec<DirEntry> = Vec::new();
            for i in 0..min(file_count,count) {
                let mut dirent = DirEntry::empty();
                assert_eq!(
                    disk_inode.read_at(
                        i * DIRENT_SZ,
                        dirent.as_bytes_mut(),
                        &self.block_device,
                    ),
                    DIRENT_SZ,
                );
                v.push(dirent);
            }
            v
        })
    }

    pub fn read_at(&self, offset: usize, buf: &mut [u8]) -> usize {
        let _fs = self.fs.lock();
        self.read_disk_inode(|disk_inode| {
            disk_inode.read_at(offset, buf, &self.block_device)
        })
    }

    pub fn write_at(&self, offset: usize, buf: &[u8]) -> usize {
        let mut fs = self.fs.lock();
        let size = self.modify_disk_inode(|disk_inode| {
            self.increase_size((offset + buf.len()) as u32, disk_inode, &mut fs);
            disk_inode.write_at(offset, buf, &self.block_device)
        });
        block_cache_sync_all();
        size
    }

    pub fn clear(&self) {
        let mut fs = self.fs.lock();
        self.modify_disk_inode(|disk_inode| {
            let size = disk_inode.size;
            let data_blocks_dealloc = disk_inode.clear_size(&self.block_device);
            assert_eq!(data_blocks_dealloc.len(), DiskInode::total_blocks(size) as usize);
            for data_block in data_blocks_dealloc.into_iter() {
                fs.dealloc_data(data_block);
            }
        });
        block_cache_sync_all();
    }
}

impl Inode {
    // 实现fallocate系统调用
    pub fn fallocate(&self, offset:usize ,len:usize) {
        let origin_size= self.read_disk_inode(
            |disk_inode| {disk_inode.size as usize});
        self.increase_self_size((origin_size + len) as u32);
        const BUF_SZ: usize = 512;
        let read_bound = offset + BUF_SZ;
        let mut buf = [0u8;BUF_SZ];
        let mut read_start =
            if origin_size > BUF_SZ
                {origin_size - BUF_SZ}
            else { origin_size };
        let mut write_start = read_start + len;

        while read_start > read_bound {
            self.read_at(read_start as usize,&mut buf);
            self.write_at(write_start as usize,&mut buf);
            // 不会向下溢出,read_bound > BUF_SZ,
            // 又write_start > read_start > read_bound
            // read_start - BUF_SZ > 0 , WRITE_start - BUF_SZ > 0
            write_start -= BUF_SZ;
            read_start -= BUF_SZ;
        }
        if read_start > offset {
            let last_size = read_start - offset;
            self.read_at(offset,&mut buf[..last_size]);
            self.write_at(offset + len,&mut buf[..last_size]);
        }
    }

    // 实现fdeallocate
    pub fn fdeallocate(&self, offset:usize, len:usize) {
        let origin_size= self.read_disk_inode(
            |disk_inode| {disk_inode.size as usize});
        const BUF_SZ: usize = 512;
        // 这里无符号数相减的会溢出
        let read_bound = if origin_size > BUF_SZ {origin_size - BUF_SZ} else {0};
        let mut buf = [0u8;BUF_SZ];
        let mut write_start = offset;
        let mut read_start = min(offset + len,origin_size);
        while read_start < read_bound {
            self.read_at(read_start,&mut buf);
            self.write_at(write_start,&mut buf);
            write_start += BUF_SZ;
            read_start += BUF_SZ;
        }
        if read_start < origin_size {
            let last_size = origin_size - read_start;
            self.read_at(read_start,&mut buf[..last_size]);
            self.write_at(write_start,&mut buf[..last_size]);
        }
        // 最后要回收空缺的块
        let new_size = origin_size - len;
        self.decrease_self_size(new_size as u32);
    }

    /// 删除目录下的文件或空目录
    pub fn remove(&self, name: &str) -> Result<(), &'static str>{
        let res= self.find_inode_pos(name);
        match res {
            Some((pos,inode)) => {
                let mut nlink= 0;
                inode.modify_disk_inode(|disk_inode| {
                    disk_inode.sub_nlink();
                    nlink = disk_inode.nlink;
                });
                if nlink == 0 {
                    if inode.read_disk_inode(
                        |disk_inode| {
                            disk_inode.is_file()
                        }) {
                        // 链接数为0,删除文件
                        inode.clear();
                    }
                    else if inode.read_disk_inode(
                        |disk_inode| {disk_inode.size}) == 0 { 
                        // 尝试删除目录
                        inode.clear();
                    }
                    else { 
                        return Err("Directory is not empty");
                    }
                }
                // 回收一个目录项
                self.fdeallocate(pos*DIRENT_SZ,DIRENT_SZ);
                Ok(())
            }
            None => Err("Can not find file")
        }
    }

    /// 新建一个link
    #[inline]
    pub fn link(&self, name: &str, inode_number:u32) -> Option<Arc<Inode>>{
        self.create_by_inode(name,inode_number)
    }

    /// 查看文件信息,返回三元组(inode_number, is_file, nlink)
    pub fn fstat(&self) -> OSStat {
        self.read_disk_inode(|disk_inode| {
            OSStat {
                size: disk_inode.size,
                ino: disk_inode.ino,
                f_type: disk_inode.type_.clone(),
                nlink: disk_inode.nlink,
            }
        })
    }

    // 冗余的设计
    /*
    /// 创建目录
    fn __create_dir(&self, path: &[&str]) -> Option<Arc<Inode>> {
        if path.len() == 1 {
            self.__create(path[0],None, DiskInodeType::Directory)
        }
        else {
            // 寻找下一层的目录Inode
            let next_dir = self.find(path[0]);
            match next_dir {
                Some(dir) => dir.__create_dir(&path[1..]),
                None => None
            }
        }
    }

    // 消除相对目录的“./”和“../”前缀,
    // 返回指向新的Inode的指针(如果没有更改就返回None)和不含前缀的相对目录
    fn remove_prefix<'a>(&self, relative_path: &'a str) -> (Option<Arc<Inode>>,&'a str) {
        let mut parent_dir = None;
        let mut path = relative_path;
        loop {
            if path.starts_with("../") {
                parent_dir = Some(Arc::new(self.parent_inode()));
                path = &path[3..];
            }
            else if path.starts_with("./") {
                path = &path[2..];
            }
            else {
                break
            }
        }
        (parent_dir,path)
    }

    pub fn create_dir(&self, relative_path: &str) -> Option<Arc<Inode>> {
        // 只接受相对路径
        assert_eq!(relative_path.starts_with("/"),false);
        let (parent_dir, path) = self.remove_prefix(relative_path);
        let path:Vec<&str> = path.split('/').collect();
        match parent_dir {
            Some(dir) => dir.__create_dir(&path.as_slice()),
            None => self.__create_dir(&path.as_slice())
        }
    }

    /// 删除空目录
    fn __remove_dir(&self, path: &[&str]) -> Result<(),&'static str> {
        // 寻找下一层的目录Inode
        let next_dir = self.find(path[0]);
        match next_dir {
            Some(dir) => {
                if path.len() == 1 {
                    let size = dir.read_disk_inode(
                        |disk_node|{disk_node.size});
                    if size == 0 && self.remove_file(path[0]).is_ok(){
                        Ok(())
                    }
                    else {
                        Err("Error: directory is not empty")
                    }
                }
                else {
                    dir.__remove_dir(&path[1..])
                }
            },
            None => Err("Can not found directory")
        }
    }

    pub fn remove_dir(&self, path: &str) -> Result<(),&'static str> {
        let (parent_dir, path) = self.remove_prefix(path);
        let path:Vec<&str> = path.split('/').collect();
        match parent_dir {
            Some(dir) => dir.__remove_dir(&path.as_slice()),
            None => self.__remove_dir(&path.as_slice())
        }
    }*/
    
    // 返回ino
    pub fn get_ino(&self) -> u32 {
        self.read_disk_inode(|disk_inode| {
            disk_inode.ino
        })
    }
    
    pub fn get_parent_ino(&self) -> u32 {
        self.read_disk_inode(|disk_inode| {
            disk_inode.parent_ino
        })
    }

    // 返回file system的引用
    #[inline]
    pub fn get_fs(&self) -> Arc<Mutex<EasyFileSystem>> {
        self.fs.clone()
    }

    #[inline]
    pub fn get_device(&self) -> Arc<dyn BlockDevice> {
        self.block_device.clone()
    }

    #[inline]
    pub fn is_dir(&self) -> bool {
        self.read_disk_inode(|disk_inode| {
            disk_inode.is_dir()
        })
    }

    #[inline]
    pub fn is_file(&self) -> bool {
        self.read_disk_inode(|disk_inode| {
            disk_inode.is_file()
        })
    }
    
    pub fn parent_inode(&self) -> Self {
        // 先得到父目录的ino
        let parent_ino = self.read_disk_inode(
            |dis_inode| {dis_inode.parent_ino});
        let (block_id, block_offset) = self.fs.lock().get_disk_inode_pos(parent_ino);
        // release efs lock
        Inode::new(
            block_id,
            block_offset,
            self.fs.clone(),
            self.block_device.clone(),
        )
    }
}

pub type FileType = DiskInodeType;

pub struct OSStat {
    pub size: u32,
    pub ino: u32,
    pub f_type: FileType,
    pub nlink: u32,
}