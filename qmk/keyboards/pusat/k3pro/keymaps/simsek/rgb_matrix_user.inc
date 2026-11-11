// Copyright 2026 Eresbos (github.com/ERESB0S)
// SPDX-License-Identifier: GPL-2.0-or-later
// Işık efekti: tüm ayarlar isik_ayar (ayarlar.h) içinden gelir; bilgisayardaki uygulama değiştirir.
//   MOD_SIMSEK : kısık taban; basılan tuş ve çevresi bir anda parlar, sonra söner
//   MOD_DALGA  : basılan tuştan kalın bir dalga tüm klavyeyi tarar
//   MOD_SABIT  : sadece taban renk
//   MOD_KAPALI : kapalı

RGB_MATRIX_EFFECT(SIMSEK)

#ifdef RGB_MATRIX_CUSTOM_EFFECT_IMPLS

#include "ayarlar.h"

// Her karede bir kez hesaplanan "vuruştan bu yana geçen süre" (ms); 0xFFFF = boş
static uint16_t vurus_tick[VURUS_SAYISI];

// Dalga: basılan tuştan kalın bir parlak cephe tüm klavyeyi (0..DALGA_MESAFE birim) geçer.
// Cephenin önü yumuşak, ortası tam parlak; arkasında giderek sönen bir iz kalır.
#define DALGA_MESAFE 240 // klavyenin bir ucundan öbürüne (224) biraz fazlası

static uint8_t dalga_isik(uint8_t i) {
    uint16_t sure  = isik_ayar.sure;
    uint16_t kal   = isik_ayar.kalinlik;
    uint16_t yarim = kal / 2;
    uint16_t iz    = (uint16_t)isik_ayar.iz * 120 / 100; // iz uzunluğu (birim)
    // Dalga cephesi en uzaktaki tuşu da geçip izi bitene kadar sürer
    uint32_t bitis = (uint32_t)sure * (DALGA_MESAFE + yarim + iz) / DALGA_MESAFE;
    uint8_t  en    = 0;
    for (uint8_t j = 0; j < VURUS_SAYISI; j++) {
        uint16_t tick = vurus_tick[j];
        if (tick >= bitis) continue;
        int16_t  dx   = (int16_t)g_led_config.point[i].x - vuruslar.x[j];
        int16_t  dy   = (int16_t)g_led_config.point[i].y - vuruslar.y[j];
        int16_t  dist = sqrt16((uint16_t)(dx * dx + dy * dy));
        int16_t  on   = (uint32_t)tick * DALGA_MESAFE / sure; // cephenin merkezi
        int16_t  d    = dist - on;                            // + : cephenin önünde, - : arkasında
        uint16_t v;
        if (d >= (int16_t)yarim) continue;                    // dalga daha gelmedi
        if (d >= 0) {
            v = 255 - (uint32_t)d * 255 / (yarim ? yarim : 1); // ön kenar: yumuşak
        } else if (-d <= (int16_t)yarim) {
            v = 255;                                           // arka yarı: tam parlak
        } else {
            uint16_t geri = -d - yarim;                        // izin içinde ne kadar geride
            if (geri >= iz) continue;
            v = (uint32_t)(iz - geri) * 160 / iz;              // iz: daha sönük, giderek söner
        }
        v = v * v / 255; // daha "parlak çekirdek, yumuşak kenar"
        // Dalga ilerledikçe biraz zayıflar (uçlarda %60'a iner)
        uint16_t zayif = 255 - (uint32_t)(on > DALGA_MESAFE ? DALGA_MESAFE : on) * 102 / DALGA_MESAFE;
        v = v * zayif / 255;
        if (isik_ayar.titreme && tick < 60 && ((tick / 10 + j) & 1)) v = v * 3 / 4;
        if (v > en) en = v;
    }
    return en;
}

static uint8_t simsek_isik(uint8_t i) {
    if (isik_ayar.mod == MOD_DALGA) return dalga_isik(i);
    uint16_t sure    = isik_ayar.sure;
    uint8_t  yaricap = isik_ayar.yaricap;
    uint8_t  en      = 0;
    for (uint8_t j = 0; j < VURUS_SAYISI; j++) {
        uint16_t tick = vurus_tick[j];
        if (tick >= sure) continue;
        int16_t  dx   = (int16_t)g_led_config.point[i].x - vuruslar.x[j];
        int16_t  dy   = (int16_t)g_led_config.point[i].y - vuruslar.y[j];
        uint16_t dist = sqrt16((uint16_t)(dx * dx + dy * dy));
        uint16_t uzay;
        {
            if (dist >= yaricap) continue;
            // Mesafe: merkez 255, kenar 0 (karesel düşüş = yumuşak kenar)
            uzay = 255 - dist * 255 / yaricap;
            uzay = uzay * uzay / 255;
        }
        // Zaman: ani parlama, hızlı sönme (karesel)
        uint16_t zaman = 255 - (uint32_t)tick * 255 / sure;
        zaman          = zaman * zaman / 255;
        // İlk 40 ms'de hafif titreme (şimşek hissi)
        if (isik_ayar.titreme && tick < 40 && ((tick / 8 + j) & 1)) zaman = zaman * 3 / 4;
        uint16_t v = uzay * zaman / 255;
        if (v > en) en = v;
    }
    return en;
}

