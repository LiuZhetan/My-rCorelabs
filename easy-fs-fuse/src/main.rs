use easy_fs::{
    BlockDevice,
    EasyFileSystem,
};
use std::fs::{File, OpenOptions, read_dir};
use std::io::{Read, Write, Seek, SeekFrom};
use std::sync::Mutex;
use std::sync::Arc;
use clap::{Arg, App};

const BLOCK_SZ: usize = 512;

struct BlockFile(Mutex<File>);

impl BlockDevice for BlockFile {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        let mut file = self.0.lock().unwrap();
        file.seek(SeekFrom::Start((block_id * BLOCK_SZ) as u64))
            .expect("Error when seeking!");
        assert_eq!(file.read(buf).unwrap(), BLOCK_SZ, "Not a complete block!");
    }

    fn write_block(&self, block_id: usize, buf: &[u8]) {
        let mut file = self.0.lock().unwrap();
        file.seek(SeekFrom::Start((block_id * BLOCK_SZ) as u64))
            .expect("Error when seeking!");
        assert_eq!(file.write(buf).unwrap(), BLOCK_SZ, "Not a complete block!");
    }
}

fn main() {
    easy_fs_pack().expect("Error when packing easy-fs!");
}

fn easy_fs_pack() -> std::io::Result<()> {
    let matches = App::new("EasyFileSystem packer")
        .arg(Arg::with_name("source")
            .short("s")
            .long("source")
            .takes_value(true)
            .help("Executable source dir(with backslash)")
        )
        .arg(Arg::with_name("target")
            .short("t")
            .long("target")
            .takes_value(true)
            .help("Executable target dir(with backslash)")    
        )
        .get_matches();
    let src_path = matches.value_of("source").unwrap();
    let target_path = matches.value_of("target").unwrap();
    println!("src_path = {}\ntarget_path = {}", src_path, target_path);
    let block_file = Arc::new(BlockFile(Mutex::new({
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(format!("{}{}", target_path, "fs.img"))?;
        f.set_len(16 * 2048 * 512).unwrap();
        f
    })));
    // 16MiB, at most 4095 files
    let efs = EasyFileSystem::create(
        block_file.clone(),
        16 * 2048,
        1,
    );
    let root_inode = Arc::new(EasyFileSystem::root_inode(&efs));
    let apps: Vec<_> = read_dir(src_path)
        .unwrap()
        .into_iter()
        .map(|dir_entry| {
            let mut name_with_ext = dir_entry.unwrap().file_name().into_string().unwrap();
            name_with_ext.drain(name_with_ext.find('.').unwrap()..name_with_ext.len());
            name_with_ext
        })
        .collect();
    for app in apps {
        // load app data from host file system
        let mut host_file = File::open(format!("{}{}", target_path, app)).unwrap();
        let mut all_data: Vec<u8> = Vec::new();
        host_file.read_to_end(&mut all_data).unwrap();
        // create a file in easy-fs
        let inode = root_inode.create(app.as_str()).unwrap();
        // write data to easy-fs
        inode.write_at(0, all_data.as_slice());
    }
    // list apps
    for app in root_inode.ls() {
        println!("{}", app);
    }
    Ok(())
}

#[test]
fn efs_test() -> std::io::Result<()> {
    let block_file = Arc::new(BlockFile(Mutex::new({
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open("target/fs.img")?;
        f.set_len(8192 * 512).unwrap();
        f
    })));
    EasyFileSystem::create(
        block_file.clone(),
        4096,
        1,
    );
    let efs = EasyFileSystem::open(block_file.clone());
    let root_inode = EasyFileSystem::root_inode(&efs);
    root_inode.create("filea");
    root_inode.create("fileb");
    for name in root_inode.ls() {
        println!("{}", name);
    }
    let filea = root_inode.find("filea").unwrap();
    let greet_str = "Hello, world!";
    filea.write_at(0, greet_str.as_bytes());
    //let mut buffer = [0u8; 512];
    let mut buffer = [0u8; 233];
    let len = filea.read_at(0, &mut buffer);
    assert_eq!(
        greet_str,
        core::str::from_utf8(&buffer[..len]).unwrap(),
    );

    let mut random_str_test = |len: usize| {
        filea.clear();
        assert_eq!(
            filea.read_at(0, &mut buffer),
            0,
        );
        let mut str = String::new();
        use rand;
        // random digit
        for _ in 0..len {
            str.push(char::from('0' as u8 + rand::random::<u8>() % 10));
        }
        filea.write_at(0, str.as_bytes());
        let mut read_buffer = [0u8; 127];
        let mut offset = 0usize;
        let mut read_str = String::new();
        loop {
            let len = filea.read_at(offset, &mut read_buffer);
            if len == 0 {
                break;
            }
            offset += len;
            read_str.push_str(
                core::str::from_utf8(&read_buffer[..len]).unwrap()
            );
        }
        assert_eq!(str, read_str);
    };

    random_str_test(4 * BLOCK_SZ);
    random_str_test(8 * BLOCK_SZ + BLOCK_SZ / 2);
    random_str_test(100 * BLOCK_SZ);
    random_str_test(70 * BLOCK_SZ + BLOCK_SZ / 7);
    random_str_test((12 + 128) * BLOCK_SZ);
    random_str_test(400 * BLOCK_SZ);
    random_str_test(1000 * BLOCK_SZ);
    random_str_test(2000 * BLOCK_SZ);

    Ok(())
}

