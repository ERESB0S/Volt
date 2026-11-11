// Copyright 2026 Eresbos (github.com/ERESB0S)
// SPDX-License-Identifier: GPL-3.0-or-later
//! Sistem tepsisi (bildirim alanı) simgesi: tıklayınca pencereyi gösterir, sağ tıkta Göster / Çıkış menüsü.
//! Kendi gizli penceresi ve mesaj döngüsüyle ayrı bir iş parçacığında çalışır.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicIsize, Ordering};

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_INFO, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateIcon, RegisterShellHookWindow, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DispatchMessageW, GetCursorPos,
    GetMessageW, HICON, MF_SEPARATOR, MF_STRING, MSG, RegisterClassW, RegisterWindowMessageW, SetForegroundWindow,
    TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenu, TranslateMessage, WINDOW_EX_STYLE, WM_APP, WM_CONTEXTMENU, WM_LBUTTONDBLCLK,
    WM_LBUTTONUP, WM_RBUTTONUP, WNDCLASSW, WS_OVERLAPPED,
};
use windows::core::{PCWSTR, w};

pub enum TepsiOlay {
    Goster,
    Cikis,
    /// Bir programın görev çubuğu düğmesi yanıp sönmeye başladı (yeni mesaj vb.); değer = pencere
    Yanip(isize),
}

const WM_TEPSI: u32 = WM_APP + 1;
static GERI: OnceLock<Box<dyn Fn(TepsiOlay) + Send + Sync>> = OnceLock::new();
static PENCERE: AtomicIsize = AtomicIsize::new(0);
static SIMGE: AtomicIsize = AtomicIsize::new(0);
static GOREV_CUBUGU: OnceLock<u32> = OnceLock::new();
static KABUK: OnceLock<u32> = OnceLock::new();
const HSHELL_FLASH: usize = 0x8006;

fn kopyala(hedef: &mut [u16], yazi: &str) {
    let v: Vec<u16> = yazi.encode_utf16().take(hedef.len() - 1).collect();
    hedef[..v.len()].copy_from_slice(&v);
    hedef[v.len()] = 0;
}

fn veri(bayrak: windows::Win32::UI::Shell::NOTIFY_ICON_DATA_FLAGS) -> NOTIFYICONDATAW {
    let mut n = NOTIFYICONDATAW::default();
    n.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    n.hWnd = HWND(PENCERE.load(Ordering::Relaxed) as *mut _);
    n.uID = 1;
    n.uFlags = bayrak;
    n
}

fn ekle() {
    let mut n = veri(NIF_ICON | NIF_MESSAGE | NIF_TIP);
    n.uCallbackMessage = WM_TEPSI;
    n.hIcon = HICON(SIMGE.load(Ordering::Relaxed) as *mut _);
    kopyala(&mut n.szTip, "Volt");
    unsafe {
        let _ = Shell_NotifyIconW(NIM_ADD, &n);
    }
}

/// Tepsi simgesinin üstünde kısa bir Windows bildirimi gösterir.
pub fn bildirim(baslik: &str, metin: &str) {
    if PENCERE.load(Ordering::Relaxed) == 0 {
        return;
    }
    let mut n = veri(NIF_INFO);
    kopyala(&mut n.szInfoTitle, baslik);
    kopyala(&mut n.szInfo, metin);
    n.dwInfoFlags = NIIF_INFO;
    unsafe {
        let _ = Shell_NotifyIconW(NIM_MODIFY, &n);
    }
}

/// Uygulama kapanırken simgeyi kaldır (yoksa fare üstüne gelene kadar tepside kalır).
pub fn kaldir() {
    if PENCERE.load(Ordering::Relaxed) != 0 {
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &veri(Default::default()));
        }
    }
}

fn olay(o: TepsiOlay) {
    if let Some(g) = GERI.get() {
        g(o);
    }
}

unsafe extern "system" fn pencere_islem(h: HWND, m: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if m == WM_TEPSI {
            match (l.0 & 0xFFFF) as u32 {
                WM_LBUTTONUP | WM_LBUTTONDBLCLK => olay(TepsiOlay::Goster),
                WM_RBUTTONUP | WM_CONTEXTMENU => {
                    let Ok(menu) = CreatePopupMenu() else { return LRESULT(0) };
                    let _ = AppendMenuW(menu, MF_STRING, 1, w!("Volt'u aç"));
                    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
                    let _ = AppendMenuW(menu, MF_STRING, 2, w!("Çıkış"));
                    let mut p = POINT::default();
                    let _ = GetCursorPos(&mut p);
                    let _ = SetForegroundWindow(h);
                    let secim = TrackPopupMenu(menu, TPM_RETURNCMD | TPM_RIGHTBUTTON, p.x, p.y, Some(0), h, None);
                    let _ = DestroyMenu(menu);
                    match secim.0 {
                        1 => olay(TepsiOlay::Goster),
                        2 => olay(TepsiOlay::Cikis),
                        _ => {}
                    }
                }
                _ => {}
            }
            return LRESULT(0);
        }
        // Görev çubuğunda yanıp sönen pencere (Discord vb. yeni mesajda böyle yapar)
        if Some(&m) == KABUK.get() {
            if w.0 == HSHELL_FLASH {
                olay(TepsiOlay::Yanip(l.0));
            }
            return LRESULT(0);
        }
        // Explorer yeniden başlarsa simge kaybolur; yeniden ekle
        if Some(&m) == GOREV_CUBUGU.get() {
            ekle();
        }
        DefWindowProcW(h, m, w, l)
    }
}

/// Tepsi simgesini oluşturur. `rgba`: n x n simge (RGBA, üstten alta).
pub fn baslat<F: Fn(TepsiOlay) + Send + Sync + 'static>(rgba: Vec<u8>, n: u32, geri: F) {
    let _ = GERI.set(Box::new(geri));
    std::thread::spawn(move || unsafe {
        let ornek = GetModuleHandleW(None).unwrap_or_default();
        let sinif = w!("VoltTepsi");
        let wc = WNDCLASSW { lpfnWndProc: Some(pencere_islem), hInstance: ornek.into(), lpszClassName: sinif, ..Default::default() };
        RegisterClassW(&wc);
        let Ok(h) = CreateWindowExW(WINDOW_EX_STYLE(0), sinif, w!("Volt"), WS_OVERLAPPED, 0, 0, 0, 0, None, None, Some(ornek.into()), None)
        else {
            return;
        };
        PENCERE.store(h.0 as isize, Ordering::Relaxed);
        let _ = GOREV_CUBUGU.set(RegisterWindowMessageW(w!("TaskbarCreated")));

        // RGBA -> BGRA (CreateIcon 32 bit renk + boş maske ister)
        let mut bgra = rgba;
        for p in bgra.chunks_mut(4) {
            p.swap(0, 2);
        }
        let maske = vec![0u8; (n * n / 8) as usize];
        if let Ok(s) = CreateIcon(Some(ornek.into()), n as i32, n as i32, 1, 32, maske.as_ptr(), bgra.as_ptr()) {
            SIMGE.store(s.0 as isize, Ordering::Relaxed);
        }
        ekle();
        let _ = KABUK.set(RegisterWindowMessageW(w!("SHELLHOOK")));
        let _ = RegisterShellHookWindow(h);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    });
}
