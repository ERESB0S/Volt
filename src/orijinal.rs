// Copyright 2026 Eresbos (github.com/ERESB0S)
// SPDX-License-Identifier: GPL-3.0-or-later
//! Klavyenin orijinal (Monster) yazılımını bulur.
//!
//! Volt bu dosyayı kendi içinde taşımaz. Monster'ın destek sitesinde herkese açık olan sürücü paketini
//! (pusatk3driver.zip) indirir, içindeki "Upgrade Package.exe"nin kaynaklarından (RCDATA 4000) 64 KB'lık
//! yazılımı çıkarır ve parmak izini (SHA-256) bilinen orijinal dosyayla karşılaştırır.
//! Bir kez bulunan dosya bu bilgisayarda %LOCALAPPDATA%\Volt\orijinal_k3pro.bin olarak saklanır.

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const PAKET_ADRESI: &str = "https://mnstr-support-files.s3.eu-central-1.amazonaws.com/Drivers/Pusat/pusatk3driver.zip";
/// Monster'ın 2021-12-28 tarihli K3 Pro yazılımı (0C45:5004, sürüm 1.00)
const PARMAK_IZI: &str = "f5c413582e856dc79932d4bd818c43b37a17559456e217c9fb24f8e2d6d95066";
const PENCERESIZ: u32 = 0x0800_0000;

fn sha(v: &[u8]) -> String {
    Sha256::digest(v).iter().map(|b| format!("{b:02x}")).collect()
}

fn dogru(v: &[u8]) -> bool {
    v.len() == 64 * 1024 && sha(v) == PARMAK_IZI
}

fn veri_klasoru() -> Option<PathBuf> {
    std::env::var("LOCALAPPDATA").ok().map(|d| PathBuf::from(d).join("Volt"))
}

fn saklanan_yol() -> Option<PathBuf> {
    veri_klasoru().map(|d| d.join("orijinal_k3pro.bin"))
}

