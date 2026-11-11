// Copyright 2026 Eresbos (github.com/ERESB0S)
// SPDX-License-Identifier: GPL-3.0-or-later
//! Windows'tan okunan durumlar: ekranın renkleri (ambilight), ön plandaki program,
//! açık pencerelerin programları ve mikrofonun sessizde olup olmadığı.

use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateCompatibleBitmap, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC,
    EnumDisplayMonitors, GetDIBits, GetMonitorInfoW, HALFTONE, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW, ReleaseDC, SRCCOPY, SelectObject,
    SetBrushOrgEx, SetStretchBltMode, StretchBlt,
};
use windows::Win32::Foundation::RECT;
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{IMMDeviceEnumerator, MMDeviceEnumerator, eCapture, eCommunications};
use windows::Win32::System::Com::{CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetForegroundWindow, GetSystemMetrics, GetWindowTextLengthW, GetWindowThreadProcessId, IsWindowVisible, SM_CXSCREEN,
    SM_CYSCREEN,
};
use windows::Win32::Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW};
use windows::core::{BOOL, PCWSTR, PWSTR};

pub const BOLGE: usize = 8;

pub const KENAR: usize = 4;

/// Bir piksel kümesinin ortalaması: doygunluğu artırılmış, çok karanlıksa siyah
fn ortalama(px: &[u8], w: usize, xs: std::ops::Range<usize>, ys: std::ops::Range<usize>) -> [u8; 3] {
    let mut t = [0u32; 3];
    let say = (xs.len() * ys.len()).max(1) as f32;
    for y in ys {
        for x in xs.clone() {
            let i = (y * w + x) * 4; // BGRA
            t[0] += px[i + 2] as u32;
            t[1] += px[i + 1] as u32;
            t[2] += px[i] as u32;
        }
    }
    let c = [t[0] as f32 / say, t[1] as f32 / say, t[2] as f32 / say];
    let ort = (c[0] + c[1] + c[2]) / 3.0;
    let enb = c[0].max(c[1]).max(c[2]);
    if enb < 14.0 {
        return [0; 3]; // siyaha yakın: ışık sönük
    }
    // Rengi doygunlaştır, sonra parlaklığı normalleştir: ortalama alınca çoğu bölge koyu kalıyor,
    // klavyedeki gama eğrisi de koyu tonları iyice kıstığı için ışıklar sönük görünüyordu.
    // En parlak kanal ~48 ve üstündeyse tam parlaklığa çıkar, 14..48 arasında yumuşakça artar.
    let v: [f32; 3] = std::array::from_fn(|k| (ort + (c[k] - ort) * 1.7).clamp(0.0, 255.0));
    let vmax = v[0].max(v[1]).max(v[2]).max(1.0);
    let hedef = if enb >= 48.0 { 255.0 } else { 60.0 + (enb - 14.0) / 34.0 * 195.0 };
    std::array::from_fn(|k| (v[k] * hedef / vmax).clamp(0.0, 255.0) as u8)
}

/// Ekran renkleri: 8 dikey bölge (soldan sağa) ve kenarlar (sol 4 + sağ 4, üstten alta), RGB.
pub struct EkranKare {
    pub bolge: [u8; BOLGE * 3],
    pub kenar: [u8; 2 * KENAR * 3],
}

/// Bir monitör: Windows'taki aygıt adı (ör. \\.\DISPLAY2) ve masaüstündeki yeri
#[derive(Clone)]
pub struct Monitor {
    pub ad: String,
    pub sol: i32,
    pub ust: i32,
    pub genislik: i32,
    pub yuk: i32,
    pub birincil: bool,
}

unsafe extern "system" fn monitor_say(h: HMONITOR, _: HDC, _: *mut RECT, l: LPARAM) -> BOOL {
    unsafe {
        let v = &mut *(l.0 as *mut Vec<Monitor>);
        let mut mi = MONITORINFOEXW::default();
        mi.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
        if GetMonitorInfoW(h, &mut mi as *mut MONITORINFOEXW as *mut MONITORINFO).as_bool() {
            let r = mi.monitorInfo.rcMonitor;
            let n = mi.szDevice.iter().position(|&c| c == 0).unwrap_or(mi.szDevice.len());
            v.push(Monitor {
                ad: String::from_utf16_lossy(&mi.szDevice[..n]),
                sol: r.left,
                ust: r.top,
                genislik: r.right - r.left,
                yuk: r.bottom - r.top,
                birincil: mi.monitorInfo.dwFlags & 1 != 0,
            });
        }
    }
    true.into()
}

/// Bağlı monitörler: önce ana monitör, sonra soldan sağa
pub fn monitorler() -> Vec<Monitor> {
    let mut v: Vec<Monitor> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(None, None, Some(monitor_say), LPARAM(&mut v as *mut _ as isize));
    }
    v.sort_by_key(|m| (!m.birincil, m.sol, m.ust));
    v
}

/// Seçili monitörü (aygıt adıyla; bulunamazsa ana monitör) küçültüp renklerini çıkarır.
pub fn ekran_renkleri(monitor: &str) -> Option<EkranKare> {
    let ms = monitorler();
    let m = ms.iter().find(|m| m.ad == monitor).or(ms.first());
    let (sx, sy, sw, sh) = match m {
        Some(m) => (m.sol, m.ust, m.genislik, m.yuk),
        None => unsafe { (0, 0, GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) },
    };
    const W: i32 = 64;
    const H: i32 = 18;
    let mut px = vec![0u8; (W * H * 4) as usize];
    unsafe {
        let ekran = GetDC(None);
        if ekran.is_invalid() {
            return None;
        }
        let dc = CreateCompatibleDC(Some(ekran));
        let bmp = CreateCompatibleBitmap(ekran, W, H);
        let eski = SelectObject(dc, bmp.into());
        SetStretchBltMode(dc, HALFTONE);
        let _ = SetBrushOrgEx(dc, 0, 0, None);
        let ok = StretchBlt(dc, 0, 0, W, H, Some(ekran), sx, sy, sw, sh, SRCCOPY).as_bool();
        SelectObject(dc, eski);
        let mut bi = BITMAPINFO::default();
        bi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bi.bmiHeader.biWidth = W;
        bi.bmiHeader.biHeight = -H; // üstten alta
        bi.bmiHeader.biPlanes = 1;
        bi.bmiHeader.biBitCount = 32;
        bi.bmiHeader.biCompression = BI_RGB.0;
        let n = GetDIBits(dc, bmp, 0, H as u32, Some(px.as_mut_ptr() as *mut _), &mut bi, DIB_RGB_COLORS);
        let _ = DeleteObject(bmp.into());
        let _ = DeleteDC(dc);
        ReleaseDC(None, ekran);
        if !ok || n == 0 {
            return None;
        }
    }
    let (w, h) = (W as usize, H as usize);
    let genis = w / BOLGE;
    let mut o = EkranKare { bolge: [0; BOLGE * 3], kenar: [0; 2 * KENAR * 3] };
    for z in 0..BOLGE {
        o.bolge[z * 3..z * 3 + 3].copy_from_slice(&ortalama(&px, w, z * genis..(z + 1) * genis, 0..h));
    }
    // Kenarlar: en dıştaki 1/8 genişlik, 4 yatay şerit
    for (taraf, xs) in [(0, 0..genis), (1, w - genis..w)] {
        for n in 0..KENAR {
            let ys = n * h / KENAR..(n + 1) * h / KENAR;
            let i = (taraf * KENAR + n) * 3;
            o.kenar[i..i + 3].copy_from_slice(&ortalama(&px, w, xs.clone(), ys));
        }
    }
    Some(o)
}

/// Pencerenin programının dosya adı (küçük harf, ör. "discord.exe")
pub fn pencere_exe(h: HWND) -> Option<String> {
    let yol = pencere_yolu(h)?;
    Some(yol.rsplit('\\').next()?.to_lowercase())
}

