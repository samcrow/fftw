use anyhow::Result;
use std::env::var;
use std::fs::{canonicalize, File};
use std::io::{copy, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use zip::ZipArchive;

fn download_archive_windows(out_dir: &Path) -> Result<()> {
    if out_dir.join("libfftw3.dll").exists() && out_dir.join("libfftw3f.dll").exists() {
        return Ok(());
    }

    let archive = out_dir.join("fftw_windows.zip");
    if !archive.exists() {
        // Download
        let mut conn = ftp::FtpStream::connect("ftp.fftw.org:21")?;
        conn.login("anonymous", "anonymous")?;
        conn.cwd("pub/fftw")?;
        let buf = conn.simple_retr("fftw-3.3.5-dll64.zip")?.into_inner();
        // TODO calc checksum
        let mut f = File::create(&archive)?;
        f.write(&buf)?;
    }
    let f = File::open(&archive)?;
    let mut zip = ZipArchive::new(f)?;
    let target = var("TARGET").unwrap();
    for name in &["fftw3-3", "fftw3f-3"] {
        for ext in &["dll", "def"] {
            let filename = format!("lib{}.{}", name, ext);
            let mut zf = zip.by_name(&filename)?;
            let mut f = File::create(out_dir.join(filename))?;
            copy(&mut zf, &mut f)?;
        }
        run(cc::windows_registry::find_tool(&target, "lib.exe")
            .unwrap()
            .to_command()
            .arg("/MACHINE:X64")
            .arg(format!("/DEF:lib{}.def", name))
            .arg(format!("/OUT:lib{}.lib", name))
            .current_dir(out_dir))
    }
    Ok(())
}

fn build_unix(out_dir: &Path) {
    let src_dir = PathBuf::from(var("CARGO_MANIFEST_DIR").unwrap()).join("fftw-3.3.8");
    let out_src_dir = out_dir.join("src");
    fs_extra::dir::copy(
        src_dir,
        &out_src_dir,
        &fs_extra::dir::CopyOptions {
            overwrite: true,
            skip_exist: false,
            buffer_size: 64000,
            copy_inside: true,
            depth: 0,
            content_only: false,
        },
    )
    .unwrap();
    if !out_dir.join("lib/libfftw3.a").exists() {
        build_fftw(&[], &out_src_dir, &out_dir);
    }
    if !out_dir.join("lib/libfftw3f.a").exists() {
        build_fftw(&["--enable-single"], &out_src_dir, &out_dir);
    }
}

fn build_fftw(flags: &[&str], src_dir: &Path, out_dir: &Path) {
    run(
        Command::new(canonicalize(src_dir.join("configure")).unwrap())
            .arg("--with-pic")
            .arg("--enable-static")
            .arg("--disable-doc")
            .arg(format!("--prefix={}", out_dir.display()))
            .args(flags)
            .current_dir(&src_dir),
    );
    run(Command::new("make")
        .arg(format!("-j{}", var("NUM_JOBS").unwrap()))
        .current_dir(&src_dir));
    run(Command::new("make").arg("install").current_dir(&src_dir));


const FFTW: &'static str = "fftw-3.3.6-pl1";
const ARCHIVE: &'static str = "fftw-3.3.6-pl1.tar.gz";
const URI: &'static str = "http://www.fftw.org/fftw-3.3.6-pl1.tar.gz";
const MD5SUM: u128 = 0x682a0e78d6966ca37c7446d4ab4cc2a1;

fn correct_sum() -> [u8; 16] {
    let mut bytes = unsafe { ::std::mem::transmute::<u128, [u8; 16]>(MD5SUM) };
    bytes.reverse();
    bytes
}

/// Converts a Rust target triple into an autotools target triple that can be used to cross-compile
/// FFTW
fn rust_target_to_fftw_target(target: &str) -> &'static str {
    match target {
        "armv7-unknown-linux-gnueabihf" => "arm-linux-gnueabihf",
        _ => panic!("Unsupported target {}", target),
    }
}

fn main() -> Result<()> {
    let out_dir = PathBuf::from(var("OUT_DIR").unwrap());
    let archive_path = out_dir.join(ARCHIVE);
    let src_dir = out_dir.join(FFTW);

    let host = var("HOST").unwrap();
    let target = var("TARGET").unwrap();

    if !archive_path.exists() {
        download(URI, ARCHIVE, &out_dir);
    }
    if check_sum(&archive_path)? != correct_sum() {
        panic!("check sum of archive is incorrect");
    }
    expand(&archive_path, &out_dir);

    if host == target {
        build_fftw(
            &["--enable-static", "--with-pic", "--enable-single"],
            &src_dir,
            &out_dir,
        );
        build_fftw(&["--enable-static", "--with-pic"], &src_dir, &out_dir);
    } else {
        let fftw_target = rust_target_to_fftw_target(&target);
        build_fftw(
            &["--enable-static", "--with-pic", "--enable-single", "--host", &fftw_target],
            &src_dir,
            &out_dir,
        );
        build_fftw(&["--enable-static", "--with-pic", "--host", &fftw_target], &src_dir, &out_dir);
    }

    println!(
        "cargo:rustc-link-search={}",
        out_dir.join("usr/local/lib").display()
    );

    println!("cargo:rustc-link-lib=static=fftw3");
    println!("cargo:rustc-link-lib=static=fftw3f");

    Ok(())
>>>>>>> 6fd3ba3... Added cross-compilation support for armv7-unknown-linux-gnueabihf
}

fn run(command: &mut Command) {
    println!("Running: {:?}", command);
    match command.status() {
        Ok(status) => {
            if !status.success() {
                panic!("`{:?}` failed: {}", command, status);
            }
        }
        Err(error) => {
            panic!("failed to execute `{:?}`: {}", command, error);
        }
    }
}

fn main() {
    let out_dir = PathBuf::from(var("OUT_DIR").unwrap());
    if cfg!(target_os = "windows") {
        download_archive_windows(&out_dir).unwrap();
        println!("cargo:rustc-link-search={}", out_dir.display());
        println!("cargo:rustc-link-lib=libfftw3-3");
        println!("cargo:rustc-link-lib=libfftw3f-3");
    } else {
        build_unix(&out_dir);
        println!("cargo:rustc-link-search={}", out_dir.join("lib").display());
        println!("cargo:rustc-link-lib=static=fftw3");
        println!("cargo:rustc-link-lib=static=fftw3f");
    }
}