static inline uint8_t olcekle(uint8_t c, uint16_t yuzde) { return (uint16_t)c * yuzde / 100; }
/* Gamma düzeltmesi (2.2): ekranda seçilen renk LED'de de aynı tonda görünsün.
 * Ekran renkleri gamma eğrisiyle gösterir, LED ise doğrusal; düzeltme olmadan açık renkler (ör. #FFC8B4) LED'de beyaz görünür.
 * Sadece seçilen rengin tonuna uygulanır; parlaklık ayarları eskisi gibi doğrusal kalır. */
static const uint8_t GAMA[256] = {
    0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
    1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2,
    3, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 6, 6, 6,
    6, 7, 7, 7, 8, 8, 8, 9, 9, 9, 10, 10, 11, 11, 11, 12,
    12, 13, 13, 13, 14, 14, 15, 15, 16, 16, 17, 17, 18, 18, 19, 19,
    20, 20, 21, 22, 22, 23, 23, 24, 25, 25, 26, 26, 27, 28, 28, 29,
    30, 30, 31, 32, 33, 33, 34, 35, 35, 36, 37, 38, 39, 39, 40, 41,
    42, 43, 43, 44, 45, 46, 47, 48, 49, 49, 50, 51, 52, 53, 54, 55,
    56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71,
    73, 74, 75, 76, 77, 78, 79, 81, 82, 83, 84, 85, 87, 88, 89, 90,
    91, 93, 94, 95, 97, 98, 99, 100, 102, 103, 105, 106, 107, 109, 110, 111,
    113, 114, 116, 117, 119, 120, 121, 123, 124, 126, 127, 129, 130, 132, 133, 135,
    137, 138, 140, 141, 143, 145, 146, 148, 149, 151, 153, 154, 156, 158, 159, 161,
    163, 165, 166, 168, 170, 172, 173, 175, 177, 179, 181, 182, 184, 186, 188, 190,
    192, 194, 196, 197, 199, 201, 203, 205, 207, 209, 211, 213, 215, 217, 219, 221,
    223, 225, 227, 229, 231, 234, 236, 238, 240, 242, 244, 246, 248, 251, 253, 255,
};
static inline uint8_t olcekle_g(uint8_t c, uint16_t yuzde) { return olcekle(GAMA[c], yuzde); }

/* ---------------- Orijinal klavye animasyonlarının karşılıkları ---------------- */

static uint8_t  c_tr, c_tg, c_tb, c_pr, c_pg, c_pb, c_genel; // karede bir kez hesaplanan renkler
static uint32_t c_t;                                         // hız ayarıyla ölçeklenmiş zaman
static uint8_t  yildiz_v[DRIVER_LED_TOTAL], yildiz_h[DRIVER_LED_TOTAL], birik_v[DRIVER_LED_TOTAL];
static uint32_t lcg = 12345;

static uint8_t rastgele(void) {
    lcg = lcg * 1103515245u + 12345u;
    return lcg >> 24;
}

static RGB renk_hsv(uint8_t h, uint8_t v) {
    HSV x = {h, 255, (uint8_t)((uint16_t)v * c_genel / 100)};
    return hsv_to_rgb(x);
}

// Taban renkten vurgu rengine (parlama rengi; "renkli" açıksa verilen ton) geçiş, a = 0..255
static RGB vurgu(uint8_t a, uint8_t ton) {
    RGB p;
    if (isik_ayar.renkli) {
        p = renk_hsv(ton, 255);
    } else {
        p.r = c_pr; p.g = c_pg; p.b = c_pb;
    }
    // Alan adlarıyla doldur: QMK'da RGB yapısı bellekte G, R, B sırasında (sıralı {r, g, b} yazmak kırmızı ile yeşili değiştirir)
    RGB o = {.r = (uint8_t)(c_tr + (int16_t)(p.r - c_tr) * a / 255), .g = (uint8_t)(c_tg + (int16_t)(p.g - c_tg) * a / 255),
             .b = (uint8_t)(c_tb + (int16_t)(p.b - c_tb) * a / 255)};
    return o;
}

// 0 -> 255 -> 0 üçgen dalga
static uint8_t ucgen(uint32_t t, uint16_t periyot) {
    uint16_t x = t % periyot;
    uint32_t y = x < periyot / 2 ? (uint32_t)x * 510 / periyot : (uint32_t)(periyot - x) * 510 / periyot;
    return y > 255 ? 255 : y;
}

static inline int16_t mutlak(int16_t v) { return v < 0 ? -v : v; }

// Tepkili modlar: basılan tuşa göre bu LED'in parlaklığı (ve renkli modda tonu)
static uint8_t tepki_isik(uint8_t i, uint8_t mod, uint8_t *ton) {
    int16_t  x = g_led_config.point[i].x, y = g_led_config.point[i].y;
    uint16_t sure = isik_ayar.sure;
    uint8_t  en   = 0;
    for (uint8_t j = 0; j < VURUS_SAYISI; j++) {
        uint16_t tick = vurus_tick[j];
        if (tick >= sure) continue;
        int16_t  ddx  = x - vuruslar.x[j], ddy = y - vuruslar.y[j];
        uint16_t sol  = 255 - (uint32_t)tick * 255 / sure; // zamanla sönme
        uint16_t v    = 0;
        uint8_t  d    = sqrt16((uint16_t)(ddx * ddx + ddy * ddy));
        if (mod == MOD_TEPKILI) {
            if (mutlak(ddx) < 6 && mutlak(ddy) < 6) v = sol;
        } else if (mod == MOD_TEPKI_CIZGI) {
            if (mutlak(ddx) < 5 || mutlak(ddy) < 5) v = sol * (255 - (d > 200 ? 200 : d)) / 255;
        } else { // MOD_DALGALANMA: ince, renkli halka
            uint16_t on   = (uint32_t)tick * 240 / sure;
            uint16_t fark = d > on ? d - on : on - d;
            if (fark < 8) v = (255 - fark * 32) * sol / 255;
        }
        if (v > en) {
            en   = v;
            *ton = d * 3 + j * 40;
        }
    }
    return en;
}