/// Pencerenin programının tam yolu
fn pencere_yolu(h: HWND) -> Option<String> {
    unsafe {
        let mut pid = 0u32;
        GetWindowThreadProcessId(h, Some(&mut pid));
        if pid == 0 {
            return None;
        }
        let p = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 520];
        let mut n = buf.len() as u32;
        let r = QueryFullProcessImageNameW(p, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut n);
        let _ = CloseHandle(p);
        r.ok()?;
        Some(String::from_utf16_lossy(&buf[..n as usize]))
    }
}

fn genis(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Programın görünen adı (dosya bilgisindeki "Dosya açıklaması", yoksa ürün adı). Ör. Discord.exe -> "Discord"
pub fn program_adi(yol: &str) -> Option<String> {
    static ON_BELLEK: std::sync::Mutex<Vec<(String, Option<String>)>> = std::sync::Mutex::new(Vec::new());
    if let Some((_, a)) = ON_BELLEK.lock().ok()?.iter().find(|(y, _)| y == yol) {
        return a.clone();
    }
    let ad = unsafe {
        let dosya = genis(yol);
        let boy = GetFileVersionInfoSizeW(PCWSTR(dosya.as_ptr()), None);
        let mut veri = vec![0u8; boy as usize];
        if boy == 0 || GetFileVersionInfoW(PCWSTR(dosya.as_ptr()), None, boy, veri.as_mut_ptr() as *mut _).is_err() {
            None
        } else {
            let sorgu = |yol: &str| -> Option<String> {
                let mut p: *mut std::ffi::c_void = std::ptr::null_mut();
                let mut n = 0u32;
                if !VerQueryValueW(veri.as_ptr() as *const _, PCWSTR(genis(yol).as_ptr()), &mut p, &mut n).as_bool() || p.is_null() || n == 0 {
                    return None;
                }
                let d = std::slice::from_raw_parts(p as *const u16, n as usize);
                let t = String::from_utf16_lossy(&d[..d.iter().position(|&c| c == 0).unwrap_or(d.len())]).trim().to_string();
                (!t.is_empty()).then_some(t)
            };
            // Dil kodları: dosyanın kendi çevirisi, sonra ABD İngilizcesi
            let mut diller = Vec::new();
            let mut p: *mut std::ffi::c_void = std::ptr::null_mut();
            let mut n = 0u32;
            if VerQueryValueW(veri.as_ptr() as *const _, PCWSTR(genis("\\VarFileInfo\\Translation").as_ptr()), &mut p, &mut n).as_bool() && n >= 4 {
                let c = std::slice::from_raw_parts(p as *const u16, 2);
                diller.push(format!("{:04x}{:04x}", c[0], c[1]));
            }
            diller.extend(["040904b0".to_string(), "040904e4".to_string()]);
            ["FileDescription", "ProductName"].iter().find_map(|alan| diller.iter().find_map(|d| sorgu(&format!("\\StringFileInfo\\{d}\\{alan}"))))
        }
    };
    if let Ok(mut o) = ON_BELLEK.lock() {
        o.push((yol.to_string(), ad.clone()));
    }
    ad
}

/// Öndeki programın görünen adı
pub fn on_plan_adi() -> Option<String> {
    let h = unsafe { GetForegroundWindow() };
    if h.0.is_null() { None } else { program_adi(&pencere_yolu(h)?) }
}

/// Şu an öndeki pencerenin programı
pub fn on_plan_exe() -> Option<String> {
    let h = unsafe { GetForegroundWindow() };
    if h.0.is_null() { None } else { pencere_exe(h) }
}

unsafe extern "system" fn pencere_say(h: HWND, l: LPARAM) -> BOOL {
    unsafe {
        let v = &mut *(l.0 as *mut Vec<(String, String)>);
        if IsWindowVisible(h).as_bool() && GetWindowTextLengthW(h) > 0 {
            if let Some(y) = pencere_yolu(h) {
                let e = y.rsplit('\\').next().unwrap_or("").to_lowercase();
                if !e.is_empty() && !v.iter().any(|(x, _)| *x == e) {
                    v.push((e, y));
                }
            }
        }
    }
    true.into()
}

/// Açık (görünür, başlıklı) pencereleri olan programlar: (dosya adı, görünen ad)
pub fn acik_programlar() -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(pencere_say), LPARAM(&mut v as *mut _ as isize));
    }
    let mut o: Vec<(String, String)> = v.into_iter().map(|(e, y)| (e, program_adi(&y).unwrap_or_default())).collect();
    o.sort_by_key(|(e, a)| if a.is_empty() { e.clone() } else { a.to_lowercase() });
    o
}