#[test]
fn link_test() -> std::io::Result<()> {
    let block_file = Arc::new(BlockFile(Mutex::new({
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open("target/fs.img")?;
        f.set_len(8192 * 512).unwrap();
        f
    })));
    EasyFileSystem::create(
        block_file.clone(),
        4096,
        1,
    );
    let efs = EasyFileSystem::open(block_file.clone());
    let root_inode = EasyFileSystem::root_inode(&efs);
    root_inode.create("filea");
    root_inode.create("fileb");
    for name in root_inode.ls() {
        println!("{}", name);
    }
    let filea = root_inode.find("filea").unwrap();
    let greet_str = "Hello, world!";
    filea.write_at(0, greet_str.as_bytes());
    //let mut buffer = [0u8; 512];
    let mut buffer = [0u8; 233];
    let len = filea.read_at(0, &mut buffer);
    assert_eq!(
        greet_str,
        core::str::from_utf8(&buffer[..len]).unwrap(),
    );

    // 将filec硬链接到filea，读取filec，能够读到filea的内容
    let inode_number = root_inode.
        find_inode_number("filea")
        .expect("not a file")
        .unwrap();
    if root_inode.link("filec",inode_number).is_none() {
        panic!("file already exist");
    }
    let filec = root_inode.find("filec").unwrap();
    let len = filec.read_at(0, &mut buffer);
    let read_str = core::str::from_utf8(&buffer[..len]).unwrap();
    assert_eq!(
        greet_str,
        read_str,
    );
    println!("test1 ok!");

    // filea向文件写入,filec能读取到更新的内容
    let add_str = " add by test filea";
    filea.write_at(len, add_str.as_bytes());
    let len = filec.read_at(0, &mut buffer);
    let read_str = core::str::from_utf8(&buffer[..len]).unwrap();
    assert_eq!(
        format!("{}{}",greet_str,add_str),
        read_str,
    );
    println!("test2 ok!");


    // 现在将filec unlink, 查看filea的引用计数和根目录的目录项
    let (_,_, nlink) = filea.fstat();
    println!("Before unlink the nlink of filea is {}",nlink);
    root_inode.remove_file("filec");
    let filea = root_inode.find("filea").unwrap();
    let (_,_, nlink) = filea.fstat();
    println!("After unlink the nlink of filea is {}",nlink);
    assert_eq!(1,nlink);
    //显示根目录的文件
    println!("files under root:");
    let files = root_inode.ls();
    for file in files {
        println!("{}",file);
    }
    println!("test1 ok!");

    // 测试fallocate
    filea.fallocate(0,7);
    let header_str = "HEADER:";
    filea.write_at(0, header_str.as_bytes());
    filea.fallocate(0,1025);
    let len = filea.read_at(1025, &mut buffer);
    let read_str = core::str::from_utf8(&buffer[..len]).unwrap();
    assert_eq!(
        format!("{}{}{}", header_str, greet_str, add_str),
        read_str,
    );
    println!("test4 ok!");

    //测试fdeallocate
    filea.fdeallocate(0,1020);
    filea.write_at(0, "TEST:".as_bytes());
    let len = filea.read_at(0, &mut buffer);
    let read_str = core::str::from_utf8(&buffer[..len]).unwrap();
    assert_eq!(
        format!("{}{}{}{}", "TEST:", header_str, greet_str, add_str),
        read_str,
    );
    println!("test5 ok!");

    //测试多重目录的增加和删除功能
    root_inode.create_dir("./sub_dir1").expect("Unable to create dir");
    let files = root_inode.ls();
    for file in files {
        println!("{}",file);
    }
    let sub_dir1 = root_inode.find("sub_dir1").unwrap();
    let (ino, is_file,nlink) = sub_dir1.fstat();
    println!("successfully create file sub_dir1:");
    println!("ino:{}",ino);
    println!("is_file:{}",is_file);
    println!("nlink:{}",nlink);
    sub_dir1.create("filed");
    println!("now file in sub_dir1:");
    let files = sub_dir1.ls();
    for file in files {
        println!("{}",file);
    }
    let filed = sub_dir1.find("filed").unwrap();
    filed.write_at(0,"hello world".as_bytes());
    let len = filed.read_at(0, &mut buffer);
    let read_str = core::str::from_utf8(&buffer[..len]).unwrap();
    assert_eq!("hello world", read_str);

    println!("remove filed");
    assert_eq!(true,sub_dir1.remove_file("filed").is_ok());
    assert_eq!(sub_dir1.ls().len(),0);
    println!("remove sub_dir1");
    assert_eq!(true,root_inode.remove_dir("sub_dir1").is_ok());
    println!("now root directory:");
    let files = root_inode.ls();
    for file in files {
        println!("{}",file);
    }
    println!("test6 ok!");
    Ok(())
}