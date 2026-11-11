// Copyright 2026 Eresbos (github.com/ERESB0S)
// SPDX-License-Identifier: GPL-3.0-or-later
//! Sadece okuma: 0C45 üreticili HID aygıtlarını listeler (hiçbir şey göndermez).
fn main() -> anyhow::Result<()> {
    let api = hidapi::HidApi::new()?;
    for d in api.device_list().filter(|d| d.vendor_id() == 0x0C45) {
        println!(
            "{:04X}:{:04X} if={} up={:04X} u={:04X} rel={:04X} urun={:?} uretici={:?}",
            d.vendor_id(), d.product_id(), d.interface_number(), d.usage_page(), d.usage(),
            d.release_number(), d.product_string(), d.manufacturer_string()
        );
    }
    Ok(())
}
