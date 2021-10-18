use std::fs::{read_dir, File};

use std::io::{Result, Write};

/*
    build.rs 被 cargo build 使用
*/
fn main() {
    insert_app_data().unwrap();
}

static TARGET_PATH: &str = "../user/target/riscv64gc-unknown-none-elf/release";

fn insert_app_data() -> Result<()> {
    let mut f = File::create("src/link_app.S").unwrap(); // build 出错直接 panic
    let mut apps: Vec<_> = read_dir("../user/src/bin")
        .unwrap()
        .into_iter()
        .map(|dir_entry| {
            let mut name_with_ext = dir_entry.unwrap().file_name().into_string().unwrap();
            name_with_ext.drain(name_with_ext.find('.').unwrap()..name_with_ext.len());
            name_with_ext
        })
        .collect();
    apps.sort();

    //
    writeln!(
        f,
        r#"
    .align 3 # 使用 1<<3(8) 字节指令对齐，xmas-elf解析 需要按8字节进行对齐
    .section .data
    .global _num_app
_num_app:
    .quad {}"#,
        apps.len()
    )?;

    for i in 0..apps.len() {
        writeln!(f, r#"    .quad app_{}_start"#, i)?;
    }
    writeln!(f, r#"    .quad app_{}_end"#, apps.len() - 1)?;
    writeln!(f, r#"
    .global _app_names
_app_names:"#)?;
    writeln!(f, r#"    # 链接器会自动在每个字符串的结尾加入分隔符 \0"#)?;
    for app in apps.iter() {
        writeln!(f, r#"    .string "{}""#, app)?;
    }
    for (idx, app) in apps.iter().enumerate() {
        println!("app_{}: {}", idx, app);
        writeln!(
            f,
            r#"
    .section .data
    .global app_{0}_start
    .global app_{0}_end
app_{0}_start:
    .incbin "{2}/{1}"
app_{0}_end:
        "#, /*  */
            idx, app, TARGET_PATH
        )?;
    }
    return Ok(());
}
