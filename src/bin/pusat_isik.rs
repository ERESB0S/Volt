// Copyright 2026 Eresbos (github.com/ERESB0S)
// SPDX-License-Identifier: GPL-3.0-or-later
//! Volt – özel yazılımlı (QMK "simsek") K3 Pro için ışık ayar uygulaması.
//!
//! Bilgisayarda küçük bir yerel sunucu açar (sadece 127.0.0.1) ve arayüzü Edge'in
//! "uygulama penceresi" kipinde gösterir. Arayüz ayarları bu sunucuya, sunucu da
//! klavyeye raw HID (usage page FF60) üzerinden 32 baytlık paketlerle iletir.
//! Ek kütüphane yok: sadece Rust std + hidapi.

#![windows_subsystem = "windows"]

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::Command;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use hidapi::{HidApi, HidDevice};

const VID: u16 = 0x0C45;
const PID: u16 = 0x5104;
const RAW_USAGE_PAGE: u16 = 0xFF60;
const RAW_USAGE: u16 = 0x61;
const HID_IMZA: u8 = 0x4B;
const PORT: u16 = 47321;

const KMT_OKU: u8 = 0xA1;
const KMT_AYARLA: u8 = 0xA2;
const KMT_KAYDET: u8 = 0xA3;
const KMT_VARSAY: u8 = 0xA4;
const KMT_KAYITLI: u8 = 0xA5;
const KMT_BILGI: u8 = 0xA6;
const KMT_MAKRO: u8 = 0xA7;
const KMT_PROFIL: u8 = 0xA8;
const KMT_KENAR_TEST: u8 = 0xA9;
const KMT_SES: u8 = 0xAA;
const KMT_MAKRO_YAZ: u8 = 0xAB;
const KMT_PROFIL_YAZ: u8 = 0xAC;
const KMT_PROFIL_OKU: u8 = 0xAD;
const KMT_GUNCELLE: u8 = 0xAE;
const MOD_SES: u8 = 18;
const KMT_EKRAN: u8 = 0xAF;
const KMT_BOYA_YAZ: u8 = 0xB0;
const KMT_BOYA_OKU: u8 = 0xB1;
const KMT_UYARI: u8 = 0xB2;
const KMT_EKRAN_KENAR: u8 = 0xB3;
const KMT_GOSTERGE: u8 = 0xB4;
const KMT_TUS_OKU: u8 = 0xB5;
const KMT_TUS_YAZ: u8 = 0xB6;
const KMT_ACILIS: u8 = 0xB7;
const KMT_SISTEM: u8 = 0xB8;
const KMT_SAYAC: u8 = 0xB9;
const KENAR_SISTEM: u8 = 16;
const SAYAC_LED: usize = 122;
const TUS_SAYI: usize = 112;
/// Volt'un GitHub deposu: yeni sürüm denetimi ve indirme sayfası
const GITHUB_DEPO: &str = "ERESB0S/Volt";
const KENAR_EKRAN: u8 = 15;
const MOD_EKRAN: u8 = 19;
const BOYA_LED: usize = 140;

/// Volt ile birlikte gelen klavye yazılımı ve haberleşme sürümü. Klavyedeki daha eskiyse güncelleme önerilir.
const YAZILIM: &[u8] = include_bytes!("../../firmware/volt_k3pro.bin");
const YAZILIM_PROTOKOL: u8 = 16;

/// Yazılım güncellenirken diğer iş parçacıkları klavyeye dokunmasın
static GUNCELLENIYOR: AtomicBool = AtomicBool::new(false);
/// "Volt.exe --kaldir" ile açıldı (Windows'un Uygulamalar listesinden kaldır)
static KALDIR_MODU: AtomicBool = AtomicBool::new(false);
/// Güncelleme durumu: (aşama, ilerleme 0..1, hata)
static GUNCELLE_DURUM: Mutex<(String, f32, String)> = Mutex::new((String::new(), 0.0, String::new()));
/// Pencereyi göstermek/gizlemek için olay döngüsüne erişim
static VEKIL: Mutex<Option<tao::event_loop::EventLoopProxy<Olay>>> = Mutex::new(None);

fn olay_gonder(o: Olay) {
    if let Some(v) = VEKIL.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
        let _ = v.send_event(o);
    }
}

/// Kaydedilen yedek ve profil dosyalarının klasörü: Belgeler\Volt\<alt>
fn volt_klasoru(alt: &str) -> Option<std::path::PathBuf> {
    if !matches!(alt, "yedekler" | "profiller") {
        return None;
    }
    let d = std::path::PathBuf::from(std::env::var("USERPROFILE").ok()?).join("Documents").join("Volt").join(alt);
    std::fs::create_dir_all(&d).ok()?;
    Some(d)
}

/// Dosya adı: sadece harf, rakam, boşluk, - _ . (klasör dışına çıkılamasın)
fn dosya_adi(ad: &str) -> Option<String> {
    let ad = ad.trim().trim_end_matches(".json");
    if ad.is_empty() || ad.chars().count() > 60 || ad.starts_with('.') || !ad.chars().all(|c| c.is_alphanumeric() || " -_.".contains(c)) {
        return None;
    }
    Some(format!("{ad}.json"))
}

fn json_metin(v: &str) -> String {
    let mut o = String::from("\"");
    for c in v.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => {}
            c if (c as u32) < 0x20 => {}
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

fn alan<'a>(govde: &'a str, ad: &str) -> Option<&'a str> {
    govde.split('&').find_map(|kv| kv.strip_prefix(ad).and_then(|v| v.strip_prefix('=')))
}

/// Windows ile başlama: HKCU\...\Run altında "Volt" = "<exe>" --arka
const RUN_ANAHTAR: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
fn otobaslat_acik() -> bool {
    use std::os::windows::process::CommandExt;
    Command::new("reg").args(["query", RUN_ANAHTAR, "/v", "Volt"]).creation_flags(0x0800_0000).output().map(|o| o.status.success()).unwrap_or(false)
}
fn otobaslat_ayarla(acik: bool) -> bool {
    use std::os::windows::process::CommandExt;
    const PENCERESIZ: u32 = 0x0800_0000;
    let c = if acik {
        let Ok(exe) = std::env::current_exe() else { return false };
        Command::new("reg")
            .args(["add", RUN_ANAHTAR, "/v", "Volt", "/t", "REG_SZ", "/f", "/d", &format!("\"{}\" --arka", exe.display())])
            .creation_flags(PENCERESIZ)
            .status()
    } else {
        Command::new("reg").args(["delete", RUN_ANAHTAR, "/v", "Volt", "/f"]).creation_flags(PENCERESIZ).status()
    };
    c.map(|s| s.success()).unwrap_or(false)
}

fn guncelle_ayarla(a: &str, i: f32, h: &str) {
    *GUNCELLE_DURUM.lock().unwrap_or_else(|e| e.into_inner()) = (a.into(), i, h.into());
}

/// Klavye yazılımını yükler. hedef "volt": Volt yazılımı (orijinal yazılımdan geçiş ya da güncelleme),
/// hedef "orijinal": Monster'ın orijinal yazılımına dönüş (dosya Monster'ın sitesinden indirilir).
fn guncelle_baslat(hedef: &str) {
    if GUNCELLENIYOR.swap(true, Ordering::SeqCst) {
        return;
    }
    let orijinale = hedef == "orijinal";
    guncelle_ayarla("hazirlik", 0.0, "");
    std::thread::spawn(move || {
        use pusat_rgb::guncelle as g;
        let sonuc = (|| -> Result<()> {
            let durum = g::klavye_durumu();
            let fw: Vec<u8> = if orijinale {
                if durum == "orijinal" {
                    anyhow::bail!("Klavyede zaten orijinal yazılım var.");
                }
                guncelle_ayarla("indiriyor", 0.0, "");
                pusat_rgb::orijinal::yazilim(|_| {})?
            } else {
                YAZILIM.to_vec()
            };
            g::denetle(&fw)?;
            if durum == "yok" {
                anyhow::bail!("Klavye bulunamadı. Kablosu takılı mı?");
            }
            SES_AKTIF.store(false, Ordering::Relaxed);
            std::thread::sleep(Duration::from_millis(100));
            guncelle_ayarla("bekle", 0.0, "");
            let d = if durum == "orijinal" {
                *CIHAZ.lock().unwrap_or_else(|e| e.into_inner()) = None;
                g::orijinalden_yukleyiciye(Duration::from_secs(20))?
            } else {
                // Volt yazılımına "güncelleme moduna geç" de (v26+). Eski yazılımda kullanıcı Fn+Esc'ye basar.
                if durum == "volt" {
                    let _ = ham(KMT_GUNCELLE, b"BOOT");
                }
                *CIHAZ.lock().unwrap_or_else(|e| e.into_inner()) = None;
                g::yukleyici_bekle(Duration::from_secs(60))?
            };
            guncelle_ayarla("yaziyor", 0.0, "");
            g::yaz(&d, &fw, |i| guncelle_ayarla("yaziyor", i, ""))?;
            drop(d);
            guncelle_ayarla("baglaniyor", 1.0, "");
            if orijinale {
                return g::orijinal_bekle(Duration::from_secs(25));
            }
            let t0 = std::time::Instant::now();
            loop {
                std::thread::sleep(Duration::from_millis(500));
                if let Ok(c) = ham(KMT_BILGI, &[]) {
                    if c[3] >= YAZILIM_PROTOKOL {
                        return Ok(());
                    }
                }
                if t0.elapsed() > Duration::from_secs(20) {
                    anyhow::bail!("Yükleme bitti ama klavye yeniden bağlanmadı. Kabloyu çıkarıp tak.");
                }
            }
        })();
        match sonuc {
            Ok(()) => {
                if !orijinale {
                    // Yükleme klavyenin hafızasını sıfırlar: tuş atamalarını Volt'taki kopyadan geri yaz
                    let a = tus_dosya_oku();
                    if a.iter().any(|&k| k != 0xFFFF) {
                        let _ = tuslar_yaz(&a);
                    }
                }
                guncelle_ayarla("bitti", 1.0, "")
            }
            Err(e) => guncelle_ayarla("hata", 0.0, &format!("{e:#}")),
        }
        GUNCELLENIYOR.store(false, Ordering::SeqCst);
    });
}

const KENAR_SES: u8 = 13;

/* Ses modu: klavye MOD_SES ya da KENAR_SES'teyken çalan ses analiz edilip ~60 Hz gönderilir (sadece RAM'e). */
static SES_AKTIF: AtomicBool = AtomicBool::new(false);
static SES_SON: Mutex<Option<pusat_rgb::ses::SesKare>> = Mutex::new(None);

/// Klavyenin cevabındaki ayarlara göre ses dinlemeye gerek var mı (protokol 7+)
fn ses_gerekli(c: &[u8; 32]) -> bool {
    c[2] == 0 && c[3] >= 7 && (c[4] == MOD_SES || c[4 + 17] == KENAR_SES)
}

fn ses_baslat() {
    pusat_rgb::ses::baslat(&SES_AKTIF, |k| {
        if GUNCELLENIYOR.load(Ordering::Relaxed) {
            return;
        }
        let mut v = [0u8; 19];
        v[..16].copy_from_slice(&k.bant);
        v[16] = k.seviye;
        v[17] = k.ton;
        v[18] = k.bas;
        *SES_SON.lock().unwrap_or_else(|e| e.into_inner()) = Some(*k);
        match ham(KMT_SES, &v) {
            Ok(c) if ses_gerekli(&c) => {}
            _ => SES_AKTIF.store(false, Ordering::Relaxed), // mod değişti ya da klavye çıkarıldı
        }
    });
    // Ses / ekran kapalıyken klavyenin modunu ara ara kontrol et (Fn tuşu ya da M profiliyle değişebilir)
    std::thread::spawn(|| loop {
        std::thread::sleep(Duration::from_millis(500));
        let (ses, ekran, sistem) = (SES_AKTIF.load(Ordering::Relaxed), EKRAN_AKTIF.load(Ordering::Relaxed), SISTEM_AKTIF.load(Ordering::Relaxed));
        if (!ses || !ekran || !sistem) && !GUNCELLENIYOR.load(Ordering::Relaxed) {
            if let Ok(c) = ham(KMT_OKU, &[]) {
                if !ses && ses_gerekli(&c) {
                    SES_AKTIF.store(true, Ordering::Relaxed);
                }
                if !ekran && ekran_gerekli(&c) {
                    EKRAN_AKTIF.store(true, Ordering::Relaxed);
                    std::thread::spawn(ekran_dongu);
                }
                if !sistem && sistem_gerekli(&c) {
                    SISTEM_AKTIF.store(true, Ordering::Relaxed);
                    std::thread::spawn(sistem_dongu);
                }
            }
        }
    });
}

