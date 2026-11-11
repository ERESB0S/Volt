// Copyright 2026 Eresbos (github.com/ERESB0S)
// SPDX-License-Identifier: GPL-3.0-or-later
//! Arayüz yazı tipi Poppins (SIL Open Font License 1.1, bkz. yazi/OFL.txt), exe'ye gömülü.

const DOSYALAR: [(&str, u16, &[u8]); 4] = [
    ("Poppins-Regular.ttf", 400, include_bytes!("../yazi/Poppins-Regular.ttf")),
    ("Poppins-Medium.ttf", 500, include_bytes!("../yazi/Poppins-Medium.ttf")),
    ("Poppins-SemiBold.ttf", 600, include_bytes!("../yazi/Poppins-SemiBold.ttf")),
    ("Poppins-Bold.ttf", 700, include_bytes!("../yazi/Poppins-Bold.ttf")),
];

/// Ada göre yazı tipi dosyası (Volt'un yerel sunucusu /yazi/<ad> olarak verir)
pub fn dosya(ad: &str) -> Option<&'static [u8]> {
    DOSYALAR.iter().find(|d| d.0 == ad).map(|d| d.2)
}

/// Sunucusu olmayan pencereler (kurulum) için base64 gömülü @font-face kuralları
pub fn font_face_gomulu() -> String {
    DOSYALAR
        .iter()
        .filter(|d| d.1 == 400 || d.1 == 500 || d.1 == 600)
        .map(|d| format!("@font-face {{ font-family: \"Poppins\"; font-weight: {}; font-display: block; src: url(data:font/ttf;base64,{}) format(\"truetype\"); }}\n", d.1, base64(d.2)))
        .collect()
}

fn base64(v: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut o = String::with_capacity(v.len().div_ceil(3) * 4);
    for p in v.chunks(3) {
        let n = (p[0] as u32) << 16 | (*p.get(1).unwrap_or(&0) as u32) << 8 | *p.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            o.push(if i <= p.len() { T[(n >> (18 - 6 * i) & 63) as usize] as char } else { '=' });
        }
    }
    o
}