/// Varsayılan iletişim mikrofonu Windows'ta sessize alınmış mı? (iş parçacığı başına bir kez com_baslat çağır)
pub fn mikrofon_sessiz() -> Option<bool> {
    unsafe {
        let e: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
        let d = e.GetDefaultAudioEndpoint(eCapture, eCommunications).ok()?;
        let v: IAudioEndpointVolume = d.Activate(CLSCTX_ALL, None).ok()?;
        Some(v.GetMute().ok()?.as_bool())
    }
}

pub fn com_baslat() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
}

/* ---------------- Sistem göstergesi: CPU, RAM, GPU kullanımı ---------------- */
/// Sistem ölçer: CPU için iki ölçüm arası fark, GPU için Windows performans sayacı (3D motoru).
/// Aynı iş parçacığında kullanılmalı (PDH tutamakları).
pub struct SistemOlcer {
    cpu_once: Option<(u64, u64)>,
    sorgu: windows::Win32::System::Performance::PDH_HQUERY,
    gpu: Option<windows::Win32::System::Performance::PDH_HCOUNTER>,
}

fn ft(f: windows::Win32::Foundation::FILETIME) -> u64 {
    ((f.dwHighDateTime as u64) << 32) | f.dwLowDateTime as u64
}

impl SistemOlcer {
    pub fn yeni() -> Self {
        use windows::Win32::System::Performance::*;
        let mut sorgu = PDH_HQUERY::default();
        let mut gpu = None;
        unsafe {
            if PdhOpenQueryW(PCWSTR::null(), 0, &mut sorgu) == 0 {
                let mut c = PDH_HCOUNTER::default();
                if PdhAddEnglishCounterW(sorgu, windows::core::w!(r"\GPU Engine(*engtype_3D)\Utilization Percentage"), 0, &mut c) == 0 {
                    gpu = Some(c);
                }
                let _ = PdhCollectQueryData(sorgu);
            }
        }
        SistemOlcer { cpu_once: None, sorgu, gpu }
    }

    /// İşlemci kullanımı (0..100); ilk çağrıda None
    pub fn cpu(&mut self) -> Option<f32> {
        use windows::Win32::Foundation::FILETIME;
        let (mut bos, mut cek, mut kul) = (FILETIME::default(), FILETIME::default(), FILETIME::default());
        unsafe { windows::Win32::System::Threading::GetSystemTimes(Some(&mut bos), Some(&mut cek), Some(&mut kul)).ok()? };
        let (b, t) = (ft(bos), ft(cek) + ft(kul)); // çekirdek süresi boşta süreyi de içerir
        let once = self.cpu_once.replace((b, t));
        let (ob, ot) = once?;
        let dt = t.saturating_sub(ot);
        if dt == 0 {
            return None;
        }
        Some((100.0 * (1.0 - b.saturating_sub(ob) as f32 / dt as f32)).clamp(0.0, 100.0))
    }

    /// Bellek doluluğu (0..100)
    pub fn ram() -> Option<f32> {
        use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
        let mut m = MEMORYSTATUSEX { dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32, ..Default::default() };
        unsafe { GlobalMemoryStatusEx(&mut m).ok()? };
        Some(m.dwMemoryLoad as f32)
    }