/* ---------------- Küçük ayarlar (%LOCALAPPDATA%\Volt\ayarlar.txt, anahtar=değer satırları) ---------------- */
static AYARLAR: Mutex<Option<Vec<(String, String)>>> = Mutex::new(None);
fn ayar_yolu() -> Option<std::path::PathBuf> {
    std::env::var("LOCALAPPDATA").ok().map(|d| std::path::PathBuf::from(d).join("Volt").join("ayarlar.txt"))
}
fn ayarlar_yukle(g: &mut Option<Vec<(String, String)>>) -> &mut Vec<(String, String)> {
    g.get_or_insert_with(|| {
        ayar_yolu()
            .and_then(|y| std::fs::read_to_string(y).ok())
            .unwrap_or_default()
            .lines()
            .filter_map(|l| l.split_once('=').map(|(k, v)| (k.trim().to_string(), v.trim().to_string())))
            .collect()
    })
}
fn ayar_al(k: &str) -> Option<String> {
    let mut g = AYARLAR.lock().unwrap_or_else(|e| e.into_inner());
    ayarlar_yukle(&mut g).iter().find(|(a, _)| a == k).map(|(_, v)| v.clone())
}
fn ayar_koy(k: &str, v: &str) -> bool {
    let mut g = AYARLAR.lock().unwrap_or_else(|e| e.into_inner());
    let l = ayarlar_yukle(&mut g);
    match l.iter_mut().find(|(a, _)| a == k) {
        Some(e) => e.1 = v.to_string(),
        None => l.push((k.to_string(), v.to_string())),
    }
    let metin = l.iter().map(|(a, b)| format!("{a}={b}\n")).collect::<String>();
    let Some(y) = ayar_yolu() else { return false };
    if let Some(d) = y.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    std::fs::write(y, metin).is_ok()
}

/* ---------------- Sistem göstergesi (kenar ışıkları) ---------------- */
static SISTEM_AKTIF: AtomicBool = AtomicBool::new(false);
/// Son ölçümler (yüzde): CPU, RAM, GPU (sayaç yoksa None)
static SISTEM_SON: Mutex<(f32, f32, Option<f32>)> = Mutex::new((0.0, 0.0, None));

fn sistem_gerekli(c: &[u8; 32]) -> bool {
    c[2] == 0 && c[3] >= 15 && c[4 + 17] == KENAR_SISTEM
}
/// Şeritte gösterilecek ölçüm: "cpu" | "ram" | "gpu"
fn sistem_secim(taraf: &str) -> String {
    let v = ayar_al(if taraf == "sol" { "sistem_sol" } else { "sistem_sag" }).unwrap_or_default();
    if matches!(v.as_str(), "cpu" | "ram" | "gpu") { v } else if taraf == "sol" { "cpu".into() } else { "ram".into() }
}

/// Kenarda sistem göstergesi seçiliyken saniyede 4 kez ölçüp klavyeye gönderir
fn sistem_dongu() {
    let mut o = pusat_rgb::izle::SistemOlcer::yeni();
    let mut yum = [0f32; 3];
    loop {
        let t0 = std::time::Instant::now();
        if GUNCELLENIYOR.load(Ordering::Relaxed) {
            break;
        }
        let olcum = [o.cpu(), pusat_rgb::izle::SistemOlcer::ram(), o.gpu()];
        for (i, v) in olcum.iter().enumerate() {
            if let Some(v) = v {
                yum[i] += (v - yum[i]) * 0.5;
            }
        }
        *SISTEM_SON.lock().unwrap_or_else(|e| e.into_inner()) = (yum[0], yum[1], olcum[2].map(|_| yum[2]));
        let b = |ad: String| {
            let i = match ad.as_str() { "ram" => 1, "gpu" => 2, _ => 0 };
            (yum[i] * 2.55).round().clamp(0.0, 255.0) as u8
        };
        match ham(KMT_SISTEM, &[b(sistem_secim("sol")), b(sistem_secim("sag"))]) {
            Ok(c) if sistem_gerekli(&c) => {}
            _ => break,
        }
        std::thread::sleep(Duration::from_millis(250).saturating_sub(t0.elapsed()));
    }
    SISTEM_AKTIF.store(false, Ordering::Relaxed);
}

/* ---------------- Tuş istatistiği ----------------
 * Klavye sadece LED başına basış sayısını tutar (ne yazıldığı, sıra tutulmaz). Volt 20 sn'de bir okuyup
 * sıfırlar ve güne göre %LOCALAPPDATA%\Volt\istatistik.txt dosyasına ekler: "YYYY-AA-GG l:sayı,l:sayı". */
static ISTATISTIK: Mutex<Option<Vec<(String, Vec<u32>)>>> = Mutex::new(None);
fn istatistik_yolu() -> Option<std::path::PathBuf> {
    std::env::var("LOCALAPPDATA").ok().map(|d| std::path::PathBuf::from(d).join("Volt").join("istatistik.txt"))
}
fn istatistik_yukle(g: &mut Option<Vec<(String, Vec<u32>)>>) -> &mut Vec<(String, Vec<u32>)> {
    g.get_or_insert_with(|| {
        let metin = istatistik_yolu().and_then(|y| std::fs::read_to_string(y).ok()).unwrap_or_default();
        let mut l: Vec<(String, Vec<u32>)> = metin
            .lines()
            .filter_map(|s| {
                let (gun, veri) = s.split_once(' ').unwrap_or((s, ""));
                if gun.len() != 10 {
                    return None;
                }
                let mut v = vec![0u32; SAYAC_LED];
                for p in veri.split(',') {
                    if let Some((i, n)) = p.split_once(':') {
                        if let (Ok(i), Ok(n)) = (i.parse::<usize>(), n.parse::<u32>()) {
                            if i < SAYAC_LED {
                                v[i] = n;
                            }
                        }
                    }
                }
                Some((gun.to_string(), v))
            })
            .collect();
        l.sort_by(|a, b| a.0.cmp(&b.0));
        l
    })
}
fn istatistik_yaz(l: &[(String, Vec<u32>)]) {
    let Some(y) = istatistik_yolu() else { return };
    if let Some(d) = y.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let metin = l
        .iter()
        .map(|(g, v)| format!("{g} {}\n", v.iter().enumerate().filter(|(_, n)| **n > 0).map(|(i, n)| format!("{i}:{n}")).collect::<Vec<_>>().join(",")))
        .collect::<String>();
    let _ = std::fs::write(y, metin);
}
fn istatistik_acik() -> bool {
    ayar_al("istatistik").as_deref() != Some("0")
}
/// Klavyedeki sayaçlar (protokol 15+); sifirla ise okunanlar sıfırlanır
fn sayac_oku(sifirla: bool) -> Result<Option<Vec<u32>>> {
    let mut v = vec![0u32; SAYAC_LED];
    for bas in (0..SAYAC_LED).step_by(14) {
        let c = ham(KMT_SAYAC, &[bas as u8, sifirla as u8])?;
        if c[3] < 15 {
            return Ok(None);
        }
        for n in 0..(SAYAC_LED - bas).min(14) {
            v[bas + n] = u16::from_le_bytes([c[4 + n * 2], c[5 + n * 2]]) as u32;
        }
    }
    Ok(Some(v))
}
/// Sayaçları okuyup bugünün toplamına ekler
fn istatistik_topla() {
    if GUNCELLENIYOR.load(Ordering::Relaxed) || !istatistik_acik() {
        return;
    }
    let mut g = ISTATISTIK.lock().unwrap_or_else(|e| e.into_inner()); // iki toplama aynı anda çalışmasın
    let Ok(Some(v)) = sayac_oku(true) else { return };
    if v.iter().all(|&n| n == 0) {
        return;
    }
    let gun = pusat_rgb::izle::bugun();
    let l = istatistik_yukle(&mut g);
    if l.last().map(|(d, _)| d != &gun).unwrap_or(true) {
        l.push((gun, vec![0; SAYAC_LED]));
    }
    let bugun = &mut l.last_mut().unwrap().1;
    for (a, b) in bugun.iter_mut().zip(&v) {
        *a += b;
    }
    if l.len() > 400 {
        l.remove(0);
    }
    istatistik_yaz(l);
}
fn istatistik_baslat() {
    std::thread::spawn(|| loop {
        istatistik_topla();
        std::thread::sleep(Duration::from_secs(20));
    });
}
fn istatistik_json() -> String {
    istatistik_topla();
    let destek = matches!(ham(KMT_OKU, &[]), Ok(c) if c[3] >= 15);
    let mut g = ISTATISTIK.lock().unwrap_or_else(|e| e.into_inner());
    let gunler = istatistik_yukle(&mut g)
        .iter()
        .map(|(d, v)| format!("[{},[{}]]", json_metin(d), v.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(",")))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"ok\":true,\"destek\":{destek},\"acik\":{},\"bugun\":{},\"gunler\":[{gunler}]}}",
        istatistik_acik(),
        json_metin(&pusat_rgb::izle::bugun())
    )
}

/* ---------------- Ekran rengi (ambilight) ---------------- */
static EKRAN_AKTIF: AtomicBool = AtomicBool::new(false);
static EKRAN_SON: Mutex<([u8; 24], [u8; 24])> = Mutex::new(([0; 24], [0; 24]));

/// Ekran modu ya da kenarda ekran seçili mi (cevap: [3] protokol, [4] mod, [4+17] kenar)
fn ekran_gerekli(c: &[u8; 32]) -> bool {
    c[2] == 0 && c[3] >= 9 && (c[4] == MOD_EKRAN || (c[3] >= 10 && c[4 + 17] == KENAR_EKRAN))
}

/// Klavye ekran modundayken saniyede ~30 kez ekranın renklerini gönderir
fn ekran_dongu() {
    let mut yumusak = [0f32; 48];
    loop {
        let t0 = std::time::Instant::now();
        if GUNCELLENIYOR.load(Ordering::Relaxed) {
            break;
        }
        let monitor = oto_al().monitor;
        if let Some(k) = pusat_rgb::izle::ekran_renkleri(&monitor) {
            let (mut v, mut e) = ([0u8; 24], [0u8; 24]);
            for i in 0..24 {
                yumusak[i] += (k.bolge[i] as f32 - yumusak[i]) * 0.45;
                yumusak[24 + i] += (k.kenar[i] as f32 - yumusak[24 + i]) * 0.45;
                v[i] = yumusak[i] as u8;
                e[i] = yumusak[24 + i] as u8;
            }
            *EKRAN_SON.lock().unwrap_or_else(|e| e.into_inner()) = (v, e);
            let c = match ham(KMT_EKRAN, &v) {
                Ok(c) if ekran_gerekli(&c) => c,
                _ => break,
            };
            if c[3] >= 10 {
                let _ = ham(KMT_EKRAN_KENAR, &e);
            }
        }
        std::thread::sleep(Duration::from_millis(33).saturating_sub(t0.elapsed()));
    }
    EKRAN_AKTIF.store(false, Ordering::Relaxed);
}

/* ---------------- Otomasyon: bildirim ışığı, mikrofon, programa göre profil ---------------- */
#[derive(Clone)]
struct Otomasyon {
    bildirim: bool,
    tuslar: bool, // yanıp sönme tuşlarda da olsun
    renk: [u8; 3],
    adet: u8,
    programlar: Vec<(String, Option<[u8; 3]>, Option<u8>)>, // program, kendi bildirim rengi ve yanıp sönme sayısı (yoksa genel)
    mikrofon: bool,
    oto: bool,
    varsayilan: i8, // -1 = kurala uymayan programda profili değiştirme
    kurallar: Vec<(String, u8)>,
    monitor: String, // ekran rengi modunun izlediği monitör (boş = ana monitör)
}

impl Default for Otomasyon {
    fn default() -> Self {
        Otomasyon { bildirim: true, tuslar: false, renk: [0x58, 0x65, 0xF2], adet: 3, programlar: vec![("discord.exe".into(), Some([0x58, 0x65, 0xF2]), Some(3))], mikrofon: true, oto: false, varsayilan: -1, kurallar: Vec::new(), monitor: String::new() }
    }
}

static OTO: Mutex<Option<Otomasyon>> = Mutex::new(None);

fn oto_yolu() -> Option<std::path::PathBuf> {
    std::env::var("LOCALAPPDATA").ok().map(|d| std::path::PathBuf::from(d).join("Volt").join("otomasyon.txt"))
}

fn exe_temiz(s: &str) -> Option<String> {
    let s = s.trim().to_lowercase();
    let s = if s.is_empty() || s.ends_with(".exe") { s } else { format!("{s}.exe") };
    (!s.is_empty() && s.len() < 80 && s.chars().all(|c| c.is_alphanumeric() || " -_.()+".contains(c))).then_some(s)
}

impl Otomasyon {
    /// "anahtar=değer" satırları; aynı biçim POST gövdesinde de (& ile) kullanılır
    fn coz(metin: &str, ayir: char) -> Otomasyon {
        let mut o = Otomasyon::default();
        for satir in metin.split(ayir) {
            let mut kv = satir.splitn(2, '=');
            let (k, v) = (kv.next().unwrap_or("").trim(), url_coz(kv.next().unwrap_or("").trim()));
            match k {
                "bildirim" => o.bildirim = v == "1",
                "bildirim_tus" => o.tuslar = v == "1",
                "renk" => o.renk = hex_coz(&v).unwrap_or(o.renk),
                "adet" => o.adet = v.parse::<u8>().unwrap_or(3).clamp(1, 10),
                // "discord.exe#5865f2*3,whatsapp.root.exe" (renksiz / sayısız = genel)
                "programlar" => {
                    o.programlar = v
                        .split(',')
                        .filter_map(|p| {
                            let (p, a) = p.rsplit_once('*').map(|(p, a)| (p, a.trim().parse::<u8>().ok().map(|a| a.clamp(1, 10)))).unwrap_or((p, None));
                            let (e, r) = p.split_once('#').map(|(e, r)| (e, hex_coz(r))).unwrap_or((p, None));
                            Some((exe_temiz(e)?, r, a))
                        })
                        .collect()
                }
                "mikrofon" => o.mikrofon = v == "1",
                "oto" => o.oto = v == "1",
                "varsayilan" => o.varsayilan = v.parse::<i8>().unwrap_or(-1).clamp(-1, 3),
                "monitor" => o.monitor = v.chars().filter(|c| !c.is_control()).take(64).collect(),
                "kurallar" => {
                    o.kurallar = v
                        .split(',')
                        .filter_map(|k| {
                            let (e, p) = k.rsplit_once(':')?;
                            Some((exe_temiz(e)?, p.trim().parse::<u8>().ok()?.min(3)))
                        })
                        .collect()
                }
                _ => {}
            }
        }
        o
    }
    fn metin(&self) -> String {
        format!(
            "bildirim={}
bildirim_tus={}
renk={}
adet={}
programlar={}
mikrofon={}
oto={}
varsayilan={}
kurallar={}
",
            self.bildirim as u8,
            self.tuslar as u8,
            hex(self.renk).trim_start_matches('#'),
            self.adet,
            self.programlar
                .iter()
                .map(|(e, r, a)| format!("{e}{}{}", r.map(hex).unwrap_or_default(), a.map(|a| format!("*{a}")).unwrap_or_default()))
                .collect::<Vec<_>>()
                .join(","),
            self.mikrofon as u8,
            self.oto as u8,
            self.varsayilan,
            self.kurallar.iter().map(|(e, p)| format!("{e}:{p}")).collect::<Vec<_>>().join(",")
        ) + &format!("monitor={}\n", self.monitor)
    }
    fn json(&self) -> String {
        format!(
            "{{\"ok\":true,\"bildirim\":{},\"bildirim_tus\":{},\"renk\":{},\"adet\":{},\"programlar\":[{}],\"mikrofon\":{},\"oto\":{},\"varsayilan\":{},\"kurallar\":[{}],\"monitor\":{},\"bildirim_izin\":{},\"son_gonderen\":[{}]}}",
            self.bildirim,
            self.tuslar,
            json_metin(&hex(self.renk)),
            self.adet,
            self.programlar
                .iter()
                .map(|(e, r, a)| format!("{{\"exe\":{},\"renk\":{},\"adet\":{}}}", json_metin(e), json_metin(&hex(r.unwrap_or(self.renk))), a.unwrap_or(self.adet)))
                .collect::<Vec<_>>()
                .join(","),
            self.mikrofon,
            self.oto,
            self.varsayilan,
            self.kurallar.iter().map(|(e, p)| format!("{{\"exe\":{},\"p\":{p}}}", json_metin(e))).collect::<Vec<_>>().join(","),
            json_metin(&self.monitor),
            BILDIRIM_IZIN.load(Ordering::Relaxed),
            SON_GONDEREN.lock().unwrap_or_else(|e| e.into_inner()).iter().map(|a| json_metin(a)).collect::<Vec<_>>().join(",")
        )
    }
}

fn oto_al() -> Otomasyon {
    let mut k = OTO.lock().unwrap_or_else(|e| e.into_inner());
    if k.is_none() {
        let o = oto_yolu().and_then(|y| std::fs::read_to_string(y).ok()).map(|m| Otomasyon::coz(&m, '\n')).unwrap_or_default();
        *k = Some(o);
    }
    k.clone().unwrap()
}

fn oto_kaydet(o: Otomasyon) -> bool {
    let tamam = oto_yolu().is_some_and(|y| {
        let _ = std::fs::create_dir_all(y.parent().unwrap());
        std::fs::write(y, o.metin()).is_ok()
    });
    *OTO.lock().unwrap_or_else(|e| e.into_inner()) = Some(o);
    tamam
}

/// Klavye yeni komutları (sürüm 9) biliyor mu? Bilmiyorsa hiç gönderme.
fn surum9() -> bool {
    matches!(ham(KMT_BILGI, &[]), Ok(c) if c[3] >= 9)
}

fn uyari_gonder(renk: [u8; 3], adet: u8, tuslar: bool) {
    if surum9() {
        let _ = ham(KMT_UYARI, &[renk[0], renk[1], renk[2], adet, 0xFF, tuslar as u8]);
    }
}

/// Windows bildirim erişimi: 0 bilinmiyor, 1 var, 2 yok
static BILDIRIM_IZIN: AtomicU8 = AtomicU8::new(0);
/// Son bildirim gönderen programların adları (en yenisi başta)
static SON_GONDEREN: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Görev çubuğunda bir pencere yanıp sönmeye başladı: listedeki programsa klavyeyi yak
fn pencere_yanip(h: isize) {
    std::thread::spawn(move || {
        let Some(exe) = pusat_rgb::izle::pencere_exe(windows::Win32::Foundation::HWND(h as *mut _)) else { return };
        program_uyari(|e| e == exe);
    });
}

/// Windows bildirimlerini dinle (saniyede bir): program tepsideyken de çalışır
fn bildirim_dinle() {
    std::thread::spawn(|| {
        pusat_rgb::izle::com_baslat();
        let mut d = None;
        let mut deneme = std::time::Instant::now() - Duration::from_secs(60);
        loop {
            std::thread::sleep(Duration::from_millis(700));
            if !oto_al().bildirim {
                continue;
            }
            if d.is_none() {
                if deneme.elapsed() < Duration::from_secs(30) {
                    continue;
                }
                deneme = std::time::Instant::now();
                match pusat_rgb::izle::BildirimDinleyici::yeni() {
                    Ok(x) => { d = Some(x); BILDIRIM_IZIN.store(1, Ordering::Relaxed); }
                    Err(_) => { BILDIRIM_IZIN.store(2, Ordering::Relaxed); continue; }
                }
            }
            for b in d.as_mut().unwrap().yeniler() {
                if !b.ad.is_empty() {
                    let mut s = SON_GONDEREN.lock().unwrap_or_else(|e| e.into_inner());
                    s.retain(|a| *a != b.ad);
                    s.insert(0, b.ad.clone());
                    s.truncate(8);
                }
                // "discord.exe" -> "discord": bildirimin adında ya da uygulama kimliğinde geçiyorsa eşleşir
                program_uyari(|e| { let k = e.trim_end_matches(".exe").replace([' ', '-', '_'], ""); !k.is_empty() && b.anahtar.contains(&k) });
            }
        }
    });
}

/// Listedeki ilk eşleşen programın renginde kenar ışıklarını yak
fn program_uyari(eslesir: impl Fn(&str) -> bool) {
    static SON: Mutex<Vec<(String, std::time::Instant)>> = Mutex::new(Vec::new());
    {
        let o = oto_al();
        if !o.bildirim || GUNCELLENIYOR.load(Ordering::Relaxed) {
            return;
        }
        let tuslar = o.tuslar;
        let Some((exe, renk, adet)) = o.programlar.iter().find(|(e, _, _)| eslesir(e)).map(|(e, r, a)| (e.clone(), r.unwrap_or(o.renk), a.unwrap_or(o.adet))) else { return };
        {
            // Aynı program art arda yanıp söner; 4 sn içinde bir kez yeter
            let mut s = SON.lock().unwrap_or_else(|e| e.into_inner());
            s.retain(|(_, t)| t.elapsed() < Duration::from_secs(4));
            if s.iter().any(|(e, _)| *e == exe) {
                return;
            }
            s.push((exe, std::time::Instant::now()));
        }
        uyari_gonder(renk, adet, tuslar);
    }
}

/// Mikrofon durumu ve öndeki programa göre profil (yarım saniyede bir)
fn otomasyon_baslat() {
    std::thread::spawn(|| {
        pusat_rgb::izle::com_baslat();
        let kendi = std::env::current_exe().ok().and_then(|y| y.file_name().map(|a| a.to_string_lossy().to_lowercase()));
        let mut son_exe: Option<String> = None;
        let mut son_hedef: Option<u8> = None;
        let mut mik_son: Option<bool> = None;
        let mut mik_zaman = std::time::Instant::now();
        loop {
            std::thread::sleep(Duration::from_millis(500));
            if GUNCELLENIYOR.load(Ordering::Relaxed) {
                continue;
            }
            let o = oto_al();
            // Mikrofon: değişince hemen, sonra 2 sn'de bir tazele (klavye 6 sn haber alamazsa kendisi söndürür)
            let sessiz = o.mikrofon && pusat_rgb::izle::mikrofon_sessiz().unwrap_or(false);
            if mik_son != Some(sessiz) || (sessiz && mik_zaman.elapsed() > Duration::from_secs(2)) {
                if surum9() && ham(KMT_UYARI, &[0, 0, 0, 0xFF, sessiz as u8]).is_ok() {
                    mik_son = Some(sessiz);
                    mik_zaman = std::time::Instant::now();
                }
            }
            // Programa göre profil
            if !o.oto {
                son_exe = None;
                son_hedef = None;
                continue;
            }
            let Some(exe) = pusat_rgb::izle::on_plan_exe() else { continue };
            if Some(&exe) == kendi.as_ref() || exe == "explorer.exe" && son_exe.is_some() && o.varsayilan < 0 {
                continue;
            }
            if son_exe.as_ref() == Some(&exe) {
                continue;
            }
            son_exe = Some(exe.clone());
            let hedef = o.kurallar.iter().find(|(e, _)| *e == exe).map(|(_, p)| *p).or((o.varsayilan >= 0).then_some(o.varsayilan as u8));
            if hedef.is_some() && hedef != son_hedef {
                if ham(KMT_PROFIL, &[hedef.unwrap()]).is_ok() {
                    son_hedef = hedef;
                }
            } else if hedef.is_none() {
                son_hedef = None;
            }
        }
    });
}

/* ---------------- Tuş boyama ---------------- */
/// Profilin boyaması: 140 renk "rrggbb" ya da yoksa null
fn boya_oku(p: u8) -> Result<Option<Vec<[u8; 3]>>> {
    let mut v = Vec::with_capacity(BOYA_LED);
    for bas in (0..BOYA_LED).step_by(8) {
        let c = ham(KMT_BOYA_OKU, &[p, bas as u8])?;
        if c[3] < 9 {
            bail!("Klavye yazılımı bu özelliği desteklemiyor, güncelle");
        }
        if c[2] != 0 {
            return Ok(None);
        }
        for i in 0..(BOYA_LED - bas).min(8) {
            v.push([c[4 + i * 3], c[5 + i * 3], c[6 + i * 3]]);
        }
    }
    Ok(Some(v))
}

fn boya_yaz(p: u8, renk: &[[u8; 3]]) -> Result<()> {
    for bas in (0..BOYA_LED).step_by(8) {
        let adet = (BOYA_LED - bas).min(8);
        let mut v = [0u8; 30];
        v[0] = p;
        v[1] = bas as u8;
        v[2] = adet as u8;
        v[3] = (bas + adet >= BOYA_LED) as u8;
        for i in 0..adet {
            v[6 + i * 3..9 + i * 3].copy_from_slice(&renk[bas + i]);
        }
        let c = ham(KMT_BOYA_YAZ, &v)?;
        if c[3] < 9 {
            bail!("Klavye yazılımı bu özelliği desteklemiyor, güncelle");
        }
        if c[2] != 0 {
            bail!("Boyama klavyeye yazılamadı");
        }
    }
    Ok(())
}

/* ---------------- Tuş atama ---------------- */
/// Klavyedeki tuşlar: LED başına (varsayılan kod, atama); atanamaz tuşta varsayılan 0xFFFF. Eski yazılımda None.
fn tuslar_oku() -> Result<Option<Vec<(u16, u16)>>> {
    let mut v = Vec::with_capacity(TUS_SAYI);
    for bas in (0..TUS_SAYI).step_by(7) {
        let c = ham(KMT_TUS_OKU, &[bas as u8])?;
        if c[3] < 13 {
            return Ok(None);
        }
        for n in 0..(TUS_SAYI - bas).min(7) {
            let o = 4 + n * 4;
            v.push((u16::from_le_bytes([c[o], c[o + 1]]), u16::from_le_bytes([c[o + 2], c[o + 3]])));
        }
    }
    Ok(Some(v))
}

fn tuslar_yaz(atama: &[u16]) -> Result<()> {
    for bas in (0..TUS_SAYI).step_by(12) {
        let adet = (TUS_SAYI - bas).min(12);
        let mut v = [0u8; 28];
        v[0] = bas as u8;
        v[1] = adet as u8;
        v[2] = (bas + adet >= TUS_SAYI) as u8;
        for i in 0..adet {
            v[4 + i * 2..6 + i * 2].copy_from_slice(&atama[bas + i].to_le_bytes());
        }
        let c = ham(KMT_TUS_YAZ, &v)?;
        if c[3] < 13 {
            bail!("Klavye yazılımı bu özelliği desteklemiyor, güncelle");
        }
        if c[2] != 0 {
            bail!("Tuş atamaları klavyeye yazılamadı");
        }
    }
    Ok(())
}

/// Atamaların Volt'taki kopyası (%LOCALAPPDATA%\Volt\tuslar.txt, "led:kod" virgülle): yazılım güncellemesinden sonra geri yazılır
fn tus_dosya_yolu() -> Option<std::path::PathBuf> {
    std::env::var("LOCALAPPDATA").ok().map(|d| std::path::PathBuf::from(d).join("Volt").join("tuslar.txt"))
}
fn tus_dosya_oku() -> Vec<u16> {
    let mut a = vec![0xFFFFu16; TUS_SAYI];
    let metin = tus_dosya_yolu().and_then(|y| std::fs::read_to_string(y).ok()).unwrap_or_default();
    for p in metin.trim().split(',') {
        if let Some((l, k)) = p.split_once(':') {
            if let (Ok(l), Ok(k)) = (l.trim().parse::<usize>(), k.trim().parse::<u16>()) {
                if l < TUS_SAYI && k <= 0x1FFF {
                    a[l] = k;
                }
            }
        }
    }
    a
}
fn tus_dosya_yaz(a: &[u16]) -> bool {
    let Some(y) = tus_dosya_yolu() else { return false };
    if let Some(d) = y.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let m = a.iter().enumerate().filter(|(_, k)| **k != 0xFFFF).map(|(l, k)| format!("{l}:{k}")).collect::<Vec<_>>().join(",");
    std::fs::write(y, m).is_ok()
}
fn tuslar_json() -> String {
    match tuslar_oku() {
        Ok(Some(v)) => {
            let l = v
                .iter()
                .enumerate()
                .filter(|(_, (d, _))| *d != 0xFFFF)
                .map(|(i, (d, a))| format!("[{i},{d},{}]", if *a <= 0x1FFF { *a as i32 } else { -1 }))
                .collect::<Vec<_>>()
                .join(",");
            format!("{{\"ok\":true,\"destek\":true,\"tuslar\":[{l}]}}")
        }
        Ok(None) => "{\"ok\":true,\"destek\":false}".into(),
        Err(e) => hata_json(&e),
    }
}

/* ---------------- Volt güncelleme denetimi ---------------- */
/// GitHub'daki son sürüm ("1.0.0"); 6 saat önbellekte tutulur. İstekte kullanıcıya ait bilgi gönderilmez.
static SON_SURUM: Mutex<Option<(std::time::Instant, String)>> = Mutex::new(None);
fn son_surum(zorla: bool) -> Result<String> {
    if !zorla {
        if let Some((t, s)) = SON_SURUM.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            if t.elapsed() < Duration::from_secs(6 * 3600) {
                return Ok(s.clone());
            }
        }
    }
    use std::os::windows::process::CommandExt;
    let o = Command::new("curl.exe")
        .args(["-s", "-f", "-L", "--max-time", "8", "-H", "User-Agent: Volt", "-H", "Accept: application/vnd.github+json"])
        .arg(format!("https://api.github.com/repos/{GITHUB_DEPO}/releases/latest"))
        .creation_flags(0x0800_0000)
        .output()
        .context("Sürüm denetlenemedi")?;
    if !o.status.success() {
        bail!("GitHub'a ulaşılamadı ya da henüz yayınlanmış sürüm yok");
    }
    let m = String::from_utf8_lossy(&o.stdout);
    let i = m.find("\"tag_name\"").context("Sürüm bilgisi okunamadı")?;
    let r = &m[i + 10..];
    let r = &r[r.find('"').context("Sürüm bilgisi okunamadı")? + 1..];
    let s: String = r[..r.find('"').context("Sürüm bilgisi okunamadı")?].trim_start_matches(['v', 'V']).chars().filter(|c| c.is_ascii_alphanumeric() || *c == '.' || *c == '-').take(20).collect();
    *SON_SURUM.lock().unwrap_or_else(|e| e.into_inner()) = Some((std::time::Instant::now(), s.clone()));
    Ok(s)
}

/// Kenar ışığı eşleştirme sonucunun saklandığı dosya (%LOCALAPPDATA%\Volt\kenar_harita.json)
fn kenar_harita_yolu() -> Option<std::path::PathBuf> {
    std::env::var("LOCALAPPDATA").ok().map(|d| std::path::PathBuf::from(d).join("Volt").join("kenar_harita.json"))
}

const ARAYUZ: &str = include_str!("pusat_isik.html");

/// Klavyedeki isik_ayar_t ile birebir aynı sıra (20 bayt).
#[derive(Clone, Copy, Default)]
struct Ayar {
    mod_: u8,
    taban: [u8; 3],
    parla: [u8; 3],
    taban_parlak: u8,
    genel_parlak: u8,
    yaricap: u8,
    sure: u16,
    titreme: u8,
    sira: u8,
    kalinlik: u8,
    iz: u8,
    tetik: u8,
    kenar: u8,
    hiz: u8,
    renkli: u8,
    kenar_a: [u8; 3],
    kenar_b: [u8; 3],
    kenar_parlak: u8,
    kenar_hiz: u8,
}

impl Ayar {
    fn baytlar(&self) -> [u8; 28] {
        let mut b = [0u8; 28];
        b[20..23].copy_from_slice(&self.kenar_a);
        b[23..26].copy_from_slice(&self.kenar_b);
        b[26] = self.kenar_parlak;
        b[27] = self.kenar_hiz;
        b[0] = self.mod_;
        b[1..4].copy_from_slice(&self.taban);
        b[4..7].copy_from_slice(&self.parla);
        b[7] = self.taban_parlak;
        b[8] = self.genel_parlak;
        b[9] = self.yaricap;
        b[10..12].copy_from_slice(&self.sure.to_le_bytes());
        b[12] = self.titreme;
        b[13] = self.sira;
        b[14] = self.kalinlik;
        b[15] = self.iz;
        b[16] = self.tetik;
        b[17] = self.kenar;
        b[18] = self.hiz;
        b[19] = self.renkli;
        b
    }

    fn baytlardan(b: &[u8]) -> Self {
        Ayar {
            mod_: b[0],
            taban: [b[1], b[2], b[3]],
            parla: [b[4], b[5], b[6]],
            taban_parlak: b[7],
            genel_parlak: b[8],
            yaricap: b[9],
            sure: u16::from_le_bytes([b[10], b[11]]),
            titreme: b[12],
            sira: b[13],
            kalinlik: b[14],
            iz: b[15],
            tetik: b[16],
            kenar: b[17],
            hiz: b[18],
            renkli: b[19],
            kenar_a: [b[20], b[21], b[22]],
            kenar_b: [b[23], b[24], b[25]],
            kenar_parlak: b[26],
            kenar_hiz: b[27],
        }
    }

    fn json(&self, durum: u8) -> String {
        format!(
            "{{\"ok\":true,\"durum\":{},\"mod\":{},\"taban\":\"{}\",\"parla\":\"{}\",\"taban_parlak\":{},\"genel_parlak\":{},\"yaricap\":{},\"sure\":{},\"titreme\":{},\"sira\":{},\"kalinlik\":{},\"iz\":{},\"tetik\":{},\"kenar\":{},\"hiz\":{},\"renkli\":{},\"kenar_a\":\"{}\",\"kenar_b\":\"{}\",\"kenar_parlak\":{},\"kenar_hiz\":{}}}",
            durum,
            self.mod_,
            hex(self.taban),
            hex(self.parla),
            self.taban_parlak,
            self.genel_parlak,
            self.yaricap,
            self.sure,
            self.titreme,
            self.sira,
            self.kalinlik,
            self.iz,
            self.tetik,
            self.kenar,
            self.hiz,
            self.renkli,
            hex(self.kenar_a),
            hex(self.kenar_b),
            self.kenar_parlak,
            self.kenar_hiz
        )
    }
}

