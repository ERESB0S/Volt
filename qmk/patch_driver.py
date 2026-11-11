# Copyright 2026 Eresbos (github.com/ERESB0S)
# SPDX-License-Identifier: GPL-2.0-or-later
"""SonixQMK rgb_matrix_sn32f24xb.c yaması: NO_LED konumlarında dizi dışına okumayı engeller.
Kullanım (WSL): python3 patch_driver.py ~/sonixqmk/drivers/led/sn32/rgb_matrix_sn32f24xb.c
"""
import sys

path = sys.argv[1]
src = open(path).read()
old = "        uint8_t led_index = g_led_config.matrix_co[row_idx][col_idx];\n"
new = old + (
    "        // K3 Pro yamasi: LED olmayan konumlar (NO_LED) icin dizi disina okuma yapma, kanali kapali tut\n"
    "        if (led_index >= DRIVER_LED_TOTAL) {\n"
    "            pwmEnableChannelI(pwmp, chan_col_order[col_idx], 0);\n"
    "            continue;\n"
    "        }\n"
)
if "K3 Pro yamasi" in src:
    print("zaten yamali")
else:
    assert src.count(old) == 1, "beklenen satir bulunamadi"
    open(path, "w").write(src.replace(old, new))
    print("yamandi")
