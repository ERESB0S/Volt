// Copyright 2026 Eresbos (github.com/ERESB0S)
// SPDX-License-Identifier: GPL-2.0-or-later
/* Pusat K3 Pro (Monster) – SonixQMK portu.
 * Pin bilgileri orijinal firmware'in çözümlenmesinden çıkarıldı: DONANIM_HARITASI.md
 * Şablon: keyboards/redragon/k580 (aynı eVision/SN32F248B tasarımı).
 */
#pragma once

#include "config_common.h"

/* USB – resmi uygulama karışmasın diye stok PID'den (5004) farklı */
#define VENDOR_ID       0x0C45
#define PRODUCT_ID      0x5104
#define DEVICE_VER      0x0001

#define MANUFACTURER    Monster
#define PRODUCT         Pusat K3 Pro - Volt
#define DESCRIPTION     Monster Pusat K3 Pro QMK

/* Tuş matrisi: 7 satır x 20 sütun, sütunlar LOW sürülür, satırlar okunur */
#define MATRIX_ROWS 7
#define MATRIX_COLS 20

#define DIODE_DIRECTION ROW2COL

#define MATRIX_COL_PINS { A0, A1, A2, A3, A4, A5, A6, A7, A8, A9, A10, A11, A12, A13, A14, B0, B1, B2, B3, B4 }
#define MATRIX_ROW_PINS { C15, D11, D10, D9, D8, D7, B5 }

/* Ses tekerleği (K580 ile aynı pinler ve yön ayarı) */
#define ENCODERS_PAD_A { D4 }
#define ENCODERS_PAD_B { D3 }
#define ENCODER_DIRECTION_FLIP
#define ENCODER_RESOLUTION 2

#define TAP_CODE_DELAY 5

#undef FORCE_NKRO
#define FORCE_NKRO
#define USB_POLLING_INTERVAL_MS 1

#define DEBOUNCE 5

/* Kilit göstergeleri (stok firmware: B13 Num, B14 Caps, B15 Scroll, B12 Win kilidi; LOW = yanık) */
#define LED_NUM_LOCK_PIN    B13
#define LED_CAPS_LOCK_PIN   B14
#define LED_SCROLL_LOCK_PIN B15
#define LED_PIN_ON_STATE    0

/* Esc basılıyken kablo takılırsa güncelleme moduna (ROM bootloader) girer */
#define BOOTMAGIC_LITE_ROW    0
#define BOOTMAGIC_LITE_COLUMN 0

#include "config_led.h"