fn hex(c: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

fn hex_coz(s: &str) -> Option<[u8; 3]> {
    let s = s.trim_start_matches('#');
    if s.len() != 6 {
        return None;
    }
    let p = |i: usize| u8::from_str_radix(&s[i..i + 2], 16).ok();
    Some([p(0)?, p(2)?, p(4)?])
}

/// Klavyeye bir komut gönderir, cevabı (durum, ayar) olarak döner.
/// Açık klavye bağlantısı: her istekte cihazları baştan taramak yavaş (yüzlerce ms),
/// bu yüzden bir kez açılıp saklanır; hata olursa kapatılıp yeniden açılır.
static CIHAZ: Mutex<Option<(HidApi, HidDevice)>> = Mutex::new(None);

fn klavye(komut: u8, ayar: Option<&Ayar>) -> Result<(u8, Ayar)> {
    let c = match ayar { Some(a) => ham(komut, &a.baytlar())?, None => ham(komut, &[])? };
    let mut a = Ayar::baytlardan(&c[4..32]);
    if c[3] < 4 {
        // Eski yazılım: yan ışık ayarları yok, uygulama varsayılanları göstersin
        (a.kenar_a, a.kenar_b, a.kenar_parlak, a.kenar_hiz) = ([0, 0, 255], [220, 230, 255], 90, 128);
    }
    Ok((c[2], a))
}

/// Ayar cevabına seçili profili ekler: "profil" (0..3, eski yazılımda -1) ve "profiller" (kayıtlı profil bit maskesi)
fn profilli(json: String) -> String {
    if !json.starts_with("{\"ok\":true") {
        return json;
    }
    let (p, m) = match ham(KMT_BILGI, &[]) {
        Ok(c) if c[3] >= 5 && c[2] == 0 => (c[21] as i32, c[22]),
        _ => (-1, 0),
    };
    format!("{},\"profil\":{},\"profiller\":{}}}", &json[..json.len() - 1], p, m)
}

/// Ana sayfa bilgisi (klavye yazılımı sürüm 2+). Eski yazılımda desteklenmiyor döner.
fn bilgi_json() -> String {
    match ham(KMT_BILGI, &[]) {
        Err(e) => hata_json(&e),
        Ok(c) if c[3] < 2 || c[2] != 0 => format!("{{\"ok\":true,\"destek\":false,\"surum\":{}}}", c[3]),
        Ok(c) => {
            let u32le = |i: usize| u32::from_le_bytes([c[i], c[i + 1], c[i + 2], c[i + 3]]);
            format!(
                "{{\"ok\":true,\"destek\":true,\"surum\":{},\"makro\":[{},{},{},{},{}],\"win_kilit\":{},\"teker_isik\":{},\"tus\":{},\"sure\":{},\"kayit_var\":{},\"kayitli_ayni\":{},\"profil\":{},\"profiller\":{}}}",
                c[3], c[4], c[5], c[6], c[7], c[8], c[9] != 0, c[10] != 0, u32le(11), u32le(15), c[19] != 0, c[20] != 0,
                if c[3] >= 5 { c[21] as i32 } else { -1 }, if c[3] >= 5 { c[22] } else { 0 }
            )
        }
    }
}

/// G1..G5 makrolarının olayları: {"ok":true,"makro":[[olay,...],...]} (klavye yazılımı sürüm 3+)
fn makro_json() -> String {
    makro_json_p(0)
}

/// p = profil+1 (0 = seçili profil)
fn makro_json_p(p: u8) -> String {
    let mut yuvalar = Vec::new();
    for y in 0..5u8 {
        let mut olaylar: Vec<u16> = Vec::new();
        loop {
            let c = match ham(KMT_MAKRO, &[y, olaylar.len() as u8, p]) {
                Ok(c) => c,
                Err(e) => return hata_json(&e),
            };
            if c[3] < 3 {
                return "{\"ok\":true,\"destek\":false}".into();
            }
            let (toplam, n) = (c[4] as usize, c[5] as usize);
            for i in 0..n.min(13) {
                olaylar.push(u16::from_le_bytes([c[6 + i * 2], c[7 + i * 2]]));
            }
            if n == 0 || olaylar.len() >= toplam {
                break;
            }
        }
        yuvalar.push(format!("[{}]", olaylar.iter().map(|o| o.to_string()).collect::<Vec<_>>().join(",")));
    }
    format!("{{\"ok\":true,\"destek\":true,\"makro\":[{}]}}", yuvalar.join(","))
}

/// Windows tarafındaki Num / Caps / Scroll Lock durumu (klavye yazılımından bağımsız)
fn kilitler_json() -> String {
    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetKeyState(tus: i32) -> i16;
    }
    let acik = |vk: i32| unsafe { GetKeyState(vk) } & 1 != 0;
    format!("{{\"ok\":true,\"num\":{},\"caps\":{},\"scroll\":{}}}", acik(0x90), acik(0x14), acik(0x91))
}

fn ham(komut: u8, veri: &[u8]) -> Result<[u8; 32]> {
    let mut kilit = CIHAZ.lock().unwrap_or_else(|e| e.into_inner());
    for deneme in 0..2 {
        if kilit.is_none() {
            *kilit = Some(cihaz_ac()?);
        }
        match konus(&kilit.as_ref().unwrap().1, komut, veri) {
            Ok(r) => return Ok(r),
            Err(e) => {
                *kilit = None;
                if deneme == 1 {
                    return Err(e);
                }
            }
        }
    }
    unreachable!()
}

fn cihaz_ac() -> Result<(HidApi, HidDevice)> {
    let api = HidApi::new().context("HID kütüphanesi başlatılamadı")?;
    let info = api
        .device_list()
        .find(|d| {
            d.vendor_id() == VID
                && d.product_id() == PID
                && d.usage_page() == RAW_USAGE_PAGE
                && d.usage() == RAW_USAGE
        })
        .context("Klavye bulunamadı. Kablo takılı mı, özel yazılım (K3 Pro QMK) yüklü mü?")?;
    let dev = info.open_device(&api).context("Klavye açılamadı")?;
    Ok((api, dev))
}

fn konus(dev: &HidDevice, komut: u8, veri: &[u8]) -> Result<[u8; 32]> {
    // Önceki isteklerden kalmış cevapları temizle
    let mut cop = [0u8; 32];
    while dev.read_timeout(&mut cop, 0).unwrap_or(0) > 0 {}

    let mut paket = [0u8; 33]; // [0] = rapor no (0), ardından 32 bayt
    paket[1] = komut;
    paket[2] = HID_IMZA;
    paket[3..3 + veri.len()].copy_from_slice(veri);
    dev.write(&paket).context("Klavyeye gönderilemedi")?;

    let mut cevap = [0u8; 32];
    for _ in 0..5 {
        let n = dev.read_timeout(&mut cevap, 400).context("Klavyeden okunamadı")?;
        if n == 0 {
            bail!("Klavye cevap vermedi");
        }
        if cevap[0] == komut && cevap[1] == HID_IMZA {
            return Ok(cevap);
        }
    }
    bail!("Klavyeden beklenmeyen cevap geldi");
}

fn form_coz(govde: &str) -> Ayar {
    let mut a = Ayar::default();
    for parca in govde.split('&') {
        let mut kv = parca.splitn(2, '=');
        let (k, v) = (kv.next().unwrap_or(""), kv.next().unwrap_or("").replace("%23", "#"));
        let n = || v.parse::<u32>().unwrap_or(0);
        match k {
            "mod" => a.mod_ = n().min(21) as u8,
            "taban" => a.taban = hex_coz(&v).unwrap_or([0, 0, 255]),
            "parla" => a.parla = hex_coz(&v).unwrap_or([255, 255, 255]),
            "taban_parlak" => a.taban_parlak = n().min(100) as u8,
            "genel_parlak" => a.genel_parlak = n().min(100) as u8,
            "yaricap" => a.yaricap = n().clamp(8, 120) as u8,
            "sure" => a.sure = n().clamp(100, 3000) as u16,
            "titreme" => a.titreme = n().min(3) as u8, // şimşekte 0/1, ses modunda stil 0..3
            "sira" => a.sira = n().min(5) as u8,
            "kalinlik" => a.kalinlik = n().clamp(8, 120) as u8,
            "iz" => a.iz = n().min(100) as u8,
            "tetik" => a.tetik = n().min(2) as u8,
            "kenar" => a.kenar = n().min(16) as u8,
            "hiz" => a.hiz = n().min(255) as u8,
            "renkli" => a.renkli = (n() != 0) as u8,
            "kenar_a" => a.kenar_a = hex_coz(&v).unwrap_or([0, 0, 255]),
            "kenar_b" => a.kenar_b = hex_coz(&v).unwrap_or([220, 230, 255]),
            "kenar_parlak" => a.kenar_parlak = n().min(100) as u8,
            "kenar_hiz" => a.kenar_hiz = n().min(255) as u8,
            _ => {}
        }
    }
    a
}

/// application/x-www-form-urlencoded değerini çöz (%XX ve +)
fn url_coz(v: &str) -> String {
    let b = v.as_bytes();
    let mut o = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'+' => o.push(b' '),
            b'%' if i + 2 < b.len() => {
                match u8::from_str_radix(std::str::from_utf8(&b[i + 1..i + 3]).unwrap_or(""), 16) {
                    Ok(x) => { o.push(x); i += 2; }
                    Err(_) => o.push(b'%'),
                }
            }
            c => o.push(c),
        }
        i += 1;
    }
    String::from_utf8_lossy(&o).into_owned()
}

fn hata_json(e: &anyhow::Error) -> String {
    let m = format!("{e:#}").replace('\\', "\\\\").replace('"', "'");
    format!("{{\"ok\":false,\"hata\":\"{m}\"}}")
}

fn cevapla(s: &mut TcpStream, durum: &str, tur: &str, govde: &str) {
    let basliklar = format!(
        "HTTP/1.1 {durum}\r\nContent-Type: {tur}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
        govde.len()
    );
    let _ = s.write_all(basliklar.as_bytes());
    let _ = s.write_all(govde.as_bytes());
}

