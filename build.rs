use std::{env, path::{Path, PathBuf}, process::{self, Command}};

fn main() {
    let target = env::var("TARGET").expect("TARGET not set");
    if Path::new(&target)
        .file_stem()
        .expect("target has no file stem")
        != "x86_64-bootloader"
    {
        return;
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let kernel = PathBuf::from(match env::var("KERNEL") {
        Ok(kernel) => kernel,
        Err(_) => {
            eprintln!(
                "The KERNEL environment variable must be set for building the bootloader."
            );
            process::exit(1);
        }
    });
    let kernel_file_name = kernel.file_name().expect("KERNEL has no valid file name").to_str().expect("kernel file name not valid utf8");
    let kernel_out_path = out_dir.join(format!("kernel_bin-{}.o", kernel_file_name));
    let kernel_archive_path = out_dir.join(format!("libkernel_bin-{}.a", kernel_file_name));

    let llvm_tools = match llvm_tools::LlvmTools::new() {
        Ok(tools) => tools,
        Err(llvm_tools::Error::NotFound) => {
            eprintln!("Error: llvm-tools not found");
            eprintln!("Maybe the rustup component `llvm-tools-preview` is missing?");
            eprintln!("  Install it through: `rustup component add llvm-tools-preview`");
            process::exit(1);
        }
        Err(err) => {
            eprintln!("Failed to retrieve llvm-tools component: {:?}", err);
            process::exit(1);
        }
    };
    let objcopy = llvm_tools.tool(&llvm_tools::exe("llvm-objcopy")).expect("llvm-objcopy not found in llvm-tools");

    let mut cmd = Command::new(objcopy);
    cmd.arg("-I").arg("binary");
    cmd.arg("-O").arg("elf64-x86-64");
    cmd.arg("--binary-architecture=i386:x86-64");
    cmd.arg("--rename-section").arg(".data=.kernel");
    cmd.arg("--redefine-sym").arg(format!("_binary_{}_start=_kernel_start_addr", kernel_file_name));
    cmd.arg("--redefine-sym").arg(format!("_binary_{}_end=_kernel_end_addr", kernel_file_name));
    cmd.arg("--redefine-sym").arg(format!("_binary_{}_size=_kernel_size", kernel_file_name));
    cmd.current_dir(kernel.parent().expect("KERNEL has no valid parent dir"));
    cmd.arg(&kernel_file_name);
    cmd.arg(&kernel_out_path);
    let exit_status = cmd.status().expect("failed to run objcopy");
    if !exit_status.success() {
        eprintln!("Error: Running objcopy failed");
        process::exit(1);
    }

    let ar = llvm_tools.tool(&llvm_tools::exe("llvm-ar")).unwrap_or_else(|| {
        eprintln!("Failed to retrieve llvm-ar component");
        eprint!("This component is available since nightly-XXXX-XX-XX,");
        eprintln!("so try updating your toolchain if you're using an older nightly");
        process::exit(1);
    });
    let mut cmd = Command::new(ar);
    cmd.arg("crs");
    cmd.arg(&kernel_archive_path);
    cmd.arg(&kernel_out_path);
    let exit_status = cmd.status().expect("failed to run ar");
    if !exit_status.success() {
        eprintln!("Error: Running ar failed");
        process::exit(1);
    }

    println!("cargo:rerun-if-changed={}", kernel.display());
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=kernel_bin-{}", kernel_file_name);
}
