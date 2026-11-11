// Copyright 2026 Eresbos (github.com/ERESB0S)
// SPDX-License-Identifier: GPL-3.0-or-later
//! Volt simgesini ve sürüm bilgisini exe'lere gömer (simge/*.rc → .res, Windows SDK'daki rc.exe ile).
//! rc.exe bulunamazsa uyarı verip simgesiz derler.

use std::path::PathBuf;
use std::process::Command;

fn rc_bul() -> Option<PathBuf> {
    if let Ok(y) = std::env::var("RC") {
        return Some(y.into());
    }
    let kok = PathBuf::from(r"C:\Program Files (x86)\Windows Kits\10\bin");
    let mut surumler: Vec<PathBuf> = std::fs::read_dir(&kok).ok()?.flatten().map(|d| d.path().join(r"x64\rc.exe")).filter(|p| p.exists()).collect();
    surumler.sort();
    surumler.pop()
}

fn main() {
    println!("cargo:rerun-if-changed=simge");
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc") {
        return;
    }
    let Some(rc) = rc_bul() else {
        println!("cargo:warning=rc.exe bulunamadı (Windows SDK); exe'ler simgesiz derlenecek");
        return;
    };
    let cikti = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    for (bin, ad) in [("pusat_isik", "volt"), ("volt_kurulum", "volt_kurulum")] {
        let res = cikti.join(format!("{ad}.res"));
        let tamam = Command::new(&rc)
            .args(["/nologo", "/fo"])
            .arg(&res)
            .arg(format!("simge/{ad}.rc"))
            .status()
            .map(|d| d.success())
            .unwrap_or(false);
        if tamam {
            println!("cargo:rustc-link-arg-bin={bin}={}", res.display());
        } else {
            println!("cargo:warning={ad}.rc derlenemedi; {bin} simgesiz olacak");
        }
    }
}