/// Bir HTTP isteğini işler. `true` dönerse uygulama kapanır.
fn istek(mut s: TcpStream) -> bool {
    let _ = s.set_read_timeout(Some(Duration::from_secs(5)));
    let mut tampon = Vec::new();
    let mut parca = [0u8; 2048];
    let baslik_sonu = loop {
        let n = match s.read(&mut parca) {
            Ok(0) | Err(_) => return false,
            Ok(n) => n,
        };
        tampon.extend_from_slice(&parca[..n]);
        if let Some(i) = tampon.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
        if tampon.len() > 16 * 1024 {
            return false;
        }
    };
    let baslik = String::from_utf8_lossy(&tampon[..baslik_sonu]).to_string();
    let uzunluk = baslik
        .lines()
        .find_map(|l| {
            let l = l.to_ascii_lowercase();
            l.strip_prefix("content-length:").map(|v| v.trim().parse::<usize>().unwrap_or(0))
        })
        .unwrap_or(0)
        .min(256 * 1024);
    while tampon.len() < baslik_sonu + uzunluk {
        match s.read(&mut parca) {
            Ok(0) | Err(_) => break,
            Ok(n) => tampon.extend_from_slice(&parca[..n]),
        }
    }
    let govde = String::from_utf8_lossy(&tampon[baslik_sonu..]).to_string();

    let ilk = baslik.lines().next().unwrap_or("");
    let mut p = ilk.split_whitespace();
    let (yontem, yol) = (p.next().unwrap_or(""), p.next().unwrap_or(""));

    // Sadece bu uygulamanın kendi sayfası API'yi kullanabilsin (başka sitelerden gelen istekleri reddet)
    let host_iyi = baslik
        .lines()
        .any(|l| l.to_ascii_lowercase() == format!("host: 127.0.0.1:{PORT}"));
    if !host_iyi {
        cevapla(&mut s, "403 Forbidden", "text/plain", "yasak");
        return false;
    }
    if yontem == "POST" {
        let kaynak_iyi = baslik.lines().all(|l| {
            let k = l.to_ascii_lowercase();
            !k.starts_with("origin:") || k.contains(&format!("127.0.0.1:{PORT}"))
        });
        if !kaynak_iyi {
            cevapla(&mut s, "403 Forbidden", "text/plain", "yasak");
            return false;
        }
    }

    let sonuc = |r: Result<(u8, Ayar)>| match r {
        Ok((d, a)) => a.json(d),
        Err(e) => hata_json(&e),
    };

    match (yontem, yol) {
        ("GET", "/") => cevapla(&mut s, "200 OK", "text/html; charset=utf-8", ARAYUZ),
        ("GET", y) if y.starts_with("/yazi/") => match pusat_rgb::yazi::dosya(&y[6..]) {
            Some(b) => {
                let _ = s.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: font/ttf\r\nContent-Length: {}\r\nCache-Control: max-age=31536000\r\nConnection: close\r\n\r\n", b.len()).as_bytes());
                let _ = s.write_all(b);
            }
            None => cevapla(&mut s, "404 Not Found", "text/plain", "yok"),
        },
        ("GET", "/api/nabiz") => cevapla(&mut s, "200 OK", "text/plain", "ok"),
        ("GET", "/api/kilitler") => cevapla(&mut s, "200 OK", "application/json", &kilitler_json()),
        ("GET", "/api/makro") => cevapla(&mut s, "200 OK", "application/json", &makro_json()),
        ("GET", "/api/ses") => {
            // Önizleme için son ses karesi
            let aktif = SES_AKTIF.load(Ordering::Relaxed);
            let j = match (aktif, *SES_SON.lock().unwrap_or_else(|e| e.into_inner())) {
                (true, Some(k)) => format!(
                    "{{\"ok\":true,\"aktif\":true,\"bant\":{:?},\"seviye\":{},\"ton\":{},\"bas\":{}}}",
                    k.bant, k.seviye, k.ton, k.bas
                ),
                _ => "{\"ok\":true,\"aktif\":false}".into(),
            };
            cevapla(&mut s, "200 OK", "application/json", &j);
        }
        ("GET", "/api/ekran") => {
            let j = if EKRAN_AKTIF.load(Ordering::Relaxed) {
{
                    let (v, e) = *EKRAN_SON.lock().unwrap_or_else(|e| e.into_inner());
                    format!("{{\"ok\":true,\"aktif\":true,\"renk\":{v:?},\"kenar\":{e:?}}}")
                }
            } else {
                "{\"ok\":true,\"aktif\":false}".into()
            };
            cevapla(&mut s, "200 OK", "application/json", &j);
        }
        ("GET", "/api/otomasyon") => {
            let j = oto_al().json();
            let on = pusat_rgb::izle::on_plan_exe().unwrap_or_default();
            let mik = match pusat_rgb::izle::mikrofon_sessiz() { Some(b) => b.to_string(), None => "null".into() };
            cevapla(&mut s, "200 OK", "application/json", &format!("{},\"mikrofon_sessiz\":{mik},\"onplan\":{},\"onplan_ad\":{}}}", &j[..j.len() - 1], json_metin(&on), json_metin(&pusat_rgb::izle::on_plan_adi().unwrap_or_default())));
        }
        ("POST", "/api/otomasyon") => {
            let mut o = Otomasyon::coz(&govde, '&');
            o.monitor = oto_al().monitor; // bu sayfa monitörü göndermez
            let tamam = oto_kaydet(o);
            cevapla(&mut s, "200 OK", "application/json", &format!("{},\"kaydedildi\":{tamam}}}", { let j = oto_al().json(); j[..j.len() - 1].to_string() }));
        }
        ("POST", "/api/uyari-dene") => {
            let o = Otomasyon::coz(&govde, '&');
            let j = if !surum9() { "{\"ok\":false,\"hata\":\"Klavye yazılımı bu özelliği desteklemiyor, güncelle\"}".to_string() } else {
                match ham(KMT_UYARI, &[o.renk[0], o.renk[1], o.renk[2], o.adet, 0xFF, o.tuslar as u8]) { Ok(_) => "{\"ok\":true}".into(), Err(e) => hata_json(&e) }
            };
            cevapla(&mut s, "200 OK", "application/json", &j);
        }
        ("GET", "/api/gosterge") | ("POST", "/api/gosterge") => {
            // Kilit ve Fn göstergeleri (klavyede saklanır, tüm profiller için ortak)
            let veri = if yontem == "POST" {
                let b = |ad: &str| (alan(&govde, ad) == Some("1")) as u8;
                let r = hex_coz(&url_coz(alan(&govde, "renk").unwrap_or(""))).unwrap_or([255, 255, 255]);
                vec![1, b("caps") | b("num") << 1 | b("scroll") << 2 | b("fn") << 3, r[0], r[1], r[2]]
            } else {
                vec![0]
            };
            let j = match ham(KMT_GOSTERGE, &veri) {
                Ok(c) if c[3] >= 11 && c[2] == 0 => format!(
                    "{{\"ok\":true,\"destek\":true,\"caps\":{},\"num\":{},\"scroll\":{},\"fn\":{},\"renk\":{}}}",
                    c[4] & 1 != 0, c[4] & 2 != 0, c[4] & 4 != 0, c[4] & 8 != 0, json_metin(&hex([c[5], c[6], c[7]]))
                ),
                Ok(c) if c[3] >= 11 => "{\"ok\":false,\"hata\":\"Göstergeler kaydedilemedi\"}".into(),
                Ok(_) => "{\"ok\":true,\"destek\":false}".into(),
                Err(e) => hata_json(&e),
            };
            cevapla(&mut s, "200 OK", "application/json", &j);
        }
        ("GET", "/api/sistem") => {
            let (c, r, g) = *SISTEM_SON.lock().unwrap_or_else(|e| e.into_inner());
            let gpu = g.map(|g| format!("{g:.0}")).unwrap_or_else(|| "null".into());
            let j = format!(
                "{{\"ok\":true,\"aktif\":{},\"cpu\":{c:.0},\"ram\":{r:.0},\"gpu\":{gpu},\"sol\":{},\"sag\":{}}}",
                SISTEM_AKTIF.load(Ordering::Relaxed),
                json_metin(&sistem_secim("sol")),
                json_metin(&sistem_secim("sag"))
            );
            cevapla(&mut s, "200 OK", "application/json", &j);
        }
        ("POST", "/api/sistem") => {
            let mut tamam = true;
            for (alan_ad, anahtar) in [("sol", "sistem_sol"), ("sag", "sistem_sag")] {
                if let Some(v) = alan(&govde, alan_ad).filter(|v| matches!(*v, "cpu" | "ram" | "gpu")) {
                    tamam &= ayar_koy(anahtar, v);
                }
            }
            cevapla(&mut s, "200 OK", "application/json", &format!("{{\"ok\":true,\"kaydedildi\":{tamam}}}"));
        }
        ("GET", "/api/istatistik") => cevapla(&mut s, "200 OK", "application/json", &istatistik_json()),
        ("POST", "/api/istatistik") => {
            match alan(&govde, "islem") {
                Some("sil") => {
                    let mut g = ISTATISTIK.lock().unwrap_or_else(|e| e.into_inner());
                    *g = Some(Vec::new());
                    if let Some(y) = istatistik_yolu() {
                        let _ = std::fs::remove_file(y);
                    }
                    let _ = sayac_oku(true); // klavyede bekleyen sayılar da silinsin
                }
                Some("ac") => {
                    let acik = alan(&govde, "acik") == Some("1");
                    ayar_koy("istatistik", if acik { "1" } else { "0" });
                    if acik {
                        let _ = sayac_oku(true); // kapalıyken birikenler sayılmasın
                    }
                }
                _ => {}
            }
            cevapla(&mut s, "200 OK", "application/json", &istatistik_json());
        }
        ("GET", "/api/acilis") | ("POST", "/api/acilis") => {
            // islem=kaydet|dene, stil=0..6 (0 = kapalı); GET sadece okur
            let stil = alan(&govde, "stil").and_then(|v| v.parse::<u8>().ok()).unwrap_or(0);
            let islem = match alan(&govde, "islem") {
                Some("kaydet") if yontem == "POST" => 1,
                Some("dene") if yontem == "POST" => 2,
                _ => 0,
            };
            let j = match ham(KMT_ACILIS, &[islem, stil]) {
                Ok(c) if c[3] >= 14 && c[2] == 0 => format!("{{\"ok\":true,\"destek\":true,\"stil\":{}}}", c[4]),
                Ok(c) if c[3] >= 14 => "{\"ok\":false,\"hata\":\"Açılış animasyonu kaydedilemedi\"}".into(),
                Ok(_) => "{\"ok\":true,\"destek\":false}".into(),
                Err(e) => hata_json(&e),
            };
            cevapla(&mut s, "200 OK", "application/json", &j);
        }
        ("GET", "/api/tuslar") => cevapla(&mut s, "200 OK", "application/json", &tuslar_json()),
        ("POST", "/api/tuslar") => {
            // atama="led:kod,led:kod" (atanmayan tuşlar varsayılan); tablonun tamamı yazılır
            let mut a = vec![0xFFFFu16; TUS_SAYI];
            for p in url_coz(alan(&govde, "atama").unwrap_or("")).split(',') {
                if let Some((l, k)) = p.split_once(':') {
                    if let (Ok(l), Ok(k)) = (l.parse::<usize>(), k.parse::<u16>()) {
                        if l < TUS_SAYI && k <= 0x1FFF {
                            a[l] = k;
                        }
                    }
                }
            }
            let j = match tuslar_yaz(&a) {
                Ok(()) => {
                    tus_dosya_yaz(&a);
                    tuslar_json()
                }
                Err(e) => hata_json(&e),
            };
            cevapla(&mut s, "200 OK", "application/json", &j);
        }
        ("GET", "/api/surum-denetle") | ("POST", "/api/surum-denetle") => {
            let j = match son_surum(yontem == "POST") {
                Ok(v) => format!("{{\"ok\":true,\"son\":{}}}", json_metin(&v)),
                Err(e) => hata_json(&e),
            };
            cevapla(&mut s, "200 OK", "application/json", &j);
        }
        ("GET", "/api/monitorler") => {
            let secili = oto_al().monitor;
            let v = pusat_rgb::izle::monitorler();
            let liste = v
                .iter()
                .map(|m| format!("{{\"ad\":{},\"gen\":{},\"yuk\":{},\"sol\":{},\"ust\":{},\"birincil\":{}}}", json_metin(&m.ad), m.genislik, m.yuk, m.sol, m.ust, m.birincil))
                .collect::<Vec<_>>()
                .join(",");
            cevapla(&mut s, "200 OK", "application/json", &format!("{{\"ok\":true,\"secili\":{},\"monitorler\":[{liste}]}}", json_metin(&secili)));
        }
        ("POST", "/api/ekran-monitor") => {
            let mut o = oto_al();
            o.monitor = url_coz(alan(&govde, "ad").unwrap_or("")).chars().filter(|c| !c.is_control()).take(64).collect();
            let tamam = oto_kaydet(o);
            cevapla(&mut s, "200 OK", "application/json", &format!("{{\"ok\":true,\"kaydedildi\":{tamam}}}"));
        }
        ("GET", "/api/programlar") => {
            let v = pusat_rgb::izle::acik_programlar();
            cevapla(&mut s, "200 OK", "application/json", &format!("{{\"ok\":true,\"programlar\":[{}]}}", v.iter().map(|(e, a)| format!("{{\"exe\":{},\"ad\":{}}}", json_metin(e), json_metin(a))).collect::<Vec<_>>().join(",")));
        }
        ("POST", "/api/boya-oku") => {
            let p = alan(&govde, "p").and_then(|v| v.parse::<u8>().ok()).unwrap_or(0).min(3);
            let j = match boya_oku(p) {
                Ok(Some(v)) => format!("{{\"ok\":true,\"var\":true,\"renk\":[{}]}}", v.iter().map(|c| json_metin(&hex(*c))).collect::<Vec<_>>().join(",")),
                Ok(None) => "{\"ok\":true,\"var\":false}".into(),
                Err(e) => hata_json(&e),
            };
            cevapla(&mut s, "200 OK", "application/json", &j);
        }
        ("POST", "/api/boya-yaz") => {
            let p = alan(&govde, "p").and_then(|v| v.parse::<u8>().ok()).unwrap_or(0).min(3);
            let renk: Vec<[u8; 3]> = url_coz(alan(&govde, "renk").unwrap_or("")).split(',').map(|h| hex_coz(h).unwrap_or([0, 0, 0])).collect();
            let j = if renk.len() != BOYA_LED { "{\"ok\":false,\"hata\":\"Renk sayısı hatalı\"}".to_string() } else {
                match boya_yaz(p, &renk) { Ok(()) => "{\"ok\":true}".into(), Err(e) => hata_json(&e) }
            };
            cevapla(&mut s, "200 OK", "application/json", &j);
        }
        ("GET", "/api/bilgi") => cevapla(&mut s, "200 OK", "application/json", &bilgi_json()),
        ("GET", "/api/oku") => cevapla(&mut s, "200 OK", "application/json", &profilli(sonuc(klavye(KMT_OKU, None)))),
        ("POST", "/api/kenar-test") => {
            // i=0..17: o kenar LED'i tek başına beyaz yanar; i=255: bitir
            let i = govde.split('&').find_map(|kv| kv.strip_prefix("i=")).and_then(|v| v.parse::<u8>().ok()).unwrap_or(255);
            let j = match ham(KMT_KENAR_TEST, &[i]) {
                Ok(c) if c[3] >= 6 && c[2] == 0 => "{\"ok\":true,\"destek\":true}".to_string(),
                Ok(_) => "{\"ok\":true,\"destek\":false}".to_string(),
                Err(e) => hata_json(&e),
            };
            cevapla(&mut s, "200 OK", "application/json", &j);
        }
        ("POST", "/api/baglanti") => {
            // Sadece sabit bağlantılar açılır (arayüzden gelen adres kullanılmaz)
            let url = match govde.split('&').find_map(|kv| kv.strip_prefix("k=")) {
                Some("github") => Some("https://github.com/ERESB0S"),
                Some("surum") => Some("https://github.com/ERESB0S/Volt/releases/latest"),
                _ => None,
            };
            if let Some(u) = url { let _ = Command::new("rundll32").args(["url.dll,FileProtocolHandler", u]).spawn(); }
            cevapla(&mut s, "200 OK", "application/json", if url.is_some() { "{\"ok\":true}" } else { "{\"ok\":false,\"hata\":\"bilinmeyen bağlantı\"}" });
        }
        ("GET", "/api/kenar-harita") => {
            let j = kenar_harita_yolu().and_then(|y| std::fs::read_to_string(y).ok()).unwrap_or_else(|| "null".into());
            cevapla(&mut s, "200 OK", "application/json", &format!("{{\"ok\":true,\"harita\":{}}}", j));
        }
        ("POST", "/api/kenar-harita") => {
            // Gövde: h=<JSON dizi> (18 öğe; her biri yer adı ya da boş)
            let ham_h = govde.split('&').find_map(|kv| kv.strip_prefix("h=")).unwrap_or("");
            let h = url_coz(ham_h);
            let json_gibi = (h.starts_with('[') && h.ends_with(']')) || (h.starts_with('{') && h.ends_with('}'));
            let tamam = json_gibi && h.len() < 4096
                && kenar_harita_yolu().map(|y| { let _ = std::fs::create_dir_all(y.parent().unwrap()); std::fs::write(y, &h).is_ok() }).unwrap_or(false);
            cevapla(&mut s, "200 OK", "application/json", if tamam { "{\"ok\":true}" } else { "{\"ok\":false,\"hata\":\"kaydedilemedi\"}" });
        }
        ("GET", "/api/profil") => {
            // Hafif kontrol: sadece seçili profil (uygulama sık sorar, M tuşu anında görünsün)
            let j = match ham(KMT_BILGI, &[]) {
                Ok(c) if c[3] >= 5 && c[2] == 0 => format!("{{\"ok\":true,\"profil\":{},\"profiller\":{},\"zaman\":{},\"hiz\":{}}}", c[21], c[22],
                    u32::from_le_bytes([c[23], c[24], c[25], c[26]]), if c[3] >= 9 { c[27] } else { 0 }),
                Ok(_) => "{\"ok\":true,\"profil\":-1,\"profiller\":0}".into(),
                Err(e) => hata_json(&e),
            };
            cevapla(&mut s, "200 OK", "application/json", &j);
        }
        ("POST", "/api/profil") => {
            let p = govde.split('&').find_map(|kv| kv.strip_prefix("p=")).and_then(|v| v.parse::<u8>().ok()).unwrap_or(0).min(3);
            let r = ham(KMT_PROFIL, &[p]).map(|c| (c[2], Ayar::baytlardan(&c[4..32])));
            cevapla(&mut s, "200 OK", "application/json", &profilli(sonuc(r)));
        }
        ("POST", "/api/ayarla") => {
            let a = form_coz(&govde);
            cevapla(&mut s, "200 OK", "application/json", &sonuc(klavye(KMT_AYARLA, Some(&a))));
        }
        ("POST", "/api/kaydet") => {
            // Ekrandaki ayar gönderildiyse önce onu uygula, sonra kaydet (bekleyen değişiklik kaçmasın)
            let r = if govde.contains("mod=") {
                klavye(KMT_AYARLA, Some(&form_coz(&govde))).and_then(|_| klavye(KMT_KAYDET, None))
            } else {
                klavye(KMT_KAYDET, None)
            };
            cevapla(&mut s, "200 OK", "application/json", &sonuc(r));
        }
        ("POST", "/api/varsayilan") => cevapla(&mut s, "200 OK", "application/json", &sonuc(klavye(KMT_VARSAY, None))),
        ("POST", "/api/kayitli") => cevapla(&mut s, "200 OK", "application/json", &profilli(sonuc(klavye(KMT_KAYITLI, None)))),
        ("POST", "/api/profil-oku") => {
            // Bir profilin klavyede kayıtlı ayarı (yedek için)
            let p = alan(&govde, "p").and_then(|v| v.parse::<u8>().ok()).unwrap_or(0);
            let j = match ham(KMT_PROFIL_OKU, &[p]) {
                Ok(c) if c[3] < 8 => "{\"ok\":false,\"hata\":\"Bunun için klavye yazılımını güncelle\"}".into(),
                Ok(c) if c[2] == 2 => "{\"ok\":true,\"yok\":true}".into(),
                Ok(c) => Ayar::baytlardan(&c[4..32]).json(c[2]),
                Err(e) => hata_json(&e),
            };
            cevapla(&mut s, "200 OK", "application/json", &j);
        }
        ("POST", "/api/profil-yaz") => {
            // Bir profilin kaydını yaz (yedekten geri yükleme, profil içe aktarma)
            let p = alan(&govde, "p").and_then(|v| v.parse::<u8>().ok()).unwrap_or(255);
            let mut v = vec![p];
            v.extend_from_slice(&form_coz(&govde).baytlar());
            let j = match ham(KMT_PROFIL_YAZ, &v) {
                Ok(c) if c[3] < 8 => "{\"ok\":false,\"hata\":\"Bunun için klavye yazılımını güncelle\"}".into(),
                Ok(c) if c[2] != 0 => "{\"ok\":false,\"hata\":\"Profil kaydedilemedi\"}".into(),
                Ok(_) => "{\"ok\":true}".into(),
                Err(e) => hata_json(&e),
            };
            cevapla(&mut s, "200 OK", "application/json", &j);
        }
        ("POST", "/api/makro-oku") => {
            let p = alan(&govde, "p").and_then(|v| v.parse::<u8>().ok()).map(|p| p.min(3) + 1).unwrap_or(0);
            cevapla(&mut s, "200 OK", "application/json", &makro_json_p(p));
        }
        ("POST", "/api/makro-yaz") => {
            // p=profil n=G yuvası o=olaylar (virgülle, en çok 63; boş = makroyu sil)
            let p = alan(&govde, "p").and_then(|v| v.parse::<u8>().ok()).unwrap_or(255);
            let n = alan(&govde, "n").and_then(|v| v.parse::<u8>().ok()).unwrap_or(255);
            let o: Vec<u16> = url_coz(alan(&govde, "o").unwrap_or("")).split(',').filter_map(|x| x.trim().parse().ok()).collect();
            let j = (|| -> Result<()> {
                if p > 3 || n > 4 || o.len() > 63 {
                    bail!("Geçersiz makro");
                }
                let parcalar: Vec<&[u16]> = if o.is_empty() { vec![&[]] } else { o.chunks(12).collect() };
                let mut bas = 0u8;
                for (i, parca) in parcalar.iter().enumerate() {
                    let mut v = vec![p, n, bas, parca.len() as u8, (i + 1 == parcalar.len()) as u8, 0];
                    for x in parca.iter() {
                        v.extend_from_slice(&x.to_le_bytes());
                    }
                    let c = ham(KMT_MAKRO_YAZ, &v)?;
                    if c[3] < 8 {
                        bail!("Bunun için klavye yazılımını güncelle");
                    }
                    if c[2] != 0 {
                        bail!("Makro yazılamadı");
                    }
                    bas += parca.len() as u8;
                }
                Ok(())
            })();
            cevapla(&mut s, "200 OK", "application/json", &match j { Ok(()) => "{\"ok\":true}".into(), Err(e) => hata_json(&e) });
        }
        ("POST", "/api/dosyalar") => {
            // Belgeler\Volt\<alt> içindeki .json dosyaları (yeniden eskiye)
            let j = match volt_klasoru(&url_coz(alan(&govde, "alt").unwrap_or(""))) {
                None => "{\"ok\":false,\"hata\":\"Klasör açılamadı\"}".into(),
                Some(d) => {
                    let mut l: Vec<(std::time::SystemTime, String)> = std::fs::read_dir(&d)
                        .map(|r| {
                            r.flatten()
                                .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
                                .map(|e| (e.metadata().and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH), e.file_name().to_string_lossy().into_owned()))
                                .collect()
                        })
                        .unwrap_or_default();
                    l.sort_by(|a, b| b.0.cmp(&a.0));
                    let ogeler: Vec<String> = l
                        .iter()
                        .map(|(t, a)| format!("{{\"ad\":{},\"zaman\":{}}}", json_metin(a), t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)))
                        .collect();
                    format!("{{\"ok\":true,\"klasor\":{},\"dosyalar\":[{}]}}", json_metin(&d.display().to_string()), ogeler.join(","))
                }
            };
            cevapla(&mut s, "200 OK", "application/json", &j);
        }
        ("POST", "/api/dosya-oku") => {
            let yol = volt_klasoru(&url_coz(alan(&govde, "alt").unwrap_or(""))).zip(dosya_adi(&url_coz(alan(&govde, "ad").unwrap_or(""))));
            let j = match yol.and_then(|(d, a)| std::fs::read_to_string(d.join(a)).ok()) {
                Some(i) if i.len() < 256 * 1024 => format!("{{\"ok\":true,\"icerik\":{}}}", json_metin(&i)),
                _ => "{\"ok\":false,\"hata\":\"Dosya okunamadı\"}".into(),
            };
            cevapla(&mut s, "200 OK", "application/json", &j);
        }
        ("POST", "/api/dosya-yaz") => {
            let yol = volt_klasoru(&url_coz(alan(&govde, "alt").unwrap_or(""))).zip(dosya_adi(&url_coz(alan(&govde, "ad").unwrap_or(""))));
            let icerik = url_coz(alan(&govde, "icerik").unwrap_or(""));
            let j = match yol {
                Some((d, a)) if icerik.starts_with('{') && std::fs::write(d.join(&a), &icerik).is_ok() => format!("{{\"ok\":true,\"ad\":{}}}", json_metin(&a)),
                _ => "{\"ok\":false,\"hata\":\"Dosya kaydedilemedi (adda sadece harf, rakam, boşluk, - _ kullan)\"}".into(),
            };
            cevapla(&mut s, "200 OK", "application/json", &j);
        }
        ("POST", "/api/dosya-sil") => {
            let yol = volt_klasoru(&url_coz(alan(&govde, "alt").unwrap_or(""))).zip(dosya_adi(&url_coz(alan(&govde, "ad").unwrap_or(""))));
            let tamam = yol.is_some_and(|(d, a)| std::fs::remove_file(d.join(a)).is_ok());
            cevapla(&mut s, "200 OK", "application/json", if tamam { "{\"ok\":true}" } else { "{\"ok\":false,\"hata\":\"Silinemedi\"}" });
        }
        ("POST", "/api/klasor-ac") => {
            if let Some(d) = volt_klasoru(&url_coz(alan(&govde, "alt").unwrap_or(""))) {
                let _ = Command::new("explorer").arg(d).spawn();
            }
            cevapla(&mut s, "200 OK", "application/json", "{\"ok\":true}");
        }
        ("GET", "/api/yazilim") => {
            // Volt'un içindeki klavye yazılımı
            cevapla(&mut s, "200 OK", "application/json", &format!("{{\"ok\":true,\"protokol\":{YAZILIM_PROTOKOL},\"boyut\":{}}}", YAZILIM.len()));
        }
        ("POST", "/api/guncelle") => {
            guncelle_baslat(alan(&govde, "hedef").unwrap_or("volt"));
            cevapla(&mut s, "200 OK", "application/json", "{\"ok\":true}");
        }
        ("GET", "/api/klavye") => {
            let d = if GUNCELLENIYOR.load(Ordering::Relaxed) { "gecis" } else { pusat_rgb::guncelle::klavye_durumu() };
            cevapla(&mut s, "200 OK", "application/json", &format!("{{\"ok\":true,\"durum\":\"{d}\"}}"));
        }
        ("GET", "/api/kurulum") => {
            let j = format!(
                "{{\"ok\":true,\"kurulu\":{},\"bu_kurulu\":{},\"yol\":{}}}",
                pusat_rgb::kurulum::kurulu(),
                pusat_rgb::kurulum::bu_kurulu(),
                json_metin(&pusat_rgb::kurulum::klasor().display().to_string())
            );
            cevapla(&mut s, "200 OK", "application/json", &j);
        }
        ("POST", "/api/cikis") | ("POST", "/api/kaldir") => {
            // Güncelleme sürerken asla kapanma (klavye yarım kalır)
            if GUNCELLENIYOR.load(Ordering::Relaxed) {
                cevapla(&mut s, "200 OK", "application/json", "{\"ok\":false,\"hata\":\"Klavye yazılımı yükleniyor, bitmesini bekle.\"}");
            } else {
                if yol == "/api/kaldir" {
                    pusat_rgb::kurulum::kaldir(alan(&govde, "ayarlar") == Some("1"));
                }
                cevapla(&mut s, "200 OK", "application/json", "{\"ok\":true}");
                std::thread::spawn(|| {
                    std::thread::sleep(Duration::from_millis(400));
                    olay_gonder(Olay::Cikis);
                    std::thread::sleep(Duration::from_millis(1500));
                    std::process::exit(0);
                });
            }
        }
        ("GET", "/api/guncelle") => {
            let (a, i, h) = GUNCELLE_DURUM.lock().unwrap_or_else(|e| e.into_inner()).clone();
            cevapla(&mut s, "200 OK", "application/json", &format!("{{\"ok\":true,\"asama\":{},\"ilerleme\":{i:.3},\"hata\":{}}}", json_metin(&a), json_metin(&h)));
        }
        ("GET", "/api/otobaslat") => cevapla(&mut s, "200 OK", "application/json", &format!("{{\"ok\":true,\"acik\":{}}}", otobaslat_acik())),
        ("POST", "/api/otobaslat") => {
            let tamam = otobaslat_ayarla(alan(&govde, "acik") == Some("1"));
            cevapla(&mut s, "200 OK", "application/json", &format!("{{\"ok\":{tamam},\"acik\":{}}}", otobaslat_acik()));
        }
        ("GET", "/api/goster") => {
            // İkinci kez açılınca: çalışan Volt penceresini öne getir
            olay_gonder(Olay::Goster);
            cevapla(&mut s, "200 OK", "text/plain", "ok");
        }
        ("POST", "/api/kapat") => {
            cevapla(&mut s, "200 OK", "text/plain", "hoşça kal");
            return true;
        }
        _ => cevapla(&mut s, "404 Not Found", "text/plain", "yok"),
    }
    false
}

