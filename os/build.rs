// 生成link_APP.S
use std::fs::{File, read_dir};
use std::io::{Result,Write};

const USER_APP_PATH: &str = "../user/target/riscv64gc-unknown-none-elf/release/";

fn main() {
    create_asm().unwrap();
}

fn create_asm() -> Result<()>  {
    let mut file = File::create("src/link_app.S").expect("Error: Fail to create src/link_app.S");
    let mut apps : Vec<String>= read_dir("../user/src/bin").unwrap()
        .map(| entry| -> String {
            /*
            let mut file_name = entry.unwrap().file_name().into_string().unwrap();
            let split:Vec<_> = file_name.split('.').collect();
            let result = String::from(split[0]);
            println!("split:{:?}",result);
            result*/
            let mut name_with_ext = entry.unwrap().file_name().into_string().unwrap();
            //println!("{}",name_with_ext);
            name_with_ext.drain(name_with_ext.find('.').unwrap()..name_with_ext.len());
            name_with_ext
        }).collect();
    apps.sort();
    println!("current apps: {:?}",apps);
    writeln!(
        file,
        "    .align 3
    .section .data
    .global _num_app
_num_app:
    .quad {}",
        apps.len()
    )?;
    for i in 0..apps.len() {
        writeln!(file, "    .quad app_{}_start",i)?;
    }
    writeln!(file, "    .quad app_{}_end",apps.len() - 1)?;

    for (i,app) in apps.iter().enumerate() {
        writeln!(file,"
    .section .data
    .global app_{i}_start
    .global app_{i}_end
app_{i}_start:
    .incbin \"{USER_APP_PATH}{app}.bin\"
app_{i}_end:")?
    }

    Ok(())
}