// Birikme: yeni vuruşları tuşlara ekle (karede bir kez)
static void birikme_isle(void) {
    for (uint8_t j = 0; j < VURUS_SAYISI; j++) {
        if (vurus_tick[j] == 0xFFFF || vuruslar.islendi[j]) continue;
        vuruslar.islendi[j] = 1;
        for (uint8_t i = 0; i < DRIVER_LED_TOTAL; i++) {
            int16_t ddx = g_led_config.point[i].x - vuruslar.x[j], ddy = g_led_config.point[i].y - vuruslar.y[j];
            if (mutlak(ddx) < 6 && mutlak(ddy) < 6) birik_v[i] = birik_v[i] > 135 ? 255 : birik_v[i] + 120;
        }
    }
}

// Ses verisinin geçerlilik çarpanı: paket gelmezse 0.3 sn sonra yarım saniyede söner (Volt kapandıysa)
static inline uint8_t karis(uint8_t x, uint8_t y, uint8_t a) { return x + (int16_t)(y - x) * a / 255; }
static uint8_t ses_carpan = 0;
static inline uint8_t ses_al(uint8_t v) { return (uint16_t)v * ses_carpan / 255; }

static RGB ses_isik(uint8_t i, int16_t x, int16_t y, int16_t dx, int16_t dy) {
    switch (isik_ayar.titreme) {
        case SES_SPEKTRUM: {
            // Her sütun bir frekans bandı (solda bas); seviye arttıkça aşağıdan yukarı yanar
            uint8_t  b  = (uint16_t)x * SES_BANT / 225;
            int16_t  h  = (int16_t)ses_al(ses_bant[b]) * 76 / 255 - (64 - y); // tepenin bu tuşun ne kadar üstünde olduğu
            uint8_t  a  = h <= 0 ? 0 : h >= 12 ? 255 : h * 255 / 12;
            return vurgu(a, b * 15 + (64 - y));
        }
        case SES_NABIZ: {
            // Bas vurdukça tüm klavye parlar
            uint8_t a = ses_al(ses_bas);
            return vurgu((uint16_t)a * a / 255, (uint8_t)(c_t / 8) + x);
        }
        case SES_DALGA: {
            // Ortadan dışa: merkez bas, dışarısı tiz
            uint16_t d = sqrt16((uint16_t)(dx * dx + dy * dy * 4));
            uint8_t  b = d * SES_BANT / 130;
            if (b >= SES_BANT) b = SES_BANT - 1;
            return vurgu(ses_al(ses_bant[b]), b * 15);
        }
        default: { // SES_RENK: renk baskın frekansa göre değişir, parlaklık ses seviyesine göre
            uint8_t v = ses_al(ses_seviye);
            RGB     c = renk_hsv(ses_ton + x / 6, 255);
            RGB     o = {.r = (uint8_t)(c_tr + (int16_t)(c.r - c_tr) * v / 255), .g = (uint8_t)(c_tg + (int16_t)(c.g - c_tg) * v / 255),
                         .b = (uint8_t)(c_tb + (int16_t)(c.b - c_tb) * v / 255)};
            return o;
        }
    }
}

static uint8_t hiz_gosterge = 0; // yazma hızı, yumuşatılmış 0..255

static RGB karisim(uint8_t a) { // taban -> parlama
    RGB o = {.r = (uint8_t)(c_tr + (int16_t)(c_pr - c_tr) * a / 255), .g = (uint8_t)(c_tg + (int16_t)(c_pg - c_tg) * a / 255),
             .b = (uint8_t)(c_tb + (int16_t)(c_pb - c_tb) * a / 255)};
    return o;
}
static RGB ekran_isik(int16_t x) {
    if (timer_elapsed(ekran_zaman) > 1500) return karisim(0); // Volt göndermiyorsa taban renk
    uint16_t p = (uint32_t)x * (EKRAN_BOLGE - 1) * 256 / 224;
    uint8_t  z = p >> 8, f = p & 255;
    if (z >= EKRAN_BOLGE - 1) { z = EKRAN_BOLGE - 2; f = 255; }
    const uint8_t *a = &ekran_renk[z * 3], *b = a + 3;
    RGB o = {.r = olcekle_g(karis(a[0], b[0], f), c_genel), .g = olcekle_g(karis(a[1], b[1], f), c_genel), .b = olcekle_g(karis(a[2], b[2], f), c_genel)};
    return o;
}
static RGB boya_isik(uint8_t i) {
    uint32_t adr = BOYA_ADRES(profil);
    if (*(const uint32_t *)(adr + 508) != BOYA_IMZA) return karisim(0);
    RGB o = {0, 0, 0};
    if (i >= BOYA_LED) return o;
    const uint8_t *c = (const uint8_t *)adr + i * 3;
    o.r = olcekle_g(c[0], c_genel); o.g = olcekle_g(c[1], c_genel); o.b = olcekle_g(c[2], c_genel);
    return o;
}