fn pencere_ac() {
    let url = format!("http://127.0.0.1:{PORT}/");
    let edge_yollari = [
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
    ];
    for yol in edge_yollari {
        if std::path::Path::new(yol).exists()
            && Command::new(yol)
                .arg(format!("--app={url}"))
                .arg("--window-size=1240,860")
                .spawn()
                .is_ok()
        {
            return;
        }
    }
    // Edge yoksa varsayılan tarayıcı
    let _ = Command::new("rundll32").args(["url.dll,FileProtocolHandler", &url]).spawn();
}

/// Pencereden (sayfadaki özel başlık çubuğundan) gelen istekler
#[derive(Debug)]
enum Olay {
    Surukle,
    Boyut(tao::window::ResizeDirection),
    Kucult,
    Buyut,
    Kapat,
    Goster,
    Cikis,
    /// Arayüz teması değişti (true: açık)
    Tema(bool),
}

/// Uygulama simgesi: koyu yuvarlak kare üzerinde beyaz şimşek (n x n RGBA)
fn simge(n: u32) -> Option<tao::window::Icon> {
    tao::window::Icon::from_rgba(simge_rgba(n), n, n).ok()
}

fn simge_rgba(n: u32) -> Vec<u8> {
    let mut v = vec![0u8; (n * n * 4) as usize];
    let simsek = [(0.58, 0.10), (0.26, 0.56), (0.48, 0.56), (0.40, 0.90), (0.74, 0.42), (0.52, 0.42), (0.58, 0.10)];
    let icinde = |x: f32, y: f32| {
        let mut ic = false;
        for i in 0..simsek.len() - 1 {
            let ((x1, y1), (x2, y2)) = (simsek[i], simsek[i + 1]);
            if (y1 > y) != (y2 > y) && x < (x2 - x1) * (y - y1) / (y2 - y1) + x1 {
                ic = !ic;
            }
        }
        ic
    };
    for py in 0..n {
        for px in 0..n {
            let (x, y) = ((px as f32 + 0.5) / n as f32, (py as f32 + 0.5) / n as f32);
            // yuvarlak köşeli kare
            let r = 0.22;
            let (dx, dy) = ((x - 0.5).abs() - (0.5 - r), (y - 0.5).abs() - (0.5 - r));
            let disari = (dx.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt() > r;
            let i = ((py * n + px) * 4) as usize;
            if disari {
                continue;
            }
            let c = if icinde(x, y) { [245, 245, 247, 255] } else { [24, 24, 27, 255] };
            v[i..i + 4].copy_from_slice(&c);
        }
    }
    v
}

/// Kendi penceremiz: çerçevesiz, başlık çubuğunu sayfa çiziyor (WebView2).
fn pencere(arka: bool) -> Result<()> {
    use tao::dpi::LogicalSize;
    use tao::event::{Event, WindowEvent};
    use tao::event_loop::{ControlFlow, EventLoopBuilder};
    use tao::platform::windows::WindowBuilderExtWindows;
    use tao::window::{ResizeDirection as Y, WindowBuilder};
    use wry::{WebContext, WebViewBuilder, WebViewBuilderExtWindows};

    let dongu = EventLoopBuilder::<Olay>::with_user_event().build();
    let vekil = dongu.create_proxy();
    *VEKIL.lock().unwrap_or_else(|e| e.into_inner()) = Some(dongu.create_proxy());
    {
        let v = dongu.create_proxy();
        pusat_rgb::tepsi::baslat(simge_rgba(32), 32, move |o| {
            let _ = v.send_event(match o {
                pusat_rgb::tepsi::TepsiOlay::Goster => Olay::Goster,
                pusat_rgb::tepsi::TepsiOlay::Cikis => Olay::Cikis,
                pusat_rgb::tepsi::TepsiOlay::Yanip(h) => return pencere_yanip(h),
            });
        });
    }
    let pen = WindowBuilder::new()
        .with_title("Volt")
        .with_decorations(false)
        .with_undecorated_shadow(true)
        .with_inner_size(LogicalSize::new(1280.0, 820.0))
        .with_min_inner_size(LogicalSize::new(980.0, 620.0))
        .with_window_icon(simge(32))
        .with_taskbar_icon(simge(64))
        .with_visible(false)
        .build(&dongu)
        .context("Pencere açılamadı")?;

    let veri = std::env::var("LOCALAPPDATA").map(|d| std::path::PathBuf::from(d).join("Volt")).ok();
    // Son kullanılan tema: pencere açılırken arka plan aynı renkte olsun
    let tema_dosya = veri.as_ref().map(|v| v.join("tema.txt"));
    let acik_tema = tema_dosya.as_ref().and_then(|d| std::fs::read_to_string(d).ok()).is_some_and(|t| t.trim() == "acik");
    let mut baglam = WebContext::new(veri);
    let gorunum = WebViewBuilder::new_with_web_context(&mut baglam)
        .with_url(format!("http://127.0.0.1:{PORT}/{}", if KALDIR_MODU.load(Ordering::Relaxed) { "#kaldir" } else { "" }))
        .with_background_color(if acik_tema { (220, 220, 225, 255) } else { (10, 10, 11, 255) })
        .with_devtools(false)
        .with_hotkeys_zoom(false)
        .with_browser_accelerator_keys(false)
        .with_default_context_menus(false)
        .with_ipc_handler(move |istek| {
            let o = match istek.body().as_str() {
                "surukle" => Olay::Surukle,
                "kucult" => Olay::Kucult,
                "buyut" => Olay::Buyut,
                "kapat" => Olay::Kapat,
                "tema:acik" => Olay::Tema(true),
                "tema:koyu" => Olay::Tema(false),
                "n" => Olay::Boyut(Y::North),
                "s" => Olay::Boyut(Y::South),
                "e" => Olay::Boyut(Y::East),
                "w" => Olay::Boyut(Y::West),
                "ne" => Olay::Boyut(Y::NorthEast),
                "nw" => Olay::Boyut(Y::NorthWest),
                "se" => Olay::Boyut(Y::SouthEast),
                "sw" => Olay::Boyut(Y::SouthWest),
                _ => return,
            };
            let _ = vekil.send_event(o);
        })
        .build(&pen)
        .context("WebView2 başlatılamadı")?;
    if !arka {
        pen.set_visible(true);
    }
    let mut ilk_gizleme = true;

    dongu.run(move |olay, _, akis| {
        *akis = ControlFlow::Wait;
        match olay {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } | Event::UserEvent(Olay::Kapat) => {
                // Kapatınca tepsiye iner: ses modu ve profiller çalışmaya devam eder
                pen.set_visible(false);
                if ilk_gizleme {
                    ilk_gizleme = false;
                    pusat_rgb::tepsi::bildirim("Volt arka planda çalışıyor", "Tamamen kapatmak için tepsideki simgeye sağ tıkla → Çıkış");
                }
            }
            Event::UserEvent(Olay::Goster) => {
                pen.set_visible(true);
                pen.set_minimized(false);
                pen.set_focus();
            }
            Event::UserEvent(Olay::Cikis) => {
                pusat_rgb::tepsi::kaldir();
                std::process::exit(0)
            }
            Event::UserEvent(Olay::Surukle) => {
                let _ = pen.drag_window();
            }
            Event::UserEvent(Olay::Boyut(y)) => {
                if !pen.is_maximized() {
                    let _ = pen.drag_resize_window(y);
                }
            }
            Event::UserEvent(Olay::Tema(acik)) => {
                let _ = gorunum.set_background_color(if acik { (220, 220, 225, 255) } else { (10, 10, 11, 255) });
                if let Some(d) = &tema_dosya {
                    let _ = std::fs::write(d, if acik { "acik" } else { "koyu" });
                }
            }
            Event::UserEvent(Olay::Kucult) => pen.set_minimized(true),
            Event::UserEvent(Olay::Buyut) => pen.set_maximized(!pen.is_maximized()),
            Event::WindowEvent { event: WindowEvent::Resized(_), .. } => {
                let _ = gorunum.evaluate_script(&format!("window.pencereBuyuk && pencereBuyuk({})", pen.is_maximized()));
            }
            _ => {}
        }
    })
}

