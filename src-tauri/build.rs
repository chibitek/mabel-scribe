use std::path::Path;
use std::process::Command;

fn try_link_native_asr() {
    println!("cargo:rerun-if-changed=../native/MabelASR/Sources/MabelASR/MabelASR.swift");
    println!("cargo:rerun-if-changed=../native/MabelASR/Package.swift");
    println!("cargo:rerun-if-changed=../scripts/build-mabel-asr.sh");
    println!("cargo:rerun-if-env-changed=MABEL_SKIP_NATIVE_ASR");
    println!("cargo:rustc-check-cfg=cfg(mabel_native_asr)");

    if std::env::var("CARGO_CFG_TARGET_OS").ok().as_deref() != Some("macos") {
        return;
    }
    if std::env::var("MABEL_SKIP_NATIVE_ASR").ok().as_deref() == Some("1") {
        println!("cargo:warning=MabelASR skipped (MABEL_SKIP_NATIVE_ASR=1)");
        return;
    }

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let script = Path::new(&manifest_dir).join("../scripts/build-mabel-asr.sh");
    let status = Command::new("bash").arg(&script).status();
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => {
            println!(
                "cargo:warning=MabelASR build failed (exit {}). Parakeet/WhisperKit will be unavailable.",
                s.code().unwrap_or(-1)
            );
            return;
        }
        Err(e) => {
            println!("cargo:warning=could not run build-mabel-asr.sh: {}", e);
            return;
        }
    }

    let dylib = Path::new(&manifest_dir).join("native-asr/libMabelASR.dylib");
    if !dylib.exists() {
        println!("cargo:warning=libMabelASR.dylib not staged; native engines unlinked");
        return;
    }

    println!(
        "cargo:rustc-link-search=native={}",
        dylib.parent().unwrap().display()
    );
    println!("cargo:rustc-link-lib=dylib=MabelASR");
    println!("cargo:rustc-link-lib=framework=Foundation");
    println!("cargo:rustc-link-lib=framework=CoreML");
    println!("cargo:rustc-link-lib=framework=AVFoundation");
    println!("cargo:rustc-link-lib=framework=Accelerate");
    println!("cargo:rustc-link-lib=framework=AudioToolbox");
    println!("cargo:rustc-link-lib=framework=CoreAudio");
    println!("cargo:rustc-link-lib=framework=CoreMedia");
    println!("cargo:rustc-cfg=mabel_native_asr");
}

fn try_link_native_storekit() {
    println!("cargo:rerun-if-changed=../native/MabelStoreKit/Sources/MabelStoreKit/MabelStoreKit.swift");
    println!("cargo:rerun-if-changed=../native/MabelStoreKit/Package.swift");
    println!("cargo:rerun-if-changed=../scripts/build-mabel-storekit.sh");
    println!("cargo:rerun-if-env-changed=MABEL_SKIP_NATIVE_STOREKIT");
    println!("cargo:rustc-check-cfg=cfg(mabel_native_storekit)");

    if std::env::var("CARGO_CFG_TARGET_OS").ok().as_deref() != Some("macos") {
        return;
    }
    if std::env::var("MABEL_SKIP_NATIVE_STOREKIT").ok().as_deref() == Some("1") {
        println!("cargo:warning=MabelStoreKit skipped (MABEL_SKIP_NATIVE_STOREKIT=1)");
        return;
    }

    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let script = Path::new(&manifest_dir).join("../scripts/build-mabel-storekit.sh");
    let status = Command::new("bash").arg(&script).status();
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => {
            println!(
                "cargo:warning=MabelStoreKit build failed (exit {}). StoreKit 2 will be unavailable (fail-closed).",
                s.code().unwrap_or(-1)
            );
            return;
        }
        Err(e) => {
            println!("cargo:warning=could not run build-mabel-storekit.sh: {}", e);
            return;
        }
    }

    let dylib = Path::new(&manifest_dir).join("native-storekit/libMabelStoreKit.dylib");
    if !dylib.exists() {
        println!("cargo:warning=libMabelStoreKit.dylib not staged; StoreKit unlinked (fail-closed)");
        return;
    }

    println!(
        "cargo:rustc-link-search=native={}",
        dylib.parent().unwrap().display()
    );
    println!("cargo:rustc-link-lib=dylib=MabelStoreKit");
    println!("cargo:rustc-link-lib=framework=Foundation");
    println!("cargo:rustc-link-lib=framework=StoreKit");
    println!("cargo:rustc-link-lib=framework=AppKit");
    println!("cargo:rustc-cfg=mabel_native_storekit");
}

fn main() {
    let hash = Command::new("git")
        .args(["rev-parse", "--short=7", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string());

    let dirty = Command::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);

    println!("cargo:rustc-env=MABEL_GIT_HASH={}", hash);
    println!("cargo:rustc-env=MABEL_GIT_DIRTY={}", if dirty { "1" } else { "0" });
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/index");

    compile_mic_permission();
    try_link_native_asr();
    try_link_native_storekit();
    tauri_build::build()
}

fn compile_mic_permission() {
    println!("cargo:rerun-if-changed=src/mic_permission.m");
    if std::env::var("CARGO_CFG_TARGET_OS").ok().as_deref() != Some("macos") {
        return;
    }
    cc::Build::new()
        .file("src/mic_permission.m")
        .compile("mabel_mic_permission");
    println!("cargo:rustc-link-lib=framework=AVFoundation");
    println!("cargo:rustc-link-lib=framework=Foundation");
}
