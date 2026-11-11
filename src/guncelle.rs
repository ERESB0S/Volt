// Copyright 2026 Eresbos (github.com/ERESB0S)
// SPDX-License-Identifier: GPL-3.0-or-later
//! Klavye yazılımını Sonix ROM yükleyicisiyle yazar (Sonix Keyboard Flasher'ın "Flash QMK" adımlarının aynısı).
//!
//! Yükleyici 0C45:7040 (SN32F248B) olarak görünür; 64 baytlık HID feature raporlarıyla konuşulur:
//!   INIT    : 01 AA 55 00                         -> cevap aynı komutla başlar
//!   PREPARE : 05 AA 55 00 | adres u32 | blok u32   -> cevap komut + FA FA FA FA
//!   veri    : 64 baytlık bloklar (cevap yok)
//!   REBOOT  : 07 AA 55 00

use anyhow::{Context, Result, bail};
use hidapi::{HidApi, HidDevice};
use std::time::{Duration, Instant};

const VID: u16 = 0x0C45;
const YUKLEYICI_PID: [u16; 2] = [0x7040, 0x7900]; // SN32F248B, SN32F248
const CMD_INIT: u32 = 0x55AA01;
const CMD_PREPARE: u32 = 0x55AA05;
const CMD_REBOOT: u32 = 0x55AA07;
const DURUM_TAMAM: u32 = 0xFAFAFAFA;
pub const AZAMI: usize = 64 * 1024;

/// Yazılım dosyası bu klavyeye uygun mu? (boyut, ARM vektör tablosu, dosya sonundaki yükleyici işareti)
pub fn denetle(fw: &[u8]) -> Result<()> {
    if fw.len() < 0x100 || fw.len() > AZAMI || fw.len() % 64 != 0 {
        bail!("Yazılım boyutu geçersiz ({} bayt)", fw.len());
    }
    let k = |i: usize| u32::from_le_bytes([fw[i], fw[i + 1], fw[i + 2], fw[i + 3]]);
    let (sp, v0, v1, v2) = (k(0), k(4), k(8), k(12));
    if !(0x2000_0000..=0x2000_2000).contains(&sp) || v0 & 1 == 0 || v1 & 1 == 0 || v2 & 1 == 0 {
        bail!("Bu bir K3 Pro yazılımı gibi görünmüyor (vektör tablosu hatalı)");
    }
    Ok(())
}

/// Yükleyici modundaki klavyeyi bekler (en çok `sure`).
pub fn yukleyici_bekle(sure: Duration) -> Result<HidDevice> {
    let mut api = HidApi::new().context("HID kütüphanesi başlatılamadı")?;
    let t0 = Instant::now();
    loop {
        let _ = api.refresh_devices();
        if let Some(d) = api
            .device_list()
            .find(|d| d.vendor_id() == VID && YUKLEYICI_PID.contains(&d.product_id()) && d.interface_number() <= 0)
        {
            return d.open_device(&api).context("Güncelleme modundaki klavye açılamadı");
        }
        if t0.elapsed() > sure {
            bail!("Klavye güncelleme moduna geçmedi");
        }
        std::thread::sleep(Duration::from_millis(250));
    }
}

fn gonder(d: &HidDevice, veri: &[u8]) -> Result<()> {
    let mut p = [0u8; 65]; // [0] = rapor no
    p[1..1 + veri.len()].copy_from_slice(veri);
    d.send_feature_report(&p).context("Klavyeye yazılamadı")
}

fn oku(d: &HidDevice) -> Result<[u8; 64]> {
    let mut p = [0u8; 65];
    let n = d.get_feature_report(&mut p).context("Klavyeden okunamadı")?;
    if n < 64 {
        bail!("Yükleyiciden kısa cevap geldi");
    }
    let mut o = [0u8; 64];
    o.copy_from_slice(&p[n - 64..n]);
    Ok(o)
}

fn komut(d: &HidDevice, cmd: u32, ek: &[u32]) -> Result<[u8; 64]> {
    let mut v = cmd.to_le_bytes().to_vec();
    for e in ek {
        v.extend_from_slice(&e.to_le_bytes());
    }
    gonder(d, &v)?;
    oku(d)
}