/// Orijinal yazılımı döndürür. `durum` ne yapıldığını anlatan kısa yazılarla çağrılır.
pub fn yazilim(durum: impl Fn(&str)) -> Result<Vec<u8>> {
    // 1) Daha önce bulunduysa
    if let Some(y) = saklanan_yol() {
        if let Ok(v) = std::fs::read(&y) {
            if dogru(&v) {
                return Ok(v);
            }
        }
    }
    let gecici = std::env::temp_dir().join(format!("volt_orijinal_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&gecici);
    std::fs::create_dir_all(&gecici).context("Geçici klasör oluşturulamadı")?;
    let sonuc = (|| -> Result<Vec<u8>> {
        // 2) Kullanıcı paketi kendisi indirdiyse (İndirilenler klasörü)
        let indirilenler = std::env::var("USERPROFILE").ok().map(|d| PathBuf::from(d).join("Downloads"));
        let mut paket = None;
        if let Some(d) = &indirilenler {
            for ad in ["Upgrade Package.exe", "pusatk3driver.zip"] {
                if d.join(ad).is_file() {
                    paket = Some(d.join(ad));
                    break;
                }
            }
        }
        // 3) Monster'ın sitesinden indir
        let paket = match paket {
            Some(p) => p,
            None => {
                durum("Orijinal yazılım Monster'ın sitesinden indiriliyor…");
                let zip = gecici.join("pusatk3driver.zip");
                let c = Command::new("curl.exe")
                    .args(["-sSfL", "--max-time", "180", "-o"])
                    .arg(&zip)
                    .arg(PAKET_ADRESI)
                    .creation_flags(PENCERESIZ)
                    .output()
                    .context("İndirme başlatılamadı (curl.exe bulunamadı)")?;
                if !c.status.success() || !zip.is_file() {
                    bail!(
                        "Orijinal yazılım Monster'ın sitesinden indirilemedi. İnternet bağlantını kontrol et. \
                         Olmazsa Monster'ın destek sayfasından PUSAT K3 sürücü paketini (pusatk3driver.zip) indirip \
                         İndirilenler klasörüne koy ve tekrar dene."
                    );
                }
                zip
            }
        };
        durum("Orijinal yazılım paketten çıkarılıyor…");
        let v = paketten(&paket, &gecici)?;
        if let Some(y) = saklanan_yol() {
            let _ = std::fs::create_dir_all(y.parent().unwrap());
            let _ = std::fs::write(&y, &v);
        }
        Ok(v)
    })();
    let _ = std::fs::remove_dir_all(&gecici);
    sonuc
}


/// Monster paketinden (zip ya da Upgrade Package.exe) orijinal yazılımı çıkarır ve doğrular
pub fn paketten(paket: &Path, gecici: &Path) -> Result<Vec<u8>> {
    let exe = if paket.extension().is_some_and(|u| u.eq_ignore_ascii_case("zip")) {
        let cik = gecici.join("paket");
        std::fs::create_dir_all(&cik)?;
        let c = Command::new("tar.exe").arg("-xf").arg(&paket).arg("-C").arg(&cik).creation_flags(PENCERESIZ).status();
        if !c.is_ok_and(|s| s.success()) {
            bail!("Monster paketi açılamadı");
        }
        bul(&cik, "upgrade package.exe").context("Monster paketinde güncelleme aracı (Upgrade Package.exe) bulunamadı")?
    } else {
        paket.to_path_buf()
    };
    let v = kaynaktan_cikar(&exe)?;
    if !dogru(&v) {
        bail!("Paketteki yazılım beklenen orijinal yazılım değil (parmak izi tutmadı). Güvenlik için yüklenmedi.");
    }
    Ok(v)
}

/// Klasörde (alt klasörler dahil) adı verilen dosyayı arar (küçük/büyük harf fark etmez)
fn bul(kok: &Path, ad: &str) -> Option<PathBuf> {
    for g in std::fs::read_dir(kok).ok()?.flatten() {
        let y = g.path();
        if y.is_dir() {
            if let Some(b) = bul(&y, ad) {
                return Some(b);
            }
        } else if g.file_name().to_string_lossy().to_lowercase() == ad {
            return Some(y);
        }
    }
    None
}

/// Upgrade Package.exe'nin içindeki RCDATA 4000 kaynağı (exe çalıştırılmaz, sadece veri olarak okunur)
fn kaynaktan_cikar(exe: &Path) -> Result<Vec<u8>> {
    use windows::Win32::System::LibraryLoader::{
        FindResourceW, LOAD_LIBRARY_AS_DATAFILE, LOAD_LIBRARY_AS_IMAGE_RESOURCE, LoadLibraryExW, LoadResource, LockResource, SizeofResource,
    };
    use windows::Win32::Foundation::FreeLibrary;
    use windows::core::{HSTRING, PCWSTR};
    unsafe {
        let m = LoadLibraryExW(&HSTRING::from(exe.as_os_str()), None, LOAD_LIBRARY_AS_DATAFILE | LOAD_LIBRARY_AS_IMAGE_RESOURCE)
            .context("Güncelleme aracı okunamadı")?;
        let sonuc = (|| {
            let k = FindResourceW(Some(m), PCWSTR(4000 as *const u16), PCWSTR(10 as *const u16));
            if k.is_invalid() {
                bail!("Güncelleme aracında yazılım bulunamadı");
            }
            let boy = SizeofResource(Some(m), k) as usize;
            let y = LoadResource(Some(m), k).context("Yazılım kaynağı yüklenemedi")?;
            let p = LockResource(y) as *const u8;
            if p.is_null() || boy == 0 {
                bail!("Yazılım kaynağı boş");
            }
            Ok(std::slice::from_raw_parts(p, boy).to_vec())
        })();
        let _ = FreeLibrary(m);
        sonuc
    }
}
