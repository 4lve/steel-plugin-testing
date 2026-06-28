use std::{env, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    let Ok(out_dir) = env::var("OUT_DIR") else {
        return;
    };
    let profile_dir = PathBuf::from(out_dir)
        .ancestors()
        .nth(3)
        .expect("OUT_DIR should be inside target/<profile>/build/<package>/out")
        .to_path_buf();

    println!("cargo:rustc-link-search=native={}", profile_dir.display());
    println!(
        "cargo:rustc-link-search=native={}",
        profile_dir.join("deps").display()
    );

    let target = env::var("TARGET").unwrap_or_default();
    if target.contains("apple") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,@executable_path/deps");
    } else if target.contains("linux") || target.contains("bsd") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/deps");
    }
}