static RGB ekstra_mod(uint8_t i, uint8_t mod) {
    int16_t x = g_led_config.point[i].x, y = g_led_config.point[i].y;
    int16_t dx = x - 112, dy = y - 32;
    uint8_t faz = c_t / 8;
    uint8_t ton = 0, v;
    switch (mod) {
        case MOD_NEFES: {
            uint16_t per = 6000 - (uint16_t)isik_ayar.hiz * 20;
            uint32_t t   = timer_read32();
            v            = ucgen(t, per);
            v            = (uint16_t)v * v / 255;
            return vurgu(v, (uint8_t)(t / per * 40));
        }
        case MOD_SPEKTRUM: return renk_hsv(faz, 255);
        case MOD_RENK_CARKI: return renk_hsv(atan2_8(dy, dx) + faz, 255);
        case MOD_GOKKUSAGI: return renk_hsv(x - faz, 255);
        case MOD_DIKEY: return renk_hsv(y * 3 - faz, 255);
        case MOD_DAIRESEL: return renk_hsv(sqrt16((uint16_t)(dx * dx + dy * dy)) * 2 - faz, 255);
        case MOD_KASIRGA: return renk_hsv(atan2_8(dy, dx) + sqrt16((uint16_t)(dx * dx + dy * dy)) * 2 - faz * 2, 255);
        case MOD_TEPKILI:
        case MOD_TEPKI_CIZGI:
        case MOD_DALGALANMA:
            v = tepki_isik(i, mod, &ton);
            return vurgu(v, ton);
        case MOD_YILDIZ: {
            uint8_t sonme = isik_ayar.hiz / 48 + 2;
            yildiz_v[i]   = yildiz_v[i] > sonme ? yildiz_v[i] - sonme : 0;
            if (rastgele() == 0 && rastgele() < isik_ayar.hiz / 2 + 8) {
                yildiz_v[i] = 255;
                yildiz_h[i] = rastgele();
            }
            return vurgu(yildiz_v[i], yildiz_h[i]);
        }
        case MOD_BIRIKME:
            if (birik_v[i] && rastgele() < isik_ayar.hiz / 4 + 8) birik_v[i]--;
            return vurgu(birik_v[i], x + y);
        case MOD_VISOR: {
            int16_t pos = (uint16_t)ucgen(c_t, 3000) * 224 / 255;
            int16_t d   = mutlak(x - pos);
            return vurgu(d < 40 ? 255 - d * 255 / 40 : 0, x);
        }
        case MOD_SES: return ses_isik(i, x, y, dx, dy);
        case MOD_EKRAN: return ekran_isik(x);
        case MOD_BOYAMA: return boya_isik(i);
        case MOD_HIZ: return karisim(hiz_gosterge);
        case MOD_YUKSELME: {
            int16_t pos = 88 - (int16_t)((c_t / 16) % 120);
            int16_t d   = mutlak(y - pos);
            return vurgu(d < 24 ? 255 - d * 255 / 24 : 0, y * 4);
        }
        default: {
            RGB o = {0, 0, 0};
            return o;
        }
    }
}

/* ---------------- Yan ışık yardımcıları ---------------- */
// Kenar LED'inin yan şeritteki yeri: 0 = en üst küçük ışık .. 5 = uzun çubuğun altı (iki yanda aynı); 0xFF = yok
static const uint8_t KENAR_SIRA[18] = {1, 2, 3, 4, 5, 0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 1, 2, 3, 4, 5, 0, 0};
// Hızlı yükselip yavaşça sönen tek vuruş: p anında, bas'ta başlayan, tepe değeri tepe olan
static uint8_t kenar_vurus(uint16_t p, uint16_t bas, uint8_t tepe) {
    if (p < bas || p >= bas + 300) return 0;
    uint16_t d = p - bas;
    return d < 60 ? (uint16_t)tepe * d / 60 : (uint16_t)tepe * (300 - d) / 240;
}
static uint16_t mum_adim(uint8_t k, uint32_t s) {
    uint32_t h = (uint32_t)k * 2654435761u ^ s * 40503u;
    h ^= h >> 13; h *= 0x5bd1e995u; h ^= h >> 15;
    return 110 + (h & 0x7F) + ((h >> 8) & 0x1F); // 110..268 -> en çok 255'e kırpılır
}
static uint8_t mum(uint8_t k, uint32_t kt) {
    uint32_t s = kt / 90;
    uint16_t a = mum_adim(k, s), b = mum_adim(k, s + 1), f = kt % 90;
    uint16_t v = a + ((int16_t)(b - a) * (int16_t)f) / 90;
    return v > 255 ? 255 : v;
}

