// Copyright 2026 Eresbos (github.com/ERESB0S)
// SPDX-License-Identifier: GPL-3.0-or-later
//! Volt kurulum programı: içindeki Volt.exe'yi %LOCALAPPDATA%\Programs\Volt'a kurar, kısayolları ve
//! Windows'un "Uygulamalar" listesindeki kaydı oluşturur. Yönetici izni istemez.
//!
//! Derleme sırası: önce `cargo build --release --bin pusat_isik`, sonra `cargo build --release --bin volt_kurulum`.
#![windows_subsystem = "windows"]

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;
use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::WindowBuilder;
use wry::{WebViewBuilder, WebViewBuilderExtWindows};

static VOLT: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/target/release/pusat_isik.exe"));
const PORT: u16 = 47321;

enum Olay {
    Betik(String),
    Kapat,
}

/// Çalışan Volt'a basit bir HTTP isteği
fn istek(yontem: &str, yol: &str) -> Option<String> {
    let mut s = TcpStream::connect_timeout(&([127, 0, 0, 1], PORT).into(), Duration::from_millis(500)).ok()?;
    s.set_read_timeout(Some(Duration::from_secs(3))).ok()?;
    s.write_all(format!("{yontem} {yol} HTTP/1.1\r\nHost: 127.0.0.1:{PORT}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").as_bytes()).ok()?;
    let mut c = String::new();
    let _ = s.read_to_string(&mut c);
    Some(c)
}

/// Açık bir Volt varsa kapat (klavye yazılımı yüklenirken asla)
fn volt_kapat() -> Result<(), String> {
    let Some(d) = istek("GET", "/api/guncelle") else { return Ok(()) };
    if ["yaziyor", "bekle", "baglaniyor", "hazirlik", "indiriyor"].iter().any(|a| d.contains(&format!("\"asama\":\"{a}\""))) {
        return Err("Volt şu an klavyeye yazılım yüklüyor. Bitince tekrar dene.".into());
    }
    let _ = istek("POST", "/api/cikis");
    for _ in 0..30 {
        std::thread::sleep(Duration::from_millis(200));
        if TcpListener::bind(("127.0.0.1", PORT)).is_ok() {
            std::thread::sleep(Duration::from_millis(300));
            return Ok(());
        }
    }
    Err("Açık olan Volt kapatılamadı. Tepsideki Volt simgesine sağ tıklayıp Çıkış'ı seç ve tekrar dene.".into())
}

fn js_metin(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o += "\\\"",
            '\\' => o += "\\\\",
            '\n' => o += "\\n",
            c if (c as u32) < 0x20 => o += &format!("\\u{:04x}", c as u32),
            c => o.push(c),
        }
    }
    o + "\""
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
            let r = 0.22;
            let (dx, dy) = ((x - 0.5).abs() - (0.5 - r), (y - 0.5).abs() - (0.5 - r));
            if (dx.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt() > r {
                continue;
            }
            let i = ((py * n + px) * 4) as usize;
            let c = if icinde(x, y) { [245, 245, 247, 255] } else { [24, 24, 27, 255] };
            v[i..i + 4].copy_from_slice(&c);
        }
    }
    v
}

fn main() {
    let dongu = EventLoopBuilder::<Olay>::with_user_event().build();
    let vekil = dongu.create_proxy();
    let simge = |n| tao::window::Icon::from_rgba(simge_rgba(n), n, n).ok();
    let pen = WindowBuilder::new()
        .with_title("Volt Kurulum")
        .with_inner_size(LogicalSize::new(520.0, 470.0))
        .with_resizable(false)
        .with_window_icon(simge(32))
        .build(&dongu)
        .expect("pencere");
    let ilk = format!(
        "window.KURULUM={{kurulu:{},yol:{},surum:{}}};",
        pusat_rgb::kurulum::kurulu(),
        js_metin(&pusat_rgb::kurulum::klasor().display().to_string()),
        js_metin(pusat_rgb::kurulum::SURUM)
    );
    let v2 = vekil.clone();
    // WebView2 verisi exe'nin yanına (ör. İndirilenler) değil, geçici klasöre
    let mut baglam = wry::WebContext::new(Some(std::env::temp_dir().join("VoltKurulum")));
    let gorunum = WebViewBuilder::new_with_web_context(&mut baglam)
        .with_html(include_str!("volt_kurulum.html"))
        .with_initialization_script(&ilk)
        .with_background_color((10, 10, 11, 255))
        .with_devtools(false)
        .with_default_context_menus(false)
        .with_ipc_handler(move |m| {
            let m = m.body().to_string();
            if let Some(s) = m.strip_prefix("kur:") {
                let (masaustu, oto) = (s.as_bytes().first() == Some(&b'1'), s.as_bytes().get(2) == Some(&b'1'));
                let v = v2.clone();
                std::thread::spawn(move || {
                    let yaz = |js: String| {
                        let _ = v.send_event(Olay::Betik(js));
                    };
                    let durum = |t: &str| yaz(format!("durum({})", js_metin(t)));
                    durum("Açık olan Volt kapatılıyor…");
                    if let Err(e) = volt_kapat() {
                        return yaz(format!("hata({})", js_metin(&e)));
                    }
                    match pusat_rgb::kurulum::kur(VOLT, masaustu, oto, durum) {
                        Ok(_) => yaz("bitti()".into()),
                        Err(e) => yaz(format!("hata({})", js_metin(&format!("{e:#}")))),
                    }
                });
            } else if m == "ac" {
                let _ = std::process::Command::new(pusat_rgb::kurulum::exe_yolu()).spawn();
                let _ = v2.send_event(Olay::Kapat);
            } else if m == "kapat" {
                let _ = v2.send_event(Olay::Kapat);
            }
        })
        .build(&pen)
        .expect("WebView2 başlatılamadı");

    dongu.run(move |olay, _, akis| {
        *akis = ControlFlow::Wait;
        match olay {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } | Event::UserEvent(Olay::Kapat) => *akis = ControlFlow::Exit,
            Event::UserEvent(Olay::Betik(js)) => {
                let _ = gorunum.evaluate_script(&js);
            }
            _ => {}
        }
    });
}
