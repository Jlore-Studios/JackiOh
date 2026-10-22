//! One use of each path the pure crates' `clippy.toml` bans (CLAUDE.md rule 4). Nothing compiles it
//! here: `acceptance.rs`'s `v2_every_entry_of_the_pure_crates_clippy_toml_fires` writes it into a
//! scratch crate and runs clippy over it with `crates/engine/clippy.toml`, so an entry this file
//! does not use, or one clippy cannot resolve, fails that test. A new entry gets its use here.

use std::path::Path;

pub fn types() {
    let _: Option<std::collections::HashMap<u8, u8>> = None;
    let _: Option<std::collections::HashSet<u8>> = None;
    let _: Option<std::time::Instant> = None;
    let _: Option<std::time::SystemTime> = None;
    let _: Option<std::fs::File> = None;
    let _: Option<std::thread::Thread> = None;
    let _: Option<std::sync::Mutex<u8>> = None;
    let _: Option<std::sync::RwLock<u8>> = None;
    let _: Option<std::cell::RefCell<u8>> = None;
    let _: Option<std::hash::RandomState> = None;
    let _: Option<std::fs::OpenOptions> = None;
    let _: Option<std::fs::DirBuilder> = None;
    let _: Option<std::net::TcpListener> = None;
    let _: Option<std::net::TcpStream> = None;
    let _: Option<std::net::UdpSocket> = None;
    let _: Option<std::process::Command> = None;
    let _: Option<std::process::Child> = None;
    let _: Option<std::thread::Builder> = None;
}

pub fn clock_and_environment() {
    let _ = std::time::Instant::now();
    let _ = std::time::SystemTime::now();
    let _ = std::env::var("X");
    let _ = std::env::vars();
    let _ = std::env::args();
    let _ = std::env::args_os();
    let _ = std::env::var_os("X");
    let _ = std::env::vars_os();
    unsafe { std::env::set_var("X", "1") };
    unsafe { std::env::remove_var("X") };
    let _ = std::env::current_dir();
    let _ = std::env::set_current_dir("x");
    let _ = std::env::current_exe();
    let _ = std::env::temp_dir();
}

pub fn files(path: &Path) {
    let _ = std::fs::read(path);
    let _ = std::fs::read_to_string(path);
    let _ = std::fs::write(path, "x");
    let _ = std::fs::canonicalize(path);
    let _ = std::fs::copy(path, path);
    let _ = std::fs::create_dir(path);
    let _ = std::fs::create_dir_all(path);
    let _ = std::fs::exists(path);
    let _ = std::fs::hard_link(path, path);
    let _ = std::fs::read_dir(path);
    let _ = std::fs::read_link(path);
    let _ = std::fs::remove_dir(path);
    let _ = std::fs::remove_dir_all(path);
    let _ = std::fs::remove_file(path);
    let _ = std::fs::rename(path, path);
    let _ = std::fs::symlink_metadata(path);
    if let Ok(metadata) = std::fs::metadata(path) {
        let _ = std::fs::set_permissions(path, metadata.permissions());
    }
    let _ = path.canonicalize();
    let _ = path.exists();
    let _ = path.is_dir();
    let _ = path.is_file();
    let _ = path.is_symlink();
    let _ = path.metadata();
    let _ = path.read_dir();
    let _ = path.read_link();
    let _ = path.symlink_metadata();
    let _ = path.try_exists();
    let _ = std::io::stdin();
    let _ = std::io::stdout();
    let _ = std::io::stderr();
}

pub fn threads() {
    let _ = std::thread::spawn(|| {});
    let _ = std::thread::available_parallelism();
    let _ = std::thread::current();
    std::thread::park();
    std::thread::park_timeout(std::time::Duration::ZERO);
    std::thread::scope(|_| {});
    std::thread::sleep(std::time::Duration::ZERO);
}

pub fn process() {
    let _ = std::process::id();
}

pub fn exit() {
    std::process::exit(0)
}

pub fn abort() {
    std::process::abort()
}
