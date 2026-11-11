# Copyright 2026 Eresbos (github.com/ERESB0S)
# SPDX-License-Identifier: GPL-2.0-or-later
RGB_MATRIX_CUSTOM_USER = yes
# Bilgisayardaki ayar uygulamasıyla haberleşme kanalı
RAW_ENABLE = yes
# QMK'nın flash-EEPROM'u bu çipte silmeden yazıyor; kullanma. Ayarlar keymap.c'de kendi sayfamızda.
EEPROM_DRIVER = transient

# Gecikmesiz basış: basış anında gönderilir, sadece bırakış 5 ms bekletilir (varsayılan sym_defer_g her basışı 5 ms geciktirir)
DEBOUNCE_TYPE = asym_eager_defer_pk