    /// Ekran kartı (3D) kullanımı (0..100); sayaç yoksa None
    pub fn gpu(&mut self) -> Option<f32> {
        use windows::Win32::System::Performance::*;
        let c = self.gpu?;
        unsafe {
            if PdhCollectQueryData(self.sorgu) != 0 {
                return None;
            }
            let (mut boyut, mut adet) = (0u32, 0u32);
            let r = PdhGetFormattedCounterArrayW(c, PDH_FMT_DOUBLE, &mut boyut, &mut adet, None);
            if r != PDH_MORE_DATA || boyut == 0 {
                return Some(0.0);
            }
            let mut tampon = vec![0u64; boyut as usize / 8 + 1];
            let p = tampon.as_mut_ptr() as *mut PDH_FMT_COUNTERVALUE_ITEM_W;
            if PdhGetFormattedCounterArrayW(c, PDH_FMT_DOUBLE, &mut boyut, &mut adet, Some(p)) != 0 {
                return None;
            }
            let ogeler = std::slice::from_raw_parts(p, adet as usize);
            let t: f64 = ogeler.iter().filter(|o| o.FmtValue.CStatus == 0).map(|o| o.FmtValue.Anonymous.doubleValue).sum();
            Some((t as f32).clamp(0.0, 100.0))
        }
    }
}

impl Drop for SistemOlcer {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::System::Performance::PdhCloseQuery(self.sorgu);
        }
    }
}

/// Bugünün tarihi (yerel saat), "YYYY-AA-GG"
pub fn bugun() -> String {
    let s = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
    format!("{:04}-{:02}-{:02}", s.wYear, s.wMonth, s.wDay)
}

/* ---------------- Windows bildirimleri (bildirim ışığı) ---------------- */
/// Windows'un bildirim merkezine düşen yeni bildirimleri okur (Discord, WhatsApp, Teams...).
/// Program penceresi tepsideyken de çalışır. Windows "Bildirimlere erişim" iznini ister.
pub struct BildirimDinleyici {
    l: windows::UI::Notifications::Management::UserNotificationListener,
    gorulen: std::collections::HashSet<u32>,
    ilk: bool,
}

/// Yeni bildirim: (görünen ad, eşleştirme anahtarı = ad + AUMID, küçük harf, boşluksuz)
pub struct YeniBildirim {
    pub ad: String,
    pub anahtar: String,
}

impl BildirimDinleyici {
    pub fn yeni() -> Result<Self, String> {
        use windows::UI::Notifications::Management::{UserNotificationListener, UserNotificationListenerAccessStatus as D};
        let l = UserNotificationListener::Current().map_err(|e| e.message().to_string())?;
        let mut d = l.GetAccessStatus().map_err(|e| e.message().to_string())?;
        if d == D::Unspecified {
            d = l.RequestAccessAsync().and_then(|a| a.join()).unwrap_or(D::Denied);
        }
        if d != D::Allowed {
            return Err("izin yok".into());
        }
        Ok(Self { l, gorulen: Default::default(), ilk: true })
    }

    /// Son çağrıdan beri gelen bildirimler (ilk çağrıda eskiler sadece işaretlenir)
    pub fn yeniler(&mut self) -> Vec<YeniBildirim> {
        use windows::UI::Notifications::NotificationKinds;
        let mut v = Vec::new();
        let Ok(liste) = self.l.GetNotificationsAsync(NotificationKinds::Toast).and_then(|a| a.join()) else { return v };
        let n = liste.Size().unwrap_or(0);
        let mut simdiki = std::collections::HashSet::new();
        for i in 0..n {
            let Ok(b) = liste.GetAt(i) else { continue };
            let Ok(id) = b.Id() else { continue };
            simdiki.insert(id);
            if self.gorulen.contains(&id) || self.ilk {
                continue;
            }
            let bilgi = b.AppInfo().ok();
            let ad = bilgi.as_ref().and_then(|a| a.DisplayInfo().ok()).and_then(|d| d.DisplayName().ok()).map(|s| s.to_string()).unwrap_or_default();
            let aumid = bilgi.as_ref().and_then(|a| a.AppUserModelId().ok()).map(|s| s.to_string()).unwrap_or_default();
            let anahtar = format!("{ad}|{aumid}").to_lowercase().replace([' ', '-', '_'], "");
            v.push(YeniBildirim { ad, anahtar });
        }
        self.gorulen = simdiki;
        self.ilk = false;
        v
    }
}