/// Yazılımı 0x0 adresine yazar. `ilerleme` 0..1 arası çağrılır.
pub fn yaz(d: &HidDevice, fw: &[u8], mut ilerleme: impl FnMut(f32)) -> Result<()> {
    denetle(fw)?;
    let u32le = |c: &[u8], i: usize| u32::from_le_bytes([c[i], c[i + 1], c[i + 2], c[i + 3]]);

    let c = komut(d, CMD_INIT, &[])?;
    if u32le(&c, 0) != CMD_INIT {
        bail!("Yükleyici başlatılamadı");
    }
    let bloklar = (fw.len() / 64) as u32;
    let c = komut(d, CMD_PREPARE, &[0, bloklar])?;
    if u32le(&c, 0) != CMD_PREPARE || u32le(&c, 4) != DURUM_TAMAM {
        bail!("Yükleyici yazmaya hazırlanamadı");
    }
    // Buradan sonra yarıda kalırsa klavye yeniden yüklenene kadar çalışmaz
    for (i, parca) in fw.chunks(64).enumerate() {
        gonder(d, parca)?;
        if i % 16 == 0 {
            ilerleme(i as f32 / bloklar as f32);
        }
    }
    ilerleme(1.0);
    gonder(d, &CMD_REBOOT.to_le_bytes())?;
    Ok(())
}

/* ---------------- Orijinal (Monster) yazılım ile geçiş ---------------- */
pub const ORIJINAL_PID: u16 = 0x5004;
pub const VOLT_PID: u16 = 0x5104;

/// Klavyede şu an ne var: "volt", "orijinal", "yukleyici" ya da "yok"
pub fn klavye_durumu() -> &'static str {
    let Ok(api) = HidApi::new() else { return "yok" };
    let mut d = "yok";
    for i in api.device_list().filter(|i| i.vendor_id() == VID) {
        match i.product_id() {
            VOLT_PID => return "volt",
            ORIJINAL_PID => d = "orijinal",
            p if YUKLEYICI_PID.contains(&p) && d == "yok" => d = "yukleyici",
            _ => {}
        }
    }
    d
}

/// Orijinal yazılımdaki klavyeyi güncelleme moduna alır (Sonix Flasher'ın "Reboot to Bootloader [eVision]" komutu)
/// ve güncelleme modundaki klavyeyi döndürür. Bu klavyede güncelleme modu yeni bir USB kimliğiyle görünmez,
/// 0C45:5004 olarak kalır; bu yüzden aday cihazlara INIT gönderilip cevap verenin yükleyici olduğu anlaşılır.
pub fn orijinalden_yukleyiciye(sure: Duration) -> Result<HidDevice> {
    let mut api = HidApi::new().context("HID kütüphanesi başlatılamadı")?;
    {
        let i = api
            .device_list()
            .find(|d| d.vendor_id() == VID && d.product_id() == ORIJINAL_PID && d.interface_number() <= 0)
            .context("Orijinal yazılımdaki klavye bulunamadı. Monster'ın uygulaması açıksa kapat.")?;
        let d = i.open_device(&api).context("Klavye açılamadı. Monster'ın uygulaması açıksa kapat.")?;
        let mut v = 0x5AA5_55AAu32.to_le_bytes().to_vec();
        v.extend_from_slice(&0xCC33_00FFu32.to_le_bytes());
        gonder(&d, &v).context("Klavye güncelleme moduna alınamadı")?;
    }
    std::thread::sleep(Duration::from_millis(1500));
    let t0 = Instant::now();
    loop {
        let _ = api.refresh_devices();
        let adaylar: Vec<_> = api
            .device_list()
            .filter(|d| d.vendor_id() == VID && (YUKLEYICI_PID.contains(&d.product_id()) || d.product_id() == ORIJINAL_PID) && d.interface_number() <= 0)
            .cloned()
            .collect();
        for a in adaylar {
            let Ok(d) = a.open_device(&api) else { continue };
            if YUKLEYICI_PID.contains(&a.product_id()) {
                return Ok(d);
            }
            if let Ok(c) = komut(&d, CMD_INIT, &[]) {
                if u32::from_le_bytes([c[0], c[1], c[2], c[3]]) == CMD_INIT {
                    return Ok(d);
                }
            }
        }
        if t0.elapsed() > sure {
            bail!("Klavye güncelleme moduna geçmedi. Kabloyu çıkar, Esc'ye basılı tutarak tak ve tekrar dene.");
        }
        std::thread::sleep(Duration::from_millis(400));
    }
}

/// Orijinal yazılım yüklendikten sonra klavyenin normal haliyle geri gelmesini bekler
pub fn orijinal_bekle(sure: Duration) -> Result<()> {
    let t0 = Instant::now();
    loop {
        std::thread::sleep(Duration::from_millis(500));
        if klavye_durumu() == "orijinal" {
            // Hâlâ yükleyicide olabilir (aynı kimlik): klavye arayüzü (1) görünüyorsa yazılım çalışıyordur
            if let Ok(api) = HidApi::new() {
                if api.device_list().any(|d| d.vendor_id() == VID && d.product_id() == ORIJINAL_PID && d.interface_number() == 1) {
                    return Ok(());
                }
            }
        }
        if t0.elapsed() > sure {
            bail!("Yükleme bitti ama klavye yeniden bağlanmadı. Kabloyu çıkarıp tak.");
        }
    }
}
