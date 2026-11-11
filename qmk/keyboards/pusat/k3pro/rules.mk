# Copyright 2026 Eresbos (github.com/ERESB0S)
# SPDX-License-Identifier: GPL-2.0-or-later
# Pusat K3 Pro – SonixQMK
SRC = ../../../drivers/led/sn32/matrix_sn32f24xx.c
SRC += config_led.c

MCU = SN32F248BF

LTO_ENABLE = no
BACKLIGHT_ENABLE = no
MAGIC_ENABLE = yes
MAGIC_KEYCODE_ENABLE = yes
BOOTMAGIC_ENABLE = yes      # Esc basılı + kablo = güncelleme modu
MOUSEKEY_ENABLE = no
EXTRAKEY_ENABLE = yes       # Medya ve ses tuşları
CONSOLE_ENABLE = no
COMMAND_ENABLE = no
SLEEP_LED_ENABLE = no
NKRO_ENABLE = yes
AUDIO_ENABLE = no
RGBLIGHT_ENABLE = no
WAIT_FOR_USB = no
CUSTOM_MATRIX = yes
KEYBOARD_SHARED_EP = yes

ENCODER_ENABLE = yes

RGB_MATRIX_ENABLE = yes
RGB_MATRIX_DRIVER = SN32F24xB
