// Copyright 2026 Eresbos (github.com/ERESB0S)
// SPDX-License-Identifier: GPL-2.0-or-later
#pragma once

// Tuşa basma reaksiyonu için son basılan tuşların takibi
// Tuş vuruşlarını keymap.c kendisi takip ediyor (basınca / bırakınca ayarı için)
#define LED_HITS_TO_REMEMBER 4 // rgb_matrix_map_row_column_to_led tampon boyutu

// Tek efekt: SIMSEK (ayarları bilgisayardaki uygulama belirler)
#define RGB_MATRIX_STARTUP_MODE RGB_MATRIX_CUSTOM_SIMSEK
#define RGB_MATRIX_STARTUP_HUE 0
#define RGB_MATRIX_STARTUP_SAT 0
#define RGB_MATRIX_STARTUP_VAL 255
#define RGB_MATRIX_STARTUP_SPD 128
// Işık hesabını küçük parçalara böl: tuş taraması arasında daha az bekleme olsun
#define RGB_MATRIX_LED_PROCESS_LIMIT 12
