// Copyright 2026 Eresbos (github.com/ERESB0S)
// SPDX-License-Identifier: GPL-3.0-or-later
//! Volt'u bilgisayara kurma ve kaldırma (yönetici izni gerekmez; sadece bu kullanıcı için kurulur).
//!
//! Kurulum yeri  : %LOCALAPPDATA%\Programs\Volt\Volt.exe
//! Kısayollar    : Başlat menüsü (ve istenirse masaüstü)
//! Windows listesi: HKCU\...\Uninstall\Volt ("Uygulamalar" listesinden kaldırma "Volt.exe --kaldir" ile yapılır)

use anyhow::{Context, Result, bail};
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

pub const SURUM: &str = "1.0.1";
const KALDIR_ANAHTAR: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\Volt";
const RUN_ANAHTAR: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
const PENCERESIZ: u32 = 0x0800_0000;

pub fn klasor() -> PathBuf {
    PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_default()).join("Programs").join("Volt")
}

pub fn exe_yolu() -> PathBuf {
    klasor().join("Volt.exe")
}

/// Kurulu mu (Windows listesindeki kayıt ve dosya)
pub fn kurulu() -> bool {
    exe_yolu().is_file()
}

/// Çalışan bu exe, kurulum klasöründeki mi?
pub fn bu_kurulu() -> bool {
    let a = std::env::current_exe().ok().and_then(|y| y.canonicalize().ok());
    let b = exe_yolu().canonicalize().ok();
    a.is_some() && a == b
}

fn reg(args: &[&str]) -> bool {
    Command::new("reg").args(args).creation_flags(PENCERESIZ).status().is_ok_and(|s| s.success())
}

/// Kısa PowerShell betiği; yollar ortam değişkeniyle verilir (tırnak sorunu olmasın)
fn ps(betik: &str, ortam: &[(&str, String)]) -> bool {
    let mut c = Command::new("powershell.exe");
    c.args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", betik]).creation_flags(PENCERESIZ);
    for (k, v) in ortam {
        c.env(k, v);
    }
    c.status().is_ok_and(|s| s.success())
}

const KISAYOL_BETIK: &str = r#"
$s = New-Object -ComObject WScript.Shell
$yerler = @([Environment]::GetFolderPath('Programs'))
if ($env:VOLT_MASAUSTU -eq '1') { $yerler += [Environment]::GetFolderPath('Desktop') }
foreach ($y in $yerler) {
  $k = $s.CreateShortcut((Join-Path $y 'Volt.lnk'))
  $k.TargetPath = $env:VOLT_EXE
  $k.WorkingDirectory = Split-Path $env:VOLT_EXE
  $k.IconLocation = $env:VOLT_EXE + ',0'
  $k.Description = 'Volt - klavye ışıkları ve ayarları'
  $k.Save()
}
"#;

const KISAYOL_SIL_BETIK: &str = r#"
foreach ($y in @([Environment]::GetFolderPath('Programs'), [Environment]::GetFolderPath('Desktop'))) {
  $l = Join-Path $y 'Volt.lnk'
  if (Test-Path $l) { Remove-Item $l -Force }
}
"#;

/// Volt'u kurar (ya da üstüne yazar). `ilerleme` kısa durum yazılarıyla çağrılır.
pub fn kur(exe: &[u8], masaustu: bool, otobaslat: bool, ilerleme: impl Fn(&str)) -> Result<PathBuf> {
    let k = klasor();
    std::fs::create_dir_all(&k).context("Kurulum klasörü oluşturulamadı")?;
    ilerleme("Dosyalar kopyalanıyor…");
    let hedef = exe_yolu();
    let gecici = k.join("Volt.exe.yeni");
    std::fs::write(&gecici, exe).context("Volt.exe yazılamadı")?;
    // Eski Volt kapanırken dosya kısa süre kilitli kalabilir
    let mut tamam = false;
    for _ in 0..20 {
        if std::fs::rename(&gecici, &hedef).is_ok() {
            tamam = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
    if !tamam {
        let _ = std::fs::remove_file(&gecici);
        bail!("Volt.exe değiştirilemedi. Volt açıksa kapatıp tekrar dene.");
    }
    ilerleme("Kısayollar oluşturuluyor…");
    let yol = hedef.display().to_string();
    ps(KISAYOL_BETIK, &[("VOLT_EXE", yol.clone()), ("VOLT_MASAUSTU", (masaustu as u8).to_string())]);
    ilerleme("Windows'a kaydediliyor…");
    let kb = (exe.len() / 1024).to_string();
    let kaldir = format!("\"{yol}\" --kaldir");
    for (ad, tur, deger) in [
        ("DisplayName", "REG_SZ", "Volt"),
        ("DisplayVersion", "REG_SZ", SURUM),
        ("Publisher", "REG_SZ", "Eresbos"),
        ("DisplayIcon", "REG_SZ", &format!("{yol},0")),
        ("InstallLocation", "REG_SZ", &k.display().to_string()),
        ("UninstallString", "REG_SZ", &kaldir),
        ("URLInfoAbout", "REG_SZ", "https://github.com/ERESB0S/Volt"),
        ("EstimatedSize", "REG_DWORD", &kb),
        ("NoModify", "REG_DWORD", "1"),
        ("NoRepair", "REG_DWORD", "1"),
    ] {
        reg(&["add", KALDIR_ANAHTAR, "/v", ad, "/t", tur, "/d", deger, "/f"]);
    }
    if otobaslat {
        reg(&["add", RUN_ANAHTAR, "/v", "Volt", "/t", "REG_SZ", "/d", &format!("\"{yol}\" --arka"), "/f"]);
    } else if reg(&["query", RUN_ANAHTAR, "/v", "Volt"]) {
        // Eski bir Volt başka yerden açılıyorsa yeni kurulan yeri göstersin
        reg(&["add", RUN_ANAHTAR, "/v", "Volt", "/t", "REG_SZ", "/d", &format!("\"{yol}\" --arka"), "/f"]);
    }
    Ok(hedef)
}

/// Kısayolları, Windows kayıtlarını ve (istenirse) ayarları siler; kurulum klasörünü Volt kapandıktan sonra siler.
pub fn kaldir(ayarlar_da: bool) {
    ps(KISAYOL_SIL_BETIK, &[]);
    reg(&["delete", KALDIR_ANAHTAR, "/f"]);
    reg(&["delete", RUN_ANAHTAR, "/v", "Volt", "/f"]);
    let mut silinecek = Vec::new();
    if kurulu() {
        silinecek.push(klasor());
    }
    if ayarlar_da {
        if let Ok(d) = std::env::var("LOCALAPPDATA") {
            // WebView2 dosyaları Volt açıkken kilitli; bu yüzden klasör de Volt kapanınca silinir
            silinecek.push(PathBuf::from(d).join("Volt"));
        }
    }
    if silinecek.is_empty() {
        return;
    }
    // Volt kapandıktan birkaç saniye sonra sil (çalışan exe kendini silemez)
    let mut komut = String::from("ping 127.0.0.1 -n 5 >nul");
    for y in &silinecek {
        komut += &format!(" & rmdir /s /q \"{}\"", y.display());
    }
    let _ = Command::new("cmd.exe").arg("/c").raw_arg(&komut).creation_flags(PENCERESIZ).spawn();
}
