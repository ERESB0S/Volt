// Copyright 2026 Eresbos (github.com/ERESB0S)
// SPDX-License-Identifier: GPL-3.0-or-later
//! K3 Pro'nun USB ışık protokolü: 64 baytlık paketlerin hazırlanması.
//! Ayrıntılar için PROTOKOL.md dosyasına bakın.

pub const VENDOR_ID: u16 = 0x0C45;
pub const PRODUCT_ID: u16 = 0x5004;
pub const INTERFACE: i32 = 1;
pub const USAGE_PAGE: u16 = 0xFF1C;

pub const PACKET_LEN: usize = 64;
pub const REPORT_ID: u8 = 0x04;
/// Bir pakette taşınabilecek en fazla veri baytı.
pub const MAX_DATA: usize = 0x36;
const DATA_START: usize = 8;

pub const LED_COUNT: usize = 126;
/// Her satırda 21 LED numarası var (0 = Esc, 21, 42, 63, 84, 105 satır başları).
pub const LEDS_PER_ROW: usize = 21;
/// Resmi uygulama özel renkleri 8 paket x 54 bayt = 432 bayt olarak yazıyor (144 LED'lik yer).
pub const COLOR_BUFFER_LEN: usize = 8 * MAX_DATA;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Command {
    Begin = 0x01,
    End = 0x02,
    ReadInfo = 0x03,
    WriteInfo = 0x04,
    ReadConfig = 0x05,
    SetParameter = 0x06,
    ReadCustomColors = 0x10,
    WriteCustomColors = 0x11,
}

pub type Packet = [u8; PACKET_LEN];

/// Bayt 3..63'ün toplamı, little-endian olarak bayt 1-2'ye yazılır.
fn apply_checksum(buf: &mut Packet) {
    let sum: u16 = buf[3..].iter().fold(0u16, |acc, &b| acc.wrapping_add(b as u16));
    buf[1] = (sum & 0xFF) as u8;
    buf[2] = (sum >> 8) as u8;
}

fn packet(cmd: Command, len: u8, param: u16, data: &[u8]) -> Packet {
    assert!(data.len() <= MAX_DATA);
    let mut buf = [0u8; PACKET_LEN];
    buf[0] = REPORT_ID;
    buf[3] = cmd as u8;
    buf[4] = len;
    buf[5..7].copy_from_slice(&param.to_le_bytes());
    buf[DATA_START..DATA_START + data.len()].copy_from_slice(data);
    apply_checksum(&mut buf);
    buf
}

/// Özel renk hafızasından `offset` konumundan başlayarak `len` bayt okuma isteği.
pub fn read_custom_colors(offset: u16, len: u8) -> Packet {
    packet(Command::ReadCustomColors, len, offset, &[])
}

/// Özel renk hafızasına `offset` konumundan başlayarak veri yazma.
pub fn write_custom_colors(offset: u16, data: &[u8]) -> Packet {
    packet(Command::WriteCustomColors, data.len() as u8, offset, data)
}

/// Her profilin ayar bloğu bu kadar bayt aralıklı (kayıttan: 0x00, 0x2A, 0x54).
pub const PROFILE_STRIDE: u16 = 0x2A;

pub const PARAM_MODE: u16 = 0x00;
pub const MODE_CUSTOM: u8 = 0x14;

/// Tek bir parametreyi yazar (bayt 5 = parametre no, veri bayt 8'den itibaren).
pub fn set_parameter(param: u16, data: &[u8]) -> Packet {
    packet(Command::SetParameter, data.len() as u8, param, data)
}

/// Ayar bloğundan `offset` konumundan başlayarak `len` bayt okur.
/// Profil 1 = 0x00, Profil 2 = 0x2A, Profil 3 = 0x54 (kayıttan).
pub fn read_config(offset: u16, len: u8) -> Packet {
    packet(Command::ReadConfig, len, offset, &[])
}

/// Klavye bilgi bloğunu okur (desteklenen modlar, aktif profil).
pub fn read_info() -> Packet {
    packet(Command::ReadInfo, 0x2C, 0, &[])
}

/// Bilgi bloğunun 10. baytı = aktif profil (0, 1, 2).
pub const INFO_ACTIVE_PROFILE: usize = 10;
pub const INFO_LEN: usize = 0x2C;

/// Bilgi bloğunu geri yazar. Resmi uygulama profil değiştirirken bunu yapıyor:
/// okuduğu bloğu sadece aktif profil baytını değiştirerek gönderiyor.
pub fn write_info(info: &[u8]) -> Packet {
    packet(Command::WriteInfo, info.len() as u8, 0, info)
}

/// Yapılandırma oturumunu başlatır.
pub fn begin() -> Packet {
    packet(Command::Begin, 0, 0, &[])
}

/// Yapılandırma oturumunu bitirir (kalıcı kayıt muhtemelen burada olur).
pub fn end() -> Packet {
    packet(Command::End, 0, 0, &[])
}

/// Cevap paketinin veri kısmı.
pub fn response_data(resp: &Packet, len: usize) -> &[u8] {
    &resp[DATA_START..DATA_START + len.min(MAX_DATA)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_matches_fixed_begin_packet() {
        // Begin paketinin bilinen hali: 04 01 00 01 00...
        let p = packet(Command::Begin, 0, 0, &[]);
        assert_eq!(&p[..4], &[0x04, 0x01, 0x00, 0x01]);
    }

    #[test]
    fn read_packet_layout() {
        let p = read_custom_colors(0x0036, 0x36);
        assert_eq!(p[3], 0x10);
        assert_eq!(p[4], 0x36);
        assert_eq!(&p[5..7], &[0x36, 0x00]);
        let sum = 0x10u16 + 0x36 + 0x36;
        assert_eq!(u16::from_le_bytes([p[1], p[2]]), sum);
    }
}
