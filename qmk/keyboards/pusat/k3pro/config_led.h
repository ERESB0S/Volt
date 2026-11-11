// Copyright 2026 Eresbos (github.com/ERESB0S)
// SPDX-License-Identifier: GPL-2.0-or-later
#pragma once

#define LED_MATRIX_ROWS MATRIX_ROWS
#define LED_MATRIX_ROW_CHANNELS 3
#define LED_MATRIX_ROWS_HW (LED_MATRIX_ROWS * LED_MATRIX_ROW_CHANNELS)

/* Sürücü her satır için sırasıyla { Mavi, Yeşil, Kırmızı } pinini bekliyor
 * (rgb_matrix_sn32f24xb.c: kanal 0 -> .b, 1 -> .g, 2 -> .r).
 * Stok firmware'den: satır0 R=C0 G=C1 B=C3, satır1 R=C4 G=C5 B=C6, satır2 R=C7 G=C8 B=C9,
 * satır3 R=C10 G=C11 B=C12, satır4 R=B6 G=B7 B=B8, satır5 R=B9 G=B10 B=B11, satır6 R=C13 G=A15 B=C14
 * DÜZELTME (v3): Klavyede denendi, kırmızı yeşil yanıyordu: stok tablodaki R ile G yer değiştirmiş.
 * Gerçekte: satır0 R=C1 G=C0, ... (her satırda R ve G pinleri takas edildi). */
#define LED_MATRIX_ROW_PINS { C3, C0, C1, \
                              C6, C4, C5, \
                              C9, C7, C8, \
                              C12, C10, C11, \
                              B8, B6, B7, \
                              B11, B9, B10, \
                              C14, C13, A15 }

#define LED_MATRIX_COLS MATRIX_COLS
#define LED_MATRIX_COL_PINS MATRIX_COL_PINS

#define DRIVER_LED_TOTAL (140)
