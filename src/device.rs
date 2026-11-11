// Copyright 2026 Eresbos (github.com/ERESB0S)
// SPDX-License-Identifier: GPL-3.0-or-later
//! Klavyeyi bulma ve paket gönderip cevap okuma.

use anyhow::{Context, Result, bail};
use hidapi::{HidApi, HidDevice};

use crate::protocol::{self, PACKET_LEN, Packet};

pub struct Keyboard {
    dev: HidDevice,
}

impl Keyboard {
    /// Klavyenin ışık komut kanalını (arayüz 1, usage page FF1C) açar.
    pub fn open() -> Result<Self> {
        let api = HidApi::new().context("HID kütüphanesi başlatılamadı")?;
        let info = api
            .device_list()
            .find(|d| {
                d.vendor_id() == protocol::VENDOR_ID
                    && d.product_id() == protocol::PRODUCT_ID
                    && d.interface_number() == protocol::INTERFACE
                    && d.usage_page() == protocol::USAGE_PAGE
            })
            .context("Pusat K3 Pro bulunamadı (kablolu bağlı mı?)")?;
        let dev = info.open_device(&api).context("Klavye açılamadı (resmi uygulama açık olabilir)")?;
        Ok(Self { dev })
    }

    /// Paketi gönderir ve klavyenin cevabını bekler.
    pub fn transact(&self, packet: &Packet, timeout_ms: i32) -> Result<Packet> {
        let written = self.dev.write(packet).context("Paket gönderilemedi")?;
        if written < PACKET_LEN {
            bail!("Paket eksik gönderildi: {written}/{PACKET_LEN} bayt");
        }
        let mut resp = [0u8; PACKET_LEN];
        let n = self.dev.read_timeout(&mut resp, timeout_ms).context("Cevap okunamadı")?;
        if n == 0 {
            bail!("Klavye {timeout_ms} ms içinde cevap vermedi");
        }
        Ok(resp)
    }

    /// Özel renk hafızasının tamamını yazar (Begin ... End içinde, resmi uygulama gibi).
    pub fn write_all_colors(&self, colors: &[u8]) -> Result<()> {
        if colors.len() != protocol::COLOR_BUFFER_LEN {
            bail!("Renk verisi {} bayt olmalı, {} geldi", protocol::COLOR_BUFFER_LEN, colors.len());
        }
        self.transact(&protocol::begin(), 1000)?;
        for (i, chunk) in colors.chunks(protocol::MAX_DATA).enumerate() {
            let req = protocol::write_custom_colors((i * protocol::MAX_DATA) as u16, chunk);
            self.transact(&req, 1000)?;
        }
        self.transact(&protocol::end(), 1000)?;
        Ok(())
    }

    /// Özel renkleri Begin/End OLMADAN gönderir (canlı mod denemesi).
    pub fn stream_colors(&self, colors: &[u8]) -> Result<()> {
        for (i, chunk) in colors.chunks(protocol::MAX_DATA).enumerate() {
            let req = protocol::write_custom_colors((i * protocol::MAX_DATA) as u16, chunk);
            self.transact(&req, 1000)?;
        }
        Ok(())
    }

    /// Tek bir renk paketini (index'inci 54 baytlık parça) Begin/End olmadan gönderir.
    pub fn stream_chunk(&self, index: usize, data: &[u8]) -> Result<()> {
        let req = protocol::write_custom_colors((index * protocol::MAX_DATA) as u16, data);
        self.transact(&req, 1000)?;
        Ok(())
    }

    /// Klavyedeki tüm özel renk hafızasını (432 bayt) okur. Hiçbir şey yazmaz.
    pub fn read_all_colors(&self) -> Result<Vec<u8>> {
        let mut out = Vec::with_capacity(protocol::COLOR_BUFFER_LEN);
        for offset in (0..protocol::COLOR_BUFFER_LEN).step_by(protocol::MAX_DATA) {
            let req = protocol::read_custom_colors(offset as u16, protocol::MAX_DATA as u8);
            let resp = self.transact(&req, 1000)?;
            out.extend_from_slice(protocol::response_data(&resp, protocol::MAX_DATA));
        }
        Ok(out)
    }
}