static bool SIMSEK(effect_params_t* params) {
    RGB_MATRIX_USE_LIMITS(led_min, led_max);

    uint8_t mod    = isik_ayar.mod;
    uint8_t genel  = isik_ayar.genel_parlak;
    uint8_t tabanp = isik_ayar.taban_parlak;
    if (mod == MOD_KAPALI) genel = 0;

    // Taban: renk x taban parlaklığı x genel parlaklık
    uint16_t ty = (uint16_t)tabanp * genel / 100;
    uint8_t  tr = olcekle_g(isik_ayar.taban_r, ty);
    uint8_t  tg = olcekle_g(isik_ayar.taban_g, ty);
    uint8_t  tb = olcekle_g(isik_ayar.taban_b, ty);
    // Parlama: renk x genel parlaklık
    uint8_t pr = olcekle_g(isik_ayar.parla_r, genel);
    uint8_t pg = olcekle_g(isik_ayar.parla_g, genel);
    uint8_t pb = olcekle_g(isik_ayar.parla_b, genel);

    bool tepki = (mod == MOD_SIMSEK || mod == MOD_DALGA) && genel > 0;

    for (uint8_t j = 0; j < VURUS_SAYISI; j++) {
        vurus_tick[j] = 0xFFFF;
        if (!vuruslar.dolu[j]) continue;
        uint16_t gecen = timer_elapsed(vuruslar.zaman[j]);
        if (gecen > 10000) {
            vuruslar.dolu[j] = 0; // çok eski, unut
            continue;
        }
        vurus_tick[j] = gecen;
    }

    // Kenar ışıkları için ortak değerler (karede bir kez)
    uint8_t  kenar_mod   = isik_ayar.kenar_mod;
    uint32_t kt          = (uint32_t)(((uint64_t)timer_read32() * (isik_ayar.kenar_hiz + 16)) >> 7); // kenar hızıyla ölçekli zaman
    uint16_t ky          = (uint16_t)isik_ayar.kenar_parlak * genel / 100;
    uint8_t  kar = olcekle_g(isik_ayar.kenar_a_r, ky), kag = olcekle_g(isik_ayar.kenar_a_g, ky), kab = olcekle_g(isik_ayar.kenar_a_b, ky);
    uint8_t  kbr = olcekle_g(isik_ayar.kenar_b_r, ky), kbg = olcekle_g(isik_ayar.kenar_b_g, ky), kbb = olcekle_g(isik_ayar.kenar_b_b, ky);
    uint16_t kenar_sure  = isik_ayar.sure < 300 ? 300 : isik_ayar.sure;
    uint8_t  kenar_cakma = 0; // KENAR_SIMSEK: son tuş vuruşundan sonra çakıp sönme
    if (kenar_mod == KENAR_SIMSEK) {
        uint16_t sure = kenar_sure;
        for (uint8_t j = 0; j < VURUS_SAYISI; j++) {
            uint16_t t = vurus_tick[j];
            if (t >= sure) continue;
            uint16_t v = 255 - (uint32_t)t * 255 / sure;
            v          = v * v / 255;
            if (t < 90 && ((t / 15) & 1)) v = v / 3; // gerçek şimşek gibi iki kez çakar
            if (v > kenar_cakma) kenar_cakma = v;
        }
    }
    // Nefes: 3 sn'lik üçgen dalga, karesel (yumuşak)
    uint16_t nefes_t = kt % 3000;
    uint16_t nefes   = nefes_t < 1500 ? (uint32_t)nefes_t * 255 / 1500 : (uint32_t)(3000 - nefes_t) * 255 / 1500;
    nefes            = nefes * nefes / 255;
    // Kalp atışı: güçlü + zayıf iki vuruş, sonra dinlenme
    uint8_t kalp = 0;
    {
        uint16_t p = kt % 1400;
        kalp       = kenar_vurus(p, 0, 255);
        uint8_t v2 = kenar_vurus(p, 330, 170);
        if (v2 > kalp) kalp = v2;
    }

    // Orijinal animasyonlar için ortak değerler
    c_tr = tr; c_tg = tg; c_tb = tb;
    c_pr = pr; c_pg = pg; c_pb = pb;
    c_genel = genel;
    c_t     = timer_read32() * (isik_ayar.hiz + 16) / 128;
    if (mod == MOD_BIRIKME) birikme_isle();
    {
        uint16_t sg = timer_elapsed(ses_zaman);
        ses_carpan  = sg < 300 ? 255 : sg < 800 ? 255 - (uint32_t)(sg - 300) * 255 / 500 : 0;
        if (sg > 60000) ses_zaman = timer_read() - 60000; // taşma olmasın
    }
    if (mr_goster && timer_elapsed(mr_zaman) >= 600) mr_goster = false;
    if (kenar_test != 0xFF && timer_elapsed(kenar_test_zaman) > 30000) kenar_test = 0xFF; // uygulama kapanırsa kendiliğinden bitsin
    if (timer_elapsed(ekran_zaman) > 60000) ekran_zaman = timer_read() - 60000;
    if (timer_elapsed(ekran_kenar_zaman) > 60000) ekran_kenar_zaman = timer_read() - 60000;
    bool ekran_kenar_var = timer_elapsed(ekran_kenar_zaman) < 1500;
    if (timer_elapsed(sistem_zaman) > 60000) sistem_zaman = timer_read() - 60000;
    bool sistem_var = timer_elapsed(sistem_zaman) < 3000;
    if (mod == MOD_HIZ || kenar_mod == KENAR_HIZ) {
        uint8_t h  = yazma_hizi();
        uint8_t he = h >= 100 ? 255 : (uint16_t)h * 255 / 100; // 10 vuruş/sn (~120 kelime/dk) = tam
        if (he > hiz_gosterge) hiz_gosterge += (he - hiz_gosterge + 3) / 4;
        else if (he < hiz_gosterge) hiz_gosterge -= (hiz_gosterge - he + 15) / 16;
    }
    // Bildirim ve mikrofon: Volt kapanırsa mikrofon ışığı kendiliğinden söner
    if (mikrofon_kapali && timer_elapsed(mikrofon_zaman) > 6000) mikrofon_kapali = false;
    bool uyari_yan = false, uyari_var = false;
    if (uyari_adet) {
        uint16_t e = timer_elapsed(uyari_zaman);
        if (e < (uint16_t)uyari_adet * 500) { uyari_var = true; uyari_yan = e % 500 < 280; }
        else uyari_adet = 0;
    }
    uint8_t ug = genel < 40 ? 40 : genel;
    // Göstergeler: kilitler ve Fn katmanı
    led_t   kilit  = host_keyboard_led_state();
    bool    fn_gos = (gosterge_bit & GOSTERGE_FN) && layer_state_is(1);
    uint8_t gr = olcekle_g(gosterge_r, ug), gg = olcekle_g(gosterge_g, ug), gb = olcekle_g(gosterge_b, ug);
    uint8_t uyr = olcekle_g(uyari_r, ug), uyg = olcekle_g(uyari_g, ug), uyb = olcekle_g(uyari_b, ug);
    // Açılış: dalganın önü (LED x koordinatı) ve rengi (parlama rengi; çok koyuysa beyaz)
    uint16_t acilis_e = 0;
    uint8_t  ar = pr, ag = pg, ab = pb;
    if (acilis_aktif) {
        acilis_e = timer_elapsed(acilis_zaman);
        if (acilis_e >= ACILIS_SURE || genel == 0 || acilis_stil == ACILIS_KAPALI) acilis_aktif = false;
        if ((uint16_t)ar + ag + ab < 90) ar = ag = ab = olcekle_g(255, genel);
    }

    for (uint8_t i = led_min; i < led_max; i++) {
        RGB_MATRIX_TEST_LED_FLAGS();
        uint8_t r, g, b;
        if (mod >= MOD_NEFES) {
            RGB c = {0, 0, 0};
            if (genel) c = ekstra_mod(i, mod);
            r = c.r; g = c.g; b = c.b;
        } else {
            uint8_t a = tepki ? simsek_isik(i) : 0;
            r = tr + (int16_t)(pr - tr) * a / 255;
            g = tg + (int16_t)(pg - tg) * a / 255;
            b = tb + (int16_t)(pb - tb) * a / 255;
        }
        // Kenar ışıkları: kendi animasyonları (KENAR_TUSLAR ise tuşlarla aynı davranır)
        if (i >= LED_KENAR_ILK && kenar_mod != KENAR_TUSLAR) {
            uint8_t k = i - LED_KENAR_ILK;
            uint8_t n = KENAR_SIRA[k]; // üstten alta yer (0..5)
            if (kenar_mod == KENAR_KAPALI || genel == 0 || n == 0xFF) {
                r = g = b = 0;
            } else if (kenar_mod == KENAR_EKRAN) {
                // Sol şerit ekranın sol kenarı, sağ şerit sağ kenarı; üstten (0) alta (5) 4 renk arasında geçiş
                if (!ekran_kenar_var) {
                    r = g = b = 0;
                } else {
                    const uint8_t *e = &ekran_kenar[g_led_config.point[i].x < 112 ? 0 : EKRAN_KENAR * 3];
                    uint16_t p = (uint16_t)n * (EKRAN_KENAR - 1) * 256 / 5;
                    uint8_t  z = p >> 8, f = p & 255;
                    if (z >= EKRAN_KENAR - 1) { z = EKRAN_KENAR - 2; f = 255; }
                    const uint8_t *a = e + z * 3, *c = a + 3;
                    r = olcekle_g(karis(a[0], c[0], f), ky); g = olcekle_g(karis(a[1], c[1], f), ky); b = olcekle_g(karis(a[2], c[2], f), ky);
                }
            } else if (kenar_mod == KENAR_SISTEM) {
                // Alttan yukarı dolar; altta yeşil, ortada sarı, üstte kırmızı (yük arttıkça kırmızıya ulaşır)
                if (!sistem_var) {
                    r = g = b = 0;
                } else {
                    uint8_t sv = g_led_config.point[i].x < 112 ? sistem_sol : sistem_sag;
                    int16_t h  = (int16_t)sv * 6 * 16 / 255 - (5 - n) * 16;
                    uint8_t a3 = h <= 0 ? 0 : h >= 16 ? 255 : h * 16;
                    uint8_t p  = (5 - n) * 51;
                    uint8_t cr = p < 128 ? p * 2 : 255, cg = p < 128 ? 255 : (255 - p) * 2;
                    r = (uint16_t)olcekle_g(cr, ky) * a3 / 255; g = (uint16_t)olcekle_g(cg, ky) * a3 / 255; b = 0;
                }
            } else if (kenar_mod == KENAR_HIZ) {
                // Gösterge: alttan yukarı dolar; altta ana renk, üstte ikinci renk
                int16_t h  = (int16_t)hiz_gosterge * 6 * 16 / 255 - (5 - n) * 16;
                uint8_t a3 = h <= 0 ? 0 : h >= 16 ? 255 : h * 16;
                uint8_t m  = (5 - n) * 51;
                r = (uint16_t)karis(kar, kbr, m) * a3 / 255; g = (uint16_t)karis(kag, kbg, m) * a3 / 255; b = (uint16_t)karis(kab, kbb, m) * a3 / 255;
            } else if (kenar_mod == KENAR_GOKKUSAGI || kenar_mod == KENAR_SPEKTRUM) {
                uint8_t ton = kt / 12 + (kenar_mod == KENAR_GOKKUSAGI ? n * 30 : 0);
                HSV     h   = {ton, 255, (uint8_t)((uint16_t)255 * ky / 100)};
                RGB c = hsv_to_rgb(h);
                r = c.r; g = c.g; b = c.b;
            } else if (kenar_mod == KENAR_SABIT || kenar_mod == KENAR_NEFES || kenar_mod == KENAR_KALP || kenar_mod == KENAR_MUM) {
                // Tek renk (A), parlaklığı değişir
                uint8_t s = kenar_mod == KENAR_SABIT ? 255
                          : kenar_mod == KENAR_NEFES ? 20 + nefes * 235 / 255
                          : kenar_mod == KENAR_KALP  ? 40 + kalp * 215 / 255
                                                     : mum(k, kt);
                r = (uint16_t)kar * s / 255; g = (uint16_t)kag * s / 255; b = (uint16_t)kab * s / 255;
            } else if (kenar_mod == KENAR_AKIS) {
                // A ile B arasında çevrede akan geçiş
                uint16_t p  = ((uint16_t)n * 512 / 6 + kt / 6) & 511;
                uint8_t  a2 = p < 256 ? p : 511 - p;
                r = karis(kar, kbr, a2); g = karis(kag, kbg, a2); b = karis(kab, kbb, a2);
            } else {
                // Sönük A üzerinde B ile parlayan modlar
                uint8_t a2 = 0;
                if (kenar_mod == KENAR_SIMSEK) {
                    a2 = kenar_cakma;
                } else if (kenar_mod == KENAR_KOMET) {
                    // İki yanda yukarıdan aşağı akan kuyruklu ışık (6 yer + 3 boşluk)
                    const uint16_t tur = 9 * 256, adim = 200;
                    uint16_t bas = (uint32_t)(kt % (9u * adim)) * 256 / adim;
                    uint16_t d   = (bas + 2 * tur - (uint16_t)n * 256) % tur; // başın ne kadar gerisinde
                    if (d < 3 * 256) { uint16_t v = 255 - (uint32_t)d * 255 / (3 * 256); a2 = v * v / 255; }
                } else if (kenar_mod == KENAR_TARAMA) {
                    uint16_t p   = kt % 2400;
                    // Yukarı aşağı gidip gelen tarama (iki yanda aynı)
                    int16_t  poz = 6 + (p < 1200 ? (uint32_t)p * 62 / 1200 : (uint32_t)(2400 - p) * 62 / 1200);
                    int16_t  d   = (int16_t)g_led_config.point[i].y - poz;
                    if (d < 0) d = -d;
                    if (d < 16) a2 = 255 - d * 255 / 16;
                } else if (kenar_mod == KENAR_SES) {
                    // Seviye göstergesi: alttan yukarı dolar; sol bas, sağ tiz
                    bool     sol = g_led_config.point[i].x < 112;
                    uint16_t t   = 0;
                    for (uint8_t j = sol ? 0 : 8; j < (sol ? 4 : 16); j++) t += ses_bant[j];
                    uint8_t  sv  = ses_al(sol ? t / 4 : t / 8);
                    int16_t  h   = (int16_t)sv * 7 * 16 / 255 - (5 - n) * 16; // alttan sayılan yer
                    uint8_t  a3  = h <= 0 ? 0 : h >= 16 ? 255 : h * 16;
                    uint8_t  m   = (5 - n) * 51; // altta ana renk, üstte ikinci renk
                    r = (uint16_t)karis(kar, kbr, m) * a3 / 255; g = (uint16_t)karis(kag, kbg, m) * a3 / 255; b = (uint16_t)karis(kab, kbb, m) * a3 / 255;
                    goto kenar_bitti;
                } else if (kenar_mod == KENAR_YANSIMA) {
                    for (uint8_t j = 0; j < VURUS_SAYISI; j++) {
                        uint16_t t = vurus_tick[j];
                        if (t >= kenar_sure) continue;
                        int16_t dx = (int16_t)g_led_config.point[i].x - vuruslar.x[j];
                        int16_t dy = (int16_t)g_led_config.point[i].y - vuruslar.y[j];
                        uint16_t d = (dx < 0 ? -dx : dx) + (dy < 0 ? -dy : dy);
                        if (d >= 80) continue;
                        uint16_t v = 255 - (uint32_t)t * 255 / kenar_sure;
                        v          = v * (255 - d * 255 / 80) / 255;
                        if (v > a2) a2 = v;
                    }
                }
                r = karis(kar / 2, kbr, a2); g = karis(kag / 2, kbg, a2); b = karis(kab / 2, kbb, a2);
            }
        kenar_bitti:;
        }
        // Bildirim yanıp sönmesi ve mikrofon kapalı uyarısı (kenar ışıkları)
        if (i >= LED_KENAR_ILK && KENAR_SIRA[i - LED_KENAR_ILK] != 0xFF) {
            if (uyari_var) { r = uyari_yan ? uyr : 0; g = uyari_yan ? uyg : 0; b = uyari_yan ? uyb : 0; }
            else if (mikrofon_kapali && KENAR_SIRA[i - LED_KENAR_ILK] == 0) { r = olcekle_g(255, ug); g = b = 0; }
        }
        // Makrosu olmayan G tuşları ve işlevsiz M tuşları kapalı
        if ((i >= LED_G1 && i < LED_G1 + 5 && makro_uzunluk(i - LED_G1) == 0) || (i == LED_TEKER && !teker_isik)) {
            r = g = b = 0;
        }
        // M1..M4: seçili profil parlama renginde yanar; MR: geri alınınca kısa süre yanar
        if (i >= LED_M1 && i < LED_M1 + 5) {
            bool yan = i < LED_M1 + 4 ? (i - LED_M1 == profil) : (mr_goster && timer_elapsed(mr_zaman) < 600);
            if (yan) { r = pr; g = pg; b = pb; } else { r = g = b = 0; }
        }
        // Göstergeler: Fn basılıyken sadece Fn ile çalışan tuşlar; kilit açıkken kendi tuşu
        if (fn_gos && i < LED_KENAR_ILK) {
            if (fn_isik[i >> 3] & (1 << (i & 7))) { r = gr; g = gg; b = gb; } else { r = g = b = 0; }
        } else if (((gosterge_bit & GOSTERGE_CAPS) && kilit.caps_lock && i == LED_CAPS) || ((gosterge_bit & GOSTERGE_NUM) && kilit.num_lock && i == LED_NUM) ||
                   ((gosterge_bit & GOSTERGE_SCROLL) && kilit.scroll_lock && i == LED_SCROLL) || (keymap_config.no_gui && i == LED_WIN)) {
            r = gr; g = gg; b = gb;
        } else if ((gosterge_bit & GOSTERGE_NUM) && !kilit.num_lock && NUMPAD_SAYI(i)) {
            r = g = b = 0; // Num Lock kapalı: sayı tuşları söner
        }
        // Açılış animasyonu: her LED sırası gelince (t0, ms) parlak renkte yanar, sonra asıl efekte döner
        if (acilis_aktif) {
            int16_t x = g_led_config.point[i].x, y = g_led_config.point[i].y, t0;
            switch (acilis_stil) {
                case ACILIS_MERKEZ: {
                    int16_t dx = mutlak(x - 112), dy = mutlak(y - 32) * 2;
                    t0 = (int32_t)(dx > dy ? dx + dy / 2 : dy + dx / 2) * 750 / 145;
                    break;
                }
                case ACILIS_YUKARI: t0 = (int32_t)y * 750 / 64; break;
                case ACILIS_KIVILCIM: t0 = (uint8_t)(i * 97 + 31) * 3; break; // sözde rastgele sıra
                case ACILIS_PERDE: t0 = (int32_t)(112 - mutlak(x - 112)) * 750 / 112; break;
                default: t0 = (int32_t)x * 750 / 224; break; // dalga, gökkuşağı
            }
            uint8_t fr = ar, fg = ag, fb = ab;
            if (acilis_stil == ACILIS_GOKKUSAGI) { RGB c = renk_hsv(x + acilis_e / 4, 255); fr = c.r; fg = c.g; fb = c.b; }
            int16_t lt = (int16_t)acilis_e - t0;
            if (lt < 0) {
                uint8_t s = (acilis_stil != ACILIS_KIVILCIM && lt > -80) ? (80 + lt) * 2 : 0; // önde hafif parıltı
                r = (uint16_t)fr * s / 255; g = (uint16_t)fg * s / 255; b = (uint16_t)fb * s / 255;
            } else if (lt < ACILIS_KUYRUK) {
                uint8_t m = (uint32_t)lt * 255 / ACILIS_KUYRUK;
                r = karis(fr, r, m); g = karis(fg, g, m); b = karis(fb, b, m);
            }
        }
        // Bildirim yanıp sönmesi sürerken (istenirse tuşlarda da) animasyon görünmez: yanarken renk, sönükken kapalı
        if (uyari_var && (i >= LED_KENAR_ILK ? KENAR_SIRA[i - LED_KENAR_ILK] != 0xFF : uyari_tuslar)) {
            r = uyari_yan ? uyr : 0; g = uyari_yan ? uyg : 0; b = uyari_yan ? uyb : 0;
        }
        // Kenar ışığı tanıma: sadece seçilen kenar LED'i beyaz
        if (kenar_test != 0xFF && i >= LED_KENAR_ILK) {
            r = g = b = (i - LED_KENAR_ILK == kenar_test) ? 255 : 0;
        }
        // Makro kaydı: Rec kırmızı yanıp söner; kayıtta seçilen G tuşu da kırmızı yanar
        if (makro_durum != MAKRO_BOSTA) {
            bool yanik = (timer_read() / 250) & 1;
            if (i == LED_REC || (makro_durum == MAKRO_KAYIT && i == LED_G1 + makro_yuva)) {
                r = (i == LED_REC && !yanik) ? 0 : 255;
                g = 0;
                b = 0;
            }
        }
        // Kanal sırası düzeltmesi: bu klavyede sürücü kanalları kaydırıyor (denendi: varsayılan 3 doğru)
        switch (isik_ayar.renk_sirasi) {
            case 0: rgb_matrix_set_color(i, r, g, b); break;
            case 1: rgb_matrix_set_color(i, r, b, g); break;
            case 2: rgb_matrix_set_color(i, g, r, b); break;
            case 3: rgb_matrix_set_color(i, g, b, r); break;
            case 4: rgb_matrix_set_color(i, b, r, g); break;
            default: rgb_matrix_set_color(i, b, g, r); break;
        }
    }
    return rgb_matrix_check_finished_leds(led_max);
}

#endif
