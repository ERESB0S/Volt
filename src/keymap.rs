// Copyright 2026 Eresbos (github.com/ERESB0S)
// SPDX-License-Identifier: GPL-3.0-or-later
//! Tuş → LED haritası (Türkçe Q, tam boy K3 Pro).
//!
//! Kalibrasyonla doğrulanan noktalar (2026-11-09):
//!   LED 7 = F7, 22 = 2, 28 = 8, 31 = * (?), 43 = E, 49 = O, 52 = Ü,
//!   64 = F, 70 = Ş, 73 = Enter, 86 = B, 91 = ".", 95 = Yukarı ok.
//! Satır başları: Esc = 0, " = 20, Tab = 40, Caps = 60, Sol Shift = 80.
//! "tahmin" yazan satırlar henüz doğrulanmadı.
//!
//! Her kayıt: (Windows sanal tuş kodu, LED, fiziksel x, fiziksel y) — birim: bir tuş genişliği.

pub struct Key {
    pub vk: i32,
    pub led: usize,
    pub x: f32,
    pub y: f32,
}

const fn k(vk: i32, led: usize, x: f32, y: f32) -> Key {
    Key { vk, led, x, y }
}

// Windows sanal tuş kodları
const VK_ESC: i32 = 0x1B;
const VK_TAB: i32 = 0x09;
const VK_CAPS: i32 = 0x14;
const VK_LSHIFT: i32 = 0xA0;
const VK_RSHIFT: i32 = 0xA1;
const VK_BACK: i32 = 0x08;
const VK_RETURN: i32 = 0x0D;
const VK_UP: i32 = 0x26;
const VK_OEM_3: i32 = 0xC0; // "
const VK_OEM_8: i32 = 0xDF; // *
const VK_OEM_MINUS: i32 = 0xBD; // -
const VK_OEM_4: i32 = 0xDB; // Ğ
const VK_OEM_6: i32 = 0xDD; // Ü
const VK_OEM_1: i32 = 0xBA; // Ş
const VK_OEM_7: i32 = 0xDE; // İ
const VK_OEM_COMMA: i32 = 0xBC; // ,
const VK_OEM_102: i32 = 0xE2; // <
const VK_OEM_2: i32 = 0xBF; // Ö
const VK_OEM_5: i32 = 0xDC; // Ç
const VK_OEM_PERIOD: i32 = 0xBE; // .

const fn f(n: i32) -> i32 {
    0x70 + n - 1 // VK_F1 = 0x70
}

const fn c(ch: u8) -> i32 {
    ch as i32
}

pub const KEYS: &[Key] = &[
    // Satır 0: Esc, F1–F12
    k(VK_ESC, 0, 0.0, 0.0),
    k(f(1), 1, 2.0, 0.0),
    k(f(2), 2, 3.0, 0.0),
    k(f(3), 3, 4.0, 0.0),
    k(f(4), 4, 5.0, 0.0),
    k(f(5), 5, 6.5, 0.0),
    k(f(6), 6, 7.5, 0.0),
    k(f(7), 7, 8.5, 0.0),
    k(f(8), 8, 9.5, 0.0),
    k(f(9), 9, 11.0, 0.0),
    k(f(10), 10, 12.0, 0.0),
    k(f(11), 11, 13.0, 0.0),
    k(f(12), 12, 14.0, 0.0),
    // Satır 1: " 1 2 … 0 * - Backspace
    k(VK_OEM_3, 20, 0.0, 1.25),
    k(c(b'1'), 21, 1.0, 1.25),
    k(c(b'2'), 22, 2.0, 1.25),
    k(c(b'3'), 23, 3.0, 1.25),
    k(c(b'4'), 24, 4.0, 1.25),
    k(c(b'5'), 25, 5.0, 1.25),
    k(c(b'6'), 26, 6.0, 1.25),
    k(c(b'7'), 27, 7.0, 1.25),
    k(c(b'8'), 28, 8.0, 1.25),
    k(c(b'9'), 29, 9.0, 1.25),
    k(c(b'0'), 30, 10.0, 1.25),
    k(VK_OEM_8, 31, 11.0, 1.25),
    k(VK_OEM_MINUS, 32, 12.0, 1.25),
    k(VK_BACK, 33, 13.5, 1.25), // tahmin
    // Satır 2: Tab Q W E R T Y U I O P Ğ Ü
    k(VK_TAB, 40, 0.25, 2.25),
    k(c(b'Q'), 41, 1.5, 2.25),
    k(c(b'W'), 42, 2.5, 2.25),
    k(c(b'E'), 43, 3.5, 2.25),
    k(c(b'R'), 44, 4.5, 2.25),
    k(c(b'T'), 45, 5.5, 2.25),
    k(c(b'Y'), 46, 6.5, 2.25),
    k(c(b'U'), 47, 7.5, 2.25),
    k(c(b'I'), 48, 8.5, 2.25),
    k(c(b'O'), 49, 9.5, 2.25),
    k(c(b'P'), 50, 10.5, 2.25),
    k(VK_OEM_4, 51, 11.5, 2.25),
    k(VK_OEM_6, 52, 12.5, 2.25),
    // Satır 3: Caps A S D F G H J K L Ş İ , Enter
    k(VK_CAPS, 60, 0.4, 3.25),
    k(c(b'A'), 61, 1.75, 3.25),
    k(c(b'S'), 62, 2.75, 3.25),
    k(c(b'D'), 63, 3.75, 3.25),
    k(c(b'F'), 64, 4.75, 3.25),
    k(c(b'G'), 65, 5.75, 3.25),
    k(c(b'H'), 66, 6.75, 3.25),
    k(c(b'J'), 67, 7.75, 3.25),
    k(c(b'K'), 68, 8.75, 3.25),
    k(c(b'L'), 69, 9.75, 3.25),
    k(VK_OEM_1, 70, 10.75, 3.25),
    k(VK_OEM_7, 71, 11.75, 3.25),
    k(VK_OEM_COMMA, 72, 12.75, 3.25),
    k(VK_RETURN, 73, 13.6, 2.75),
    // Satır 4: Sol Shift < Z X C V B N M Ö Ç . Sağ Shift, Yukarı ok
    k(VK_LSHIFT, 80, 0.1, 4.25),
    k(VK_OEM_102, 81, 1.25, 4.25),
    k(c(b'Z'), 82, 2.25, 4.25),
    k(c(b'X'), 83, 3.25, 4.25),
    k(c(b'C'), 84, 4.25, 4.25),
    k(c(b'V'), 85, 5.25, 4.25),
    k(c(b'B'), 86, 6.25, 4.25),
    k(c(b'N'), 87, 7.25, 4.25),
    k(c(b'M'), 88, 8.25, 4.25),
    k(VK_OEM_2, 89, 9.25, 4.25),
    k(VK_OEM_5, 90, 10.25, 4.25),
    k(VK_OEM_PERIOD, 91, 11.25, 4.25),
    k(VK_RSHIFT, 92, 13.0, 4.25), // tahmin
    k(VK_UP, 95, 16.25, 4.25),
];
