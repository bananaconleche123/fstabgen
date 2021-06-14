use std::io::Write;

use std::process::Command;

use colored::*;
use fstab::{FsEntry, FsTab};
use question::{Answer, Question};

const FSTAB_PATH: &str = "/etc/fstab";

fn trim_newline(s: &mut String) {
    if s.ends_with('\n') {
        s.pop();
        if s.ends_with('\r') {
            s.pop();
        }
    }
}
fn get_from_dir<S: Into<String>>(dir: S) -> Vec<String> {
    let mut files: Vec<String> = Vec::new();
    let dir_str: String = dir.into();
    for file in std::fs::read_dir(std::path::Path::new(&dir_str)).unwrap() {
        let ft: String = file.unwrap().file_name().to_str().unwrap().into();
        files.push(ft);
    }
    files
}
fn get_fs() -> Vec<String> {
    let output = Command::new("uname")
        .arg("-r")
        .output()
        .expect("failed to execute uname -r");

    let mut uname: String = String::from_utf8_lossy(&output.stdout).to_string();
    trim_newline(&mut uname);

    let dir = format!("/lib/modules/{}/kernel/fs", uname);

    let mut filetypes: Vec<String> = get_from_dir(&dir);
    filetypes.sort();

    let common_fs_str = vec![
        "ext4", "xfs", "btrfs", "f2fs", "vfat", "ntfs", "hfsplus", "tmpfs", "sysfs", "proc",
        "iso9660", "udf", "squashfs", "nfs", "cifs", "none",
    ];
    let mut common_fs: Vec<String> = Vec::new();
    for a in common_fs_str {
        common_fs.push(a.to_string());
    }
    use itertools::Itertools;

    filetypes.splice(0..0, common_fs);
    let filetypes = filetypes.into_iter().unique().collect();

    filetypes
}

fn read_input<S: Into<String>>(s: S) -> String {
    print!("{}", s.into());
    std::io::stdout().flush();
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .expect("error: unable to read user input");
    trim_newline(&mut input);
    input
}

fn get_uuid_path<S: Into<String>>(uuid: S) -> String {
    let dir: String = format!("/dev/disk/by-uuid/{}", uuid.into());
    let sdx: String = std::fs::canonicalize(std::path::Path::new(&dir))
        .unwrap()
        .to_str()
        .unwrap()
        .into();
    sdx
}

fn main() {
    if !nix::unistd::geteuid().is_root() {
        eprintln!("{}", "Administrator permission are needed".red());
        std::process::exit(1);
    };
    let path = std::path::Path::new(FSTAB_PATH);
    if !path.exists() {
        eprintln!("{}", "The fstab file does not exists".red());
        std::process::exit(1);
    }

    let mut disks: Vec<(String, String)> = Vec::new();
    for disk in std::fs::read_dir(std::path::Path::new("/dev/disk/by-uuid")).unwrap() {
        let uuid: String = disk.unwrap().file_name().to_str().unwrap().into();
        let sdx: String = get_uuid_path(&uuid);
        disks.push((sdx, uuid));
    }
    disks.sort();

    let disks_info: Vec<String> = disks.iter().map(|x| format!("{} ({})", x.0, x.1)).collect();
    let mut menu1 = youchoose::Menu::new(disks_info.iter());
    let selected_disk = &disks[menu1.show()[0]];

    let filetypes = get_fs();
    let mut menu2 = youchoose::Menu::new(filetypes.iter());
    let vfs_type = filetypes[menu2.show()[0]].clone();

    let fs_spec = format!("UUID={}", selected_disk.1);
    
    let dir = read_input(format!("Mountpoint for {}: ", selected_disk.0));
    let mountpoint = std::path::Path::new(&dir);
    if !mountpoint.exists() {
        eprintln!("{}", "The path does not exists".red());
        std::process::exit(1);
    }

    let mount_options: Vec<String> = vec!["defaults".to_string()];
    let dump = false;
    let fsck_order = 2;

    let entry = FsEntry {
        fs_spec: fs_spec,
        mountpoint: mountpoint.to_path_buf(),
        vfs_type: vfs_type,
        mount_options: mount_options,
        dump: dump,
        fsck_order: fsck_order,
    };

    let line = format!(
        "{} {} {} {} {} {}",
        entry.fs_spec,
        entry.mountpoint.to_str().unwrap(),
        entry.vfs_type,
        entry.mount_options.join(","),
        entry.dump as u8,
        entry.fsck_order
    );
    let answer = Question::new(
        format!(
            "The following entry will be added to your fstab file:\n{}\nAre you sure?",
            line
        )
        .as_str(),
    )
    .default(Answer::NO)
    .show_defaults()
    .confirm();

    if answer == Answer::YES {
        println!("Adding entry...");
        std::fs::copy(FSTAB_PATH, format!("{}.bak", FSTAB_PATH)).unwrap();
        let fstab = FsTab::new(path);
        fstab.add_entry(entry).unwrap();
        println!("Backup saved at : {}.bak", FSTAB_PATH);
    } else {
        println!("Aborting...");
    }
}
