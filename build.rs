// Guillaume Valadon <guillaume@valadon.net>
// binutils - build.rs

use std::env;
use std::ffi;
use std::fs::File;
use std::io::Read;
use std::path;
use std::process;

extern crate cc;

fn execute_command(command: &str, arguments: Vec<&str>) {
    // Execute a command, and panic on any error

    let status = process::Command::new(command).args(arguments).status();
    match status {
        Ok(exit) => match exit.success() {
            true => (),
            false => panic!(
                "\n\n  \
                 Error '{}' exited with code {}\n\n",
                command,
                exit.code().unwrap()
            ),
        },
        Err(e) => panic!(
            "\n\n  \
             Error with '{}': {}\n\n",
            command, e
        ),
    };
}

fn change_dir(directory: &str) {
    // Go to another directory, and panic on error

    if !env::set_current_dir(directory).is_ok() {
        panic!(
            "\n\n  \
             Can't change dir to {}' !\n\n",
            directory
        );
    }
}

fn git_checkout_commit(repo: &git2::Repository, commit: &str) {
    let c = repo.find_commit(git2::Oid::from_str(commit).unwrap()).unwrap();
    repo.set_head_detached(c.id()).unwrap();
    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force())).unwrap();
}

fn build_binutils(version: &str, sha256sum: &str, output_directory: &str, targets: &str) {
    // Build binutils from source

    let binutils_name = format!("binutils");

    // Check if binutils is already built
    if path::Path::new(&format!("{}/built/", output_directory)).exists() {
        return;
    }

    change_dir(output_directory);
    let repo = if !path::Path::new(&binutils_name).exists() {
        git2::Repository::clone("https://github.com/itszor/binutils-vc4.git", &binutils_name).unwrap()
    } else {
        git2::Repository::open(&binutils_name).unwrap()
    };

    git_checkout_commit(&repo, "708acc851880dbeda1dd18aca4fd0a95b2573b36");

    // Calls commands to build binutils
    if path::Path::new(&binutils_name).exists() {
        change_dir(&binutils_name);
        let prefix_arg = format!("--prefix={}/built/", output_directory);
        std::env::set_var("CFLAGS", "-fPIC");
        execute_command(
            "./configure",
            vec![&prefix_arg, "--target=vc4-elf", "--disable-werror"],
        );
        execute_command("make", vec!["-j8"]);
        execute_command("make", vec!["install"]);

        std::fs::create_dir_all(&format!("{}/built/include/", output_directory));

        // Copy useful files
        execute_command(
            "cp",
            vec![
                "opcodes/config.h",
                &format!("{}/built/include/", output_directory),
            ],
        );
        execute_command(
            "cp",
            vec![
                "libiberty/libiberty.a",
                &format!("{}/built/lib/", output_directory),
            ],
        );
    }
}

fn main() {
    let version = "2.29.1";
    let sha256 = "0d9d2bbf71e17903f26a676e7fba7c200e581c84b8f2f43e72d875d0e638771c";

    // Retrieve targets to build
    let targets_var = match env::var_os("TARGETS") {
        Some(dir) => dir,
        None => ffi::OsString::from("all"),
    };
    let targets = targets_var
        .as_os_str()
        .to_str()
        .expect("Invalid TARGETS content!");

    // Get the current working directory
    let current_dir = env::current_dir().unwrap();

    // Where binutils will be built
    let out_directory = std::env::var("OUT_DIR").unwrap();

    // Build binutils
    build_binutils(version, sha256, &out_directory, targets);

    // Build our C helpers
    change_dir(current_dir.to_str().unwrap());
    cc::Build::new()
        .file("src/helpers.c")
        .include(format!("{}/built/include/", out_directory))
        .compile("helpers");

    // Locally compiled binutils libraries path
    println!(
        "cargo:rustc-link-search=native={}",
        format!("{}/built/lib/", out_directory)
    );
    println!("cargo:rustc-link-lib=static=bfd");
    println!("cargo:rustc-link-lib=static=opcodes");
    println!("cargo:rustc-link-lib=static=iberty");

    // Link to zlib
    println!("cargo:rustc-link-search=native=/usr/lib/"); // Arch Linux
    println!("cargo:rustc-link-search=native=/usr/lib/x86_64-linux-gnu/"); // Debian based
    println!("cargo:rustc-link-lib=static=z");
}