fn main() {
    let arka = std::env::args().any(|a| a == "--arka");
    let kaldir = std::env::args().any(|a| a == "--kaldir");
    KALDIR_MODU.store(kaldir, Ordering::Relaxed);
    if kaldir {
        // Çalışan bir Volt varsa kapanmasını iste (klavye yüklemesi sürüyorsa kapanmaz, o pencere öne gelir)
        if let Ok(mut s) = TcpStream::connect(("127.0.0.1", PORT)) {
            let _ = s.write_all(format!("POST /api/cikis HTTP/1.1\r\nHost: 127.0.0.1:{PORT}\r\nContent-Length: 0\r\n\r\n").as_bytes());
            let _ = s.read(&mut [0u8; 256]);
            for _ in 0..30 {
                std::thread::sleep(Duration::from_millis(200));
                if TcpListener::bind(("127.0.0.1", PORT)).is_ok() {
                    break;
                }
            }
        }
    }
    let dinleyici = match TcpListener::bind(("127.0.0.1", PORT)) {
        Ok(d) => d,
        Err(_) => {
            // Zaten açık (belki tepside): o pencereyi öne getir
            if !arka {
                if let Ok(mut s) = TcpStream::connect(("127.0.0.1", PORT)) {
                    let _ = s.write_all(format!("GET /api/goster HTTP/1.1\r\nHost: 127.0.0.1:{PORT}\r\n\r\n").as_bytes());
                    let _ = s.read(&mut [0u8; 64]);
                }
            }
            return;
        }
    };
    // Pencere kapanınca "kapat" gelir; sayfa yenilendiyse hemen yeni istek gelir.
    // Bu yüzden kapanmadan önce 3 sn bekle, bu arada istek gelmediyse çık.
    std::thread::spawn(move || {
        static ISTEK_SAYACI: AtomicU64 = AtomicU64::new(0);
        for baglanti in dinleyici.incoming() {
            let Ok(s) = baglanti else { continue };
            ISTEK_SAYACI.fetch_add(1, Ordering::SeqCst);
            if istek(s) {
                let o_an = ISTEK_SAYACI.load(Ordering::SeqCst);
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_secs(3));
                    if ISTEK_SAYACI.load(Ordering::SeqCst) == o_an {
                        std::process::exit(0);
                    }
                });
            }
        }
    });
    ses_baslat();
    otomasyon_baslat();
    bildirim_dinle();
    istatistik_baslat();
    if pencere(arka).is_err() {
        // WebView2 yoksa Edge uygulama penceresiyle devam et
        pencere_ac();
        loop {
            std::thread::park();
        }
    }
}
