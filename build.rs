#![feature(path_try_exists)]

use std::fmt::Debug;
use std::fs;
use std::env;
use std::path::PathBuf;
use std::process::Command;
use std::str::FromStr;

use cc;

fn main() {
    // Gather any env vars we need
    let is_debug  = get_env_var::<bool, _>("DEBUG").unwrap_or(false);
    let is_secure = get_env_var::<String, _>("CARGO_FEATURE_SECURE").is_ok();
    let debug     = if is_debug { "Debug" } else { "Release" };
    let secure    = if is_secure { "ON" } else { "OFF" };

    let out_dir = get_env_var::<PathBuf, _>("OUT_DIR")
        .expect("Failed to get the output directory");

    let lib_path = out_dir.join("mimalloc");
    
    // Clone the configured version of mi-malloc if not already present
    if !fs::try_exists(&lib_path).unwrap_or(false) {
        let output = Command::new("git")
            .arg("clone")
            .arg("-b")
            .arg("v1.7.2")
            .arg("--single-branch")
            .arg("https://github.com/microsoft/mimalloc.git")
            .arg(lib_path.display().to_string())
            .output()
            .expect("Failed to find git on the system. Install git to build this project.");

        if !output.status.success() {
            panic!(
                "Failed to clone mi-malloc repository with error '{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    // Build the library
    cc::Build::new()
        .define("MAKE_BUILD_TYPE", debug)
        .define("MI_SECURE", secure)
        .define("MI_OVERRIDE", "OFF")
        .define("MI_OSX_ZONE", "OFF")
        .define("MI_BUILD_SHARED", "OFF")
        .define("MI_BUILD_TESTS", "OFF")
        .define("MI_BUILD_OBJECT", "OFF")
        .include(&lib_path)
        .compile("mimalloc");

    let search_path = lib_path.join(if is_debug { "build/Debug/" } else { "build/Release" });
    let lib_name    = match (is_debug, is_secure) {
        (true, true)   => "mimalloc-secure-static-debug",
        (true, false)  => "mimalloc-static-debug",
        (false, true)  => "mimalloc-secure-static",
        (false, false) => "mimalloc-static",
    };

    // Tell cargo to link the built library
    println!("cargo:rustc-link-search=native={}", search_path.display());
    println!("cargo:rustc-link-lib=static={}", lib_name);

    // Tell cargo when we need to re-run
    println!("cargo:rerun-if-changed=mimalloc");
}

fn get_env_var<'a, T, E>(key: &str) -> Result<T, env::VarError>
    where T: FromStr<Err=E>,
          E: Debug
{
    let str_var = env::var(key)?;
    let var     = match T::from_str(str_var.as_str()) {
        Ok(x)  => x,
        Err(e) => panic!("Failed to parse env var. value: '{}', error:'{}': {:?}", key, str_var, e)
    };

    Ok(var)
}