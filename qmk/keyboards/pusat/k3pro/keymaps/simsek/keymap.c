// Copyright 2026 Eresbos (github.com/ERESB0S)
// SPDX-License-Identifier: GPL-2.0-or-later
/* Pusat K3 Pro – "simsek" keymap.
 * Tuş kodları stok firmware'in tuş tablosuyla aynı (Türkçe Q düzenini Windows çözüyor).
 * Matris: 7 satır x 20 sütun (DONANIM_HARITASI.md).
 */
#include QMK_KEYBOARD_H

#include <string.h>
#include "raw_hid.h"
#include "usb_main.h"
#include "Flash.h"
#include "ayarlar.h"

enum katmanlar { _TEMEL, _FN };

enum ozel_tuslar {
    ISIK_AC = SAFE_RANGE, // Fn+F8
    MOD_ONC,              // Fn+F9
    MOD_SON,              // Fn+F10
    SURE_AZ,              // Fn+F11
    SURE_CO,              // Fn+F12
    PARL_AR,              // Fn+Yukarı
    PARL_AZ,              // Fn+Aşağı
    YARI_AZ,              // Fn+Sol
    YARI_AR,              // Fn+Sağ
    MAKRO_REC,            // Rec tuşu
    TEKER_MOD,            // medya tuşlarının yanındaki geçiş tuşu: tekerlek = ses / ışık parlaklığı
    PROFIL_1, PROFIL_2, PROFIL_3, PROFIL_4, // M1..M4: profil seç
    PROFIL_GERI,          // MR: profili kayıtlı hâline döndür
    TUS_COKLU,            // atanmış tuş + ek tuşlar birlikte (tuş atama, sürüm 17)
};

// G tuşları: G1 = Ctrl + Num *, G2 = Ctrl + Num -, G3 = Ctrl + Num + (kullanıcının eski atamaları)
#define TUS_G1 KC_NO // G tuşları sadece Rec ile kaydedilen makroyu oynatır
#define TUS_G2 KC_NO
#define TUS_G3 KC_NO
#define TUS_G4 KC_NO
#define TUS_G5 KC_NO

const uint16_t PROGMEM keymaps[][MATRIX_ROWS][MATRIX_COLS] = {
    /*          0        1        2        3        4        5        6        7        8        9        10       11       12       13       14       15       16       17       18       19 */
    [_TEMEL] = {
        {   KC_ESC,  KC_F1,   KC_F2,   KC_F3,   KC_F4,   KC_F5,   KC_F6,   KC_F7,   KC_F8,   KC_F9,   KC_F10,  KC_F11,  KC_F12,  KC_MNXT, KC_PSCR, KC_SCRL, KC_PAUS, TEKER_MOD,KC_MPRV, KC_MPLY },
        {   KC_GRV,  KC_1,    KC_2,    KC_3,    KC_4,    KC_5,    KC_6,    KC_7,    KC_8,    KC_9,    KC_0,    KC_MINS, KC_EQL,  KC_BSPC, KC_INS,  KC_HOME, KC_PGUP, KC_NUM,  KC_PSLS, KC_PAST },
        {   KC_TAB,  KC_Q,    KC_W,    KC_E,    KC_R,    KC_T,    KC_Y,    KC_U,    KC_I,    KC_O,    KC_P,    KC_LBRC, KC_RBRC, KC_BSLS, KC_DEL,  KC_END,  KC_PGDN, KC_P7,   KC_P8,   KC_P9   },
        {   KC_CAPS, KC_A,    KC_S,    KC_D,    KC_F,    KC_G,    KC_H,    KC_J,    KC_K,    KC_L,    KC_SCLN, KC_QUOT, KC_NUHS, KC_ENT,  KC_NO,   KC_NO,   KC_NO,   KC_P4,   KC_P5,   KC_P6   },
        {   KC_LSFT, KC_NUBS, KC_Z,    KC_X,    KC_C,    KC_V,    KC_B,    KC_N,    KC_M,    KC_COMM, KC_DOT,  KC_SLSH, KC_INT1, KC_RSFT, KC_NO,   KC_UP,   KC_NO,   KC_P1,   KC_P2,   KC_P3   },
        {   KC_LCTL, KC_LGUI, KC_LALT, KC_SPC,  KC_RALT, MO(_FN), KC_APP,  KC_PMNS, KC_RCTL, KC_PENT, KC_PPLS, KC_NO,   KC_NO,   KC_NO,   KC_LEFT, KC_DOWN, KC_RGHT, KC_NO,   KC_P0,   KC_PDOT },
        {   TUS_G1,  TUS_G2,  TUS_G3,  TUS_G4,  TUS_G5,  MAKRO_REC,  KC_NO,   KC_NO,   KC_NO,   KC_NO,   KC_NO,   KC_NO,   KC_NO,   KC_NO,   KC_NO,   PROFIL_1, PROFIL_2, PROFIL_3, PROFIL_4, PROFIL_GERI }
    },
    /* Fn katmanı (sağ Alt'ın yanındaki Fn tuşu) – orijinal yazılımla aynı kısayollar:
     *   Fn + F1 medya oynatıcı, F2 ses -, F3 ses +, F4 sessiz, F5 durdur, F6 önceki, F7 oynat/duraklat,
     *   F8 sonraki, F9 e-posta, F10 tarayıcı ana sayfa, F11 hesap makinesi, F12 arama
     *   Fn + Win          : Windows tuşunu kilitle/aç (kilit ışığı yanar)
     * Işık (geçici; kalıcı yapmak için uygulamada "Klavyeye kaydet"):
     *   Fn + PrtSc        : sonraki ışık modu (şimşek, dalga, sabit, kapalı)
     *   Fn + ScrLk        : ışıkları aç/kapat
     *   Fn + Yukarı/Aşağı : genel parlaklık
     *   Fn + Sol/Sağ      : yayılma / dalga kalınlığı
     *   Fn + * / -        : süre kısa / uzun (numaraların sağındaki iki tuş)
     *   Fn + Esc          : güncelleme moduna geç (geri yükleme / yeni yazılım için)
     */
    [_FN] = {
        {   RESET  , KC_MSEL, KC_VOLD, KC_VOLU, KC_MUTE, KC_MSTP, KC_MPRV, KC_MPLY, KC_MNXT, KC_MAIL, KC_WHOM, KC_CALC, KC_WSCH, _______, MOD_SON, ISIK_AC, _______, _______, _______, _______ },
        {   _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, SURE_AZ, SURE_CO, _______, _______, _______, _______, _______, _______, _______ },
        {   _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______ },
        {   _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______ },
        {   _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, PARL_AR, _______, _______, _______, _______ },
        {   _______, GUI_TOG, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, YARI_AZ, PARL_AZ, YARI_AR, _______, _______, _______ },
        {   _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______, _______ }
    }
};

/* Ses tekerleği */
/* Ses tekerleği: TEKER_MOD tuşu ses ile ışık parlaklığı arasında geçiş yapar */
bool teker_isik = false;

bool encoder_update_user(uint8_t index, bool clockwise) {
    if (teker_isik) {
        uint8_t p = isik_ayar.genel_parlak;
        isik_ayar.genel_parlak = clockwise ? (p > 95 ? 100 : p + 5) : (p < 5 ? 0 : p - 5);
    } else {
        tap_code(clockwise ? KC_VOLU : KC_VOLD);
    }
    return false;
}

/* ---------------- Işık ayarları ---------------- */

isik_ayar_t isik_ayar;
vurus_t     vuruslar;

_Static_assert(sizeof(isik_ayar_t) == 28, "isik_ayar_t 28 bayt olmali (uygulama ile ayni)");

static const isik_ayar_t VARSAYILAN = {
    .mod          = MOD_DALGA,
    .taban_r      = 0,   .taban_g = 0,   .taban_b = 255, // mavi
    .parla_r      = 220, .parla_g = 230, .parla_b = 255, // beyaz (hafif mavi)
    .taban_parlak = 45,
    .genel_parlak = 100,
    .yaricap      = 34,
    .sure         = 700,  // dalga: klavyeyi baştan sona geçme süresi
    .titreme      = 1,
    .kalinlik     = 40,   // ~4 tuş
    .iz           = 40,
    .tetik        = TETIK_BASINCA,
    .kenar_mod    = KENAR_SIMSEK,
    .hiz          = 128,
    .renkli       = 0,
    .renk_sirasi  = 3, // v3 pin sırasıyla denendi: mavi->kırmızı, kırmızı->yeşil, yeşil->mavi kayıyordu
    .kenar_a_r    = 0,   .kenar_a_g = 0,   .kenar_a_b = 255, // mavi
    .kenar_b_r    = 220, .kenar_b_g = 230, .kenar_b_b = 255, // beyaz
    .kenar_parlak = 90,
    .kenar_hiz    = 128,
};

static uint8_t onceki_mod = MOD_DALGA; // Fn+F8 ile kapatıp açınca geri dönülecek mod

static void ayar_sinirla(isik_ayar_t *a) {
    if (a->mod >= MOD_SAYISI) a->mod = MOD_SIMSEK;
    if (a->taban_parlak > 100) a->taban_parlak = 100;
    if (a->genel_parlak > 100) a->genel_parlak = 100;
    if (a->yaricap < 8) a->yaricap = 8;
    if (a->yaricap > 120) a->yaricap = 120;
    if (a->sure < 100) a->sure = 100;
    if (a->sure > 3000) a->sure = 3000;
    if (a->titreme >= SES_STIL_SAYISI) a->titreme = 1; // şimşekte 0/1, ses modunda stil
    if (a->renk_sirasi > 5) a->renk_sirasi = 3;
    if (a->kalinlik < 8) a->kalinlik = 8;
    if (a->kalinlik > 120) a->kalinlik = 120;
    if (a->iz > 100) a->iz = 100;
    if (a->tetik > TETIK_IKISI) a->tetik = TETIK_BASINCA;
    if (a->kenar_mod >= KENAR_SAYISI) a->kenar_mod = KENAR_SIMSEK;
    a->renkli = a->renkli ? 1 : 0;
    if (a->kenar_parlak > 100) a->kenar_parlak = 100;
}

/* Kalıcı kayıt: flash'ta, yazılımın çok ötesindeki tek bir 64 baytlık sayfa.
 * (QMK'nın kendi "EEPROM"u bu çipte silmeden yazdığı için kullanılmıyor.)
 * Flash ömrü binlerce yazmaya yeter; sadece uygulamada "Klavyeye kaydet" ile yazılır. */
/* Flash yerleşimi (0xF000..0xFBFF):
 *   0xF000 + p*64        : profil p'nin (M1..M4) ışık ayarı
 *   0xF100 + (p*5+n)*128 : profil p'nin G(n+1) makrosu
 *   0xFB00               : seçili profil */
#define PROFIL_SAYISI 4
#define KAYIT_ADRES(p) (0xF000u + (uint32_t)(p) * 64u)
#define MAKRO_ADRES_P(p, n) (0xF100u + ((uint32_t)(p) * 5u + (n)) * 128u)
#define MAKRO_ADRES(n) MAKRO_ADRES_P(profil, n)
#define PROFIL_ADRES 0xFB00u
#define GOSTERGE_ADRES 0xFB40u // göstergeler: [0] 0xA0 | bitler, [1..3] renk
#define GOSTERGE_IMZA 0xA0u     // yazılım yüklenince bu alan 00 ile dolu gelir; imzasızsa varsayılanı kullan
#define ACILIS_ADRES 0xFB80u   // açılış animasyonu: [0] 0xC0 | stil
#define ACILIS_IMZA 0xC0u
#define PROFIL_IMZA 0x50524F00u // "PRO" + profil no
#define KAYIT_IMZA 0x41334B50u // "PK3A"

uint8_t  profil   = 0;     // M1..M4 = 0..3
uint16_t mr_zaman = 0;     // MR'a basılma anı (ışığı kısa süre yanar)
bool     mr_goster = false;
uint8_t  ses_bant[SES_BANT];
uint8_t  ses_seviye = 0, ses_ton = 0, ses_bas = 0;
uint16_t ses_zaman  = 0;
uint8_t  ekran_renk[EKRAN_BOLGE * 3];
uint16_t ekran_zaman = 0;
uint8_t  ekran_kenar[2 * EKRAN_KENAR * 3];
uint16_t ekran_kenar_zaman = 0;
uint8_t  uyari_r, uyari_g, uyari_b, uyari_adet = 0;
bool     uyari_tuslar = false;
uint8_t  sistem_sol = 0, sistem_sag = 0;
uint16_t sistem_zaman = 0;
/* Tuş istatistiği: LED başına basış sayısı (sadece sayı; ne yazıldığı tutulmaz). Volt okuyup sıfırlar. */
#define SAYAC_LED 122
static uint16_t tus_sayac[SAYAC_LED];
uint16_t uyari_zaman = 0;
bool     mikrofon_kapali = false;
uint16_t mikrofon_zaman  = 0;

/* Yazma hızı: 250 ms'lik 8 kova (son 2 sn) */
static uint8_t hiz_kova[8], hiz_son;
static void hiz_ilerle(void) {
    uint8_t simdi = (uint8_t)(timer_read32() / 250);
    uint8_t adim  = simdi - hiz_son;
    if (adim > 8) adim = 8;
    while (adim--) hiz_kova[++hiz_son & 7] = 0;
    hiz_son = simdi;
}
uint8_t yazma_hizi(void) {
    hiz_ilerle();
    uint16_t t = 0;
    for (uint8_t i = 0; i < 8; i++) t += hiz_kova[i];
    t *= 5;
    return t > 255 ? 255 : t;
}
uint8_t  kenar_test = 0xFF;
uint16_t kenar_test_zaman = 0;

typedef struct __attribute__((packed, aligned(4))) {
    uint32_t    imza;
    uint8_t     surum;
    uint8_t     uzunluk;
    uint16_t    toplam;
    isik_ayar_t ayar;
} kayit_t; // 36 bayt

static uint16_t toplam_hesapla(const isik_ayar_t *a) {
    const uint8_t *p = (const uint8_t *)a;
    uint16_t       t = 0x5A5A;
    for (uint8_t i = 0; i < sizeof(isik_ayar_t); i++) t = (uint16_t)(t * 31 + p[i]);
    return t;
}

static bool kayit_oku(uint8_t p, isik_ayar_t *a) {
    const kayit_t *k = (const kayit_t *)KAYIT_ADRES(p);
    if (k->imza != KAYIT_IMZA || k->surum != 3 || k->uzunluk != sizeof(isik_ayar_t)) return false;
    if (k->toplam != toplam_hesapla(&k->ayar)) return false;
    memcpy(a, &k->ayar, sizeof(isik_ayar_t));
    ayar_sinirla(a);
    return true;
}

static bool kayit_yaz(uint8_t p, const isik_ayar_t *a) {
    kayit_t k;
    memset(&k, 0, sizeof(k));
    k.imza    = KAYIT_IMZA;
    k.surum   = 3;
    k.uzunluk = sizeof(isik_ayar_t);
    memcpy(&k.ayar, a, sizeof(isik_ayar_t));
    k.toplam = toplam_hesapla(&k.ayar);

    if (memcmp((const void *)KAYIT_ADRES(p), &k, sizeof(k)) == 0) return true; // zaten aynı

    if (FLASH_EraseSector(KAYIT_ADRES(p)) != FLASH_OKAY) return false;
    const uint32_t *w = (const uint32_t *)&k;
    for (uint8_t i = 0; i < sizeof(k) / 4; i++) {
        if (FLASH_ProgramDWord(KAYIT_ADRES(p) + i * 4, w[i]) != FLASH_OKAY) return false;
    }
    return memcmp((const void *)KAYIT_ADRES(p), &k, sizeof(k)) == 0;
}

static uint8_t profil_oku(void) {
    uint32_t w = *(const uint32_t *)PROFIL_ADRES;
    return (w & 0xFFFFFF00u) == PROFIL_IMZA && (w & 0xFF) < PROFIL_SAYISI ? (w & 0xFF) : 0;
}

// Profile geç: o profilin kayıtlı ayarı yüklenir (yoksa varsayılan), seçim flash'a yazılır
static void profil_sec(uint8_t p) {
    if (p >= PROFIL_SAYISI) return;
    makro_durum = MAKRO_BOSTA; // yarım kalan makro kaydı iptal
    profil      = p;
    if (!kayit_oku(p, &isik_ayar)) isik_ayar = VARSAYILAN;
    if (isik_ayar.mod != MOD_KAPALI) onceki_mod = isik_ayar.mod;
    if (*(const uint32_t *)PROFIL_ADRES != (PROFIL_IMZA | p)) {
        FLASH_EraseSector(PROFIL_ADRES);
        FLASH_ProgramDWord(PROFIL_ADRES, PROFIL_IMZA | p);
    }
}

// MR: geçerli profili klavyeye son kaydedilen hâline döndür
static void profil_geri(void) {
    if (!kayit_oku(profil, &isik_ayar)) isik_ayar = VARSAYILAN;
    if (isik_ayar.mod != MOD_KAPALI) onceki_mod = isik_ayar.mod;
    mr_zaman  = timer_read();
    mr_goster = true;
}

/* ---------------- Bilgisayarla haberleşme (raw HID, 32 bayt) ----------------
 * İstek : [0]=komut [1]=0x4B ('K') [2..17]=ayar (sadece AYARLA'da)
 * Cevap : [0]=komut [1]=0x4B [2]=durum (0 = tamam) [3]=sürüm [4..19]=geçerli ayar
 */
enum {
    KMT_OKU      = 0xA1,
    KMT_AYARLA   = 0xA2, // anlık (RAM), kaydetmez
    KMT_KAYDET   = 0xA3, // geçerli ayarı flash'a yazar
    KMT_VARSAY   = 0xA4, // fabrika ayarına dön (RAM)
    KMT_KAYITLI  = 0xA5, // flash'taki kayıtlı ayarı geri yükle (RAM)
    HID_IMZA     = 0x4B,
    KMT_BILGI    = 0xA6, // ana sayfa için durum bilgisi (sürüm 2)
    KMT_MAKRO    = 0xA7, // G tuşu makrosunun içeriği (sürüm 3)
    KMT_PROFIL   = 0xA8, // [2] = profil (0..3): o profile geç (sürüm 5)
    KMT_KENAR_TEST = 0xA9, // [2] = kenar LED'i (0..17) tek başına yansın, 0xFF = bitir (sürüm 6)
    KMT_SES      = 0xAA, // [2..17] 16 bant, [18] seviye, [19] ton, [20] bas (sürüm 7); sadece RAM
    KMT_MAKRO_YAZ  = 0xAB, // [2]=profil [3]=yuva [4]=baş [5]=adet(<=12) [6]=1:son paket, kaydet [8..] olaylar (sürüm 8)
    KMT_PROFIL_YAZ = 0xAC, // [2]=profil [3..30]=ayar: o profilin kaydına yaz (yedekten geri yükleme)
    KMT_PROFIL_OKU = 0xAD, // [2]=profil: o profilin kayıtlı ayarı (cevap [4..]); durum 2 = kayıt yok
    KMT_GUNCELLE   = 0xAE, // [2..5]="BOOT": güncelleme moduna (ROM yükleyici) geç
    KMT_EKRAN      = 0xAF, // [2..25] 8 bölge RGB (sürüm 9); sadece RAM
    KMT_BOYA_YAZ   = 0xB0, // [2]=profil [3]=baş LED (8'in katı; 0 = sil) [4]=adet(<=8) [5]=1:son, imzala [8..31] RGB
    KMT_BOYA_OKU   = 0xB1, // [2]=profil [3]=baş LED: cevap [4..27] 8 LED RGB; durum 2 = boyama yok
    KMT_EKRAN_KENAR = 0xB3, // [2..13] sol 4 RGB (üstten alta) [14..25] sağ 4 RGB (sürüm 10); sadece RAM
    KMT_GOSTERGE   = 0xB4, // [2]=1: yaz ([3] bitler, [4..6] renk); cevap [4] bitler [5..7] renk (sürüm 11)
    KMT_UYARI      = 0xB2, // [2..4] RGB [5] yanıp sönme adedi (0xFF = değişmez) [6] mikrofon kapalı (0/1, 0xFF = değişmez)
    KMT_TUS_OKU    = 0xB5, // [2]=baş LED: cevap [4..31] 7 tuş x (varsayılan u16, atama u16); atanamaz tuşta varsayılan 0xFFFF (sürüm 13)
    KMT_TUS_YAZ    = 0xB6, // [2]=baş LED (çift; 0 = tabloyu sil) [3]=adet(<=12) [4]=1: son paket, imzala [6..29] kodlar (0xFFFF = atama yok)
    KMT_ACILIS     = 0xB7, // [2]=0 oku, 1 stili [3] yap ve kaydet, 2 [3] stilini şimdi oynat (kaydetmez); cevap [4] stil (sürüm 14)
    KMT_SISTEM     = 0xB8, // [2] sol şerit [3] sağ şerit doluluğu (0..255); sadece RAM (sürüm 15)
    KMT_TUS_EK_OKU = 0xBA, // [2]=baş LED: cevap [4..31] 14 tuşun ek tuşları (u16: ek1 | ek2 << 8) (sürüm 17)
    KMT_TUS_EK_YAZ = 0xBB, // KMT_TUS_YAZ ile aynı biçim; ek tuş tablosu (sürüm 17)
    KMT_SAYAC      = 0xB9, // [2]=baş LED [3]=1: okununca sıfırla; cevap [4..31] 14 LED x u16 basış sayısı
    PROTOKOL_SUR = 17,
};

static uint32_t tus_sayaci = 0; // açılıştan beri basılan tuş sayısı

uint8_t gosterge_bit = GOSTERGE_CAPS | GOSTERGE_NUM | GOSTERGE_SCROLL | GOSTERGE_FN, gosterge_r = 255, gosterge_g = 255, gosterge_b = 255;
uint8_t fn_isik[(DRIVER_LED_TOTAL + 7) / 8];

static void gosterge_oku(void) {
    uint32_t w = *(const uint32_t *)GOSTERGE_ADRES;
    if ((w & 0xF0) != GOSTERGE_IMZA) return; // hiç kaydedilmemiş: varsayılan
    gosterge_bit = w & 0x0F;
    gosterge_r = w >> 8; gosterge_g = w >> 16; gosterge_b = w >> 24;
}
static bool gosterge_yaz(void) {
    uint32_t w = GOSTERGE_IMZA | gosterge_bit | ((uint32_t)gosterge_r << 8) | ((uint32_t)gosterge_g << 16) | ((uint32_t)gosterge_b << 24);
    if (*(const uint32_t *)GOSTERGE_ADRES == w) return true;
    if (FLASH_EraseSector(GOSTERGE_ADRES) != FLASH_OKAY) return false;
    return FLASH_ProgramDWord(GOSTERGE_ADRES, w) == FLASH_OKAY;
}

bool     acilis_aktif = false;
uint16_t acilis_zaman = 0;
uint8_t  acilis_stil  = ACILIS_DALGA;
static uint8_t acilis_kayitli = ACILIS_DALGA; // önizleme bitince dönülecek stil
static void acilis_baslat(void) { acilis_aktif = true; acilis_zaman = timer_read(); }
static void acilis_oku(void) {
    uint32_t w = *(const uint32_t *)ACILIS_ADRES;
    if ((w & 0xF0) == ACILIS_IMZA && (w & 0x0F) < ACILIS_STIL_SAYISI) acilis_kayitli = w & 0x0F;
    acilis_stil = acilis_kayitli;
}
static bool acilis_yaz(uint8_t s) {
    acilis_kayitli = acilis_stil = s;
    uint32_t w = 0xFFFFFF00u | ACILIS_IMZA | s;
    if (*(const uint32_t *)ACILIS_ADRES == w) return true;
    if (FLASH_EraseSector(ACILIS_ADRES) != FLASH_OKAY) return false;
    return FLASH_ProgramDWord(ACILIS_ADRES, w) == FLASH_OKAY;
}

static void yaz32(uint8_t *p, uint32_t v) { p[0] = v; p[1] = v >> 8; p[2] = v >> 16; p[3] = v >> 24; }

/* ---------------- Tuş atama ----------------
 * LED numarasına göre (0..TUS_SAYI-1) yeni tuş kodu; 0xFFFF = atama yok, 0 = tuş devre dışı.
 * Sadece temel katman ve varsayılanı tek baytlık tuş kodu olanlar (Fn, tekerlek geçişi, G/M/Rec değişmez).
 * Flash 0xE600 (4 sayfa): [0] imza, [4 + i*2] kod. Tuş basılınca doğrudan flash'tan okunur. */
#define TUS_ADRES 0xE600u
#define TUS_SAYI  112
#define TUS_IMZA  0x4D525441u // "ATRM"

static bool tus_konum(uint8_t led, uint8_t *satir, uint8_t *sutun) {
    for (uint8_t r = 0; r < MATRIX_ROWS; r++)
        for (uint8_t c = 0; c < MATRIX_COLS; c++)
            if (g_led_config.matrix_co[r][c] == led) { *satir = r; *sutun = c; return true; }
    return false;
}

/* Ek tuşlar: atanmış tuşla birlikte basılacak en çok iki tuş daha. Flash 0xE700 (4 sayfa):
 * [0] imza, [4 + i*2] ek1 | ek2 << 8 (0 = yok). Yazılım yüklenince bölge 0 olur, imza olmadığından yok sayılır. */
#define EK_ADRES 0xE700u
#define EK_IMZA  0x4B455441u // "ATEK"

static uint16_t tus_ek(uint8_t l) {
    if (l >= TUS_SAYI || *(const uint32_t *)EK_ADRES != EK_IMZA) return 0;
    uint16_t e = ((const uint16_t *)(EK_ADRES + 4))[l];
    return e == 0xFFFF ? 0 : e;
}

uint16_t keymap_key_to_keycode(uint8_t layer, keypos_t key) {
    uint16_t k = pgm_read_word(&keymaps[layer][key.row][key.col]);
    if (layer == 0 && key.row < 6 && k <= 0xFF && *(const uint32_t *)TUS_ADRES == TUS_IMZA) {
        uint8_t l = g_led_config.matrix_co[key.row][key.col];
        if (l < TUS_SAYI) {
            uint16_t a = ((const uint16_t *)(TUS_ADRES + 4))[l];
            if (a <= QK_MODS_MAX) return a != 0 && tus_ek(l) ? TUS_COKLU : a;
        }
    }
    return k;
}

/* TUS_COKLU: tuşun atamasını ve ek tuşlarını birlikte bas / bırak */
static void tus_coklu(keyrecord_t *record) {
    uint8_t l = g_led_config.matrix_co[record->event.key.row][record->event.key.col];
    if (l >= TUS_SAYI) return;
    uint16_t a = ((const uint16_t *)(TUS_ADRES + 4))[l], e = tus_ek(l);
    if (record->event.pressed) {
        register_code16(a);
        if (e & 0xFF) register_code(e & 0xFF);
        if (e >> 8) register_code(e >> 8);
    } else {
        if (e >> 8) unregister_code(e >> 8);
        if (e & 0xFF) unregister_code(e & 0xFF);
        unregister_code16(a);
    }
}

/* Makro yardımcıları (aşağıda tanımlı) */
#define MAKRO_OLAY 63
static uint16_t makro_tampon[MAKRO_OLAY + 1] __attribute__((aligned(4)));
static uint8_t  makro_sayi;
static uint8_t  makro_uzunluk_p(uint8_t p, uint8_t n);
static void     makro_yaz(uint8_t p, uint8_t n);

void raw_hid_receive(uint8_t *data, uint8_t length) {
    if (length < 20 || data[1] != HID_IMZA) return; // bize ait değil, yok say
    uint8_t komut = data[0];
    uint8_t durum = 0;

    if (komut == KMT_BILGI) {
        /* [4..8] G1..G5 makro uzunluğu, [9] Win kilidi, [10] teker ışık modunda,
         * [11..14] tuş sayacı, [15..18] çalışma süresi (sn), [19] kayıtlı ayar var,
         * [20] RAM'deki ayar kayıtlıyla aynı, [21] seçili profil, [22] kayıtlı profiller (bit maskesi) */
        isik_ayar_t k;
        bool        var = kayit_oku(profil, &k);
        uint8_t     maske = 0;
        for (uint8_t p = 0; p < PROFIL_SAYISI; p++) maske |= kayit_oku(p, &k) << p;
        kayit_oku(profil, &k);
        memset(data, 0, length);
        data[0] = komut; data[1] = HID_IMZA; data[3] = PROTOKOL_SUR;
        for (uint8_t i = 0; i < 5; i++) data[4 + i] = makro_uzunluk(i);
        data[9]  = keymap_config.no_gui;
        data[10] = teker_isik;
        yaz32(&data[11], tus_sayaci);
        yaz32(&data[15], timer_read32() / 1000);
        data[19] = var;
        data[20] = var && memcmp(&k, &isik_ayar, sizeof(k)) == 0;
        data[21] = profil;
        data[22] = maske;
        yaz32(&data[23], timer_read32()); // [23..26] klavye saati (ms): uygulamadaki önizleme eşzamanlansın
        data[27] = yazma_hizi();          // [27] vuruş/sn x 10 (sürüm 9)
        raw_hid_send(data, length);
        return;
    }
    if (komut == KMT_MAKRO) {
        /* İstek [2]=G yuvası (0..4) [3]=başlangıç olayı [4]=profil+1 (0 = seçili profil, sürüm 8). Cevap [4]=toplam olay, [5]=bu paketteki olay,
         * [6..] olaylar (u16, küçük uçlu; 0x8000 = basıldı). Paket başına en çok 13 olay. */
        uint8_t yuva = data[2], bas = data[3], mp = data[4] >= 1 && data[4] <= PROFIL_SAYISI ? data[4] - 1 : profil;
        memset(data, 0, length);
        data[0] = komut; data[1] = HID_IMZA; data[3] = PROTOKOL_SUR;
        if (yuva < 5) {
            uint8_t         say = makro_uzunluk_p(mp, yuva);
            const uint16_t *o   = (const uint16_t *)MAKRO_ADRES_P(mp, yuva) + 1;
            uint8_t         n   = 0;
            for (uint8_t i = bas; i < say && n < 13; i++, n++) {
                data[6 + n * 2] = o[i] & 0xFF;
                data[7 + n * 2] = o[i] >> 8;
            }
            data[4] = say; data[5] = n;
        } else {
            data[2] = 1;
        }
        raw_hid_send(data, length);
        return;
    }

    switch (komut) {
        case KMT_OKU:
            break;
        case KMT_AYARLA: {
            isik_ayar_t yeni;
            memcpy(&yeni, &data[2], sizeof(yeni));
            ayar_sinirla(&yeni);
            isik_ayar = yeni;
            if (isik_ayar.mod != MOD_KAPALI) onceki_mod = isik_ayar.mod;
            break;
        }
        case KMT_KAYDET:
            durum = kayit_yaz(profil, &isik_ayar) ? 0 : 1;
            break;
        case KMT_VARSAY:
            isik_ayar = VARSAYILAN;
            break;
        case KMT_KAYITLI:
            if (!kayit_oku(profil, &isik_ayar)) {
                isik_ayar = VARSAYILAN;
                durum     = 2; // kayıt yoktu
            }
            break;
        case KMT_KENAR_TEST:
            kenar_test       = data[2] < 18 ? data[2] : 0xFF;
            kenar_test_zaman = timer_read();
            break;
        case KMT_SES:
            memcpy(ses_bant, &data[2], SES_BANT);
            ses_seviye = data[18];
            ses_ton    = data[19];
            ses_bas    = data[20];
            ses_zaman  = timer_read();
            break;
        case KMT_EKRAN:
            memcpy(ekran_renk, &data[2], sizeof(ekran_renk));
            ekran_zaman = timer_read();
            break;
        case KMT_GOSTERGE:
            if (data[2] == 1) {
                gosterge_bit = data[3] & 0x0F;
                gosterge_r = data[4]; gosterge_g = data[5]; gosterge_b = data[6];
                durum = gosterge_yaz() ? 0 : 1;
            }
            memset(data, 0, length);
            data[0] = komut; data[1] = HID_IMZA; data[2] = durum; data[3] = PROTOKOL_SUR;
            data[4] = gosterge_bit; data[5] = gosterge_r; data[6] = gosterge_g; data[7] = gosterge_b;
            raw_hid_send(data, length);
            return;
        case KMT_EKRAN_KENAR:
            memcpy(ekran_kenar, &data[2], sizeof(ekran_kenar));
            ekran_kenar_zaman = timer_read();
            break;
        case KMT_BOYA_YAZ: {
            uint8_t p = data[2], bas = data[3], adet = data[4];
            if (p >= PROFIL_SAYISI || (bas & 7) || adet > 8 || bas + adet > BOYA_LED) { durum = 1; break; }
            uint32_t adr = BOYA_ADRES(p);
            if (bas == 0)
                for (uint8_t k = 0; k < 8; k++) FLASH_EraseSector(adr + k * 64);
            for (uint8_t w = 0; w < (adet * 3 + 3) / 4; w++) {
                const uint8_t *q = &data[8 + w * 4];
                FLASH_ProgramDWord(adr + bas * 3 + w * 4, q[0] | (q[1] << 8) | (q[2] << 16) | ((uint32_t)q[3] << 24));
            }
            if (data[5]) FLASH_ProgramDWord(adr + 508, BOYA_IMZA);
            break;
        }
        case KMT_BOYA_OKU: {
            uint8_t p = data[2], bas = data[3];
            if (p >= PROFIL_SAYISI || bas >= BOYA_LED || *(const uint32_t *)(BOYA_ADRES(p) + 508) != BOYA_IMZA) { durum = 2; break; }
            memset(data, 0, length);
            data[0] = komut; data[1] = HID_IMZA; data[3] = PROTOKOL_SUR;
            uint8_t adet = BOYA_LED - bas < 8 ? BOYA_LED - bas : 8;
            memcpy(&data[4], (const uint8_t *)BOYA_ADRES(p) + bas * 3, adet * 3);
            raw_hid_send(data, length);
            return;
        }
        case KMT_SISTEM:
            sistem_sol = data[2]; sistem_sag = data[3];
            sistem_zaman = timer_read();
            break;
        case KMT_SAYAC: {
            uint8_t bas = data[2], sifirla = data[3] == 1;
            memset(data, 0, length);
            data[0] = komut; data[1] = HID_IMZA; data[3] = PROTOKOL_SUR;
            for (uint8_t n = 0; n < 14 && bas + n < SAYAC_LED; n++) {
                uint16_t v = tus_sayac[bas + n];
                data[4 + n * 2] = v; data[5 + n * 2] = v >> 8;
                if (sifirla) tus_sayac[bas + n] = 0;
            }
            raw_hid_send(data, length);
            return;
        }
        case KMT_ACILIS:
            if (data[2] == 1 || data[2] == 2) {
                if (data[3] >= ACILIS_STIL_SAYISI) { durum = 1; }
                else if (data[2] == 1) { durum = acilis_yaz(data[3]) ? 0 : 1; }
                else if (data[3] != ACILIS_KAPALI) { acilis_stil = data[3]; acilis_baslat(); }
            }
            memset(data, 0, length);
            data[0] = komut; data[1] = HID_IMZA; data[2] = durum; data[3] = PROTOKOL_SUR;
            data[4] = acilis_kayitli;
            raw_hid_send(data, length);
            return;
        case KMT_TUS_OKU: {
            uint8_t bas = data[2];
            bool    var = *(const uint32_t *)TUS_ADRES == TUS_IMZA;
            memset(data, 0, length);
            data[0] = komut; data[1] = HID_IMZA; data[3] = PROTOKOL_SUR;
            for (uint8_t n = 0; n < 7 && bas + n < TUS_SAYI; n++) {
                uint8_t  l = bas + n, sa, su;
                uint16_t v = 0xFFFF, a = 0xFFFF;
                if (tus_konum(l, &sa, &su)) {
                    uint16_t k = pgm_read_word(&keymaps[0][sa][su]);
                    if (sa < 6 && k <= 0xFF) v = k;
                }
                if (var) a = ((const uint16_t *)(TUS_ADRES + 4))[l];
                data[4 + n * 4] = v; data[5 + n * 4] = v >> 8;
                data[6 + n * 4] = a; data[7 + n * 4] = a >> 8;
            }
            raw_hid_send(data, length);
            return;
        }
        case KMT_TUS_EK_OKU: {
            uint8_t bas = data[2];
            memset(&data[2], 0, length - 2);
            data[3] = PROTOKOL_SUR;
            for (uint8_t n = 0; n < 14 && bas + n < TUS_SAYI; n++) {
                uint16_t e = tus_ek(bas + n);
                data[4 + n * 2] = e; data[5 + n * 2] = e >> 8;
            }
            raw_hid_send(data, length);
            return;
        }
        case KMT_TUS_EK_YAZ: {
            uint8_t bas = data[2], adet = data[3];
            if ((bas & 1) || adet > 12 || bas + adet > TUS_SAYI) { durum = 1; break; }
            if (bas == 0)
                for (uint8_t k = 0; k < 4; k++) FLASH_EraseSector(EK_ADRES + k * 64);
            for (uint8_t w = 0; w < (adet + 1) / 2; w++) {
                const uint8_t *q = &data[6 + w * 4];
                uint32_t       v = q[0] | (q[1] << 8) | (2 * w + 1 < adet ? (q[2] << 16) | ((uint32_t)q[3] << 24) : 0xFFFF0000u);
                FLASH_ProgramDWord(EK_ADRES + 4 + bas * 2 + w * 4, v);
            }
            if (data[4]) FLASH_ProgramDWord(EK_ADRES, EK_IMZA);
            break;
        }
        case KMT_TUS_YAZ: {
            uint8_t bas = data[2], adet = data[3];
            if ((bas & 1) || adet > 12 || bas + adet > TUS_SAYI) { durum = 1; break; }
            if (bas == 0)
                for (uint8_t k = 0; k < 4; k++) FLASH_EraseSector(TUS_ADRES + k * 64);
            for (uint8_t w = 0; w < (adet + 1) / 2; w++) {
                const uint8_t *q = &data[6 + w * 4];
                uint32_t       v = q[0] | (q[1] << 8) | (2 * w + 1 < adet ? (q[2] << 16) | ((uint32_t)q[3] << 24) : 0xFFFF0000u);
                FLASH_ProgramDWord(TUS_ADRES + 4 + bas * 2 + w * 4, v);
            }
            if (data[4]) FLASH_ProgramDWord(TUS_ADRES, TUS_IMZA);
            break;
        }
        case KMT_UYARI:
            if (data[5] != 0xFF) { // 0xFF = sadece mikrofon durumu
                uyari_r = data[2]; uyari_g = data[3]; uyari_b = data[4];
                uyari_adet  = data[5] > 20 ? 20 : data[5];
                uyari_zaman = timer_read();
                uyari_tuslar = data[7] == 1; // sürüm 16
            }
            if (data[6] != 0xFF) { mikrofon_kapali = data[6]; mikrofon_zaman = timer_read(); }
            break;
        case KMT_MAKRO_YAZ: {
            uint8_t p = data[2], n = data[3], bas = data[4], adet = data[5];
            if (p >= PROFIL_SAYISI || n >= 5 || adet > 12 || bas + adet > MAKRO_OLAY) { durum = 1; break; }
            makro_durum = MAKRO_BOSTA; // klavyedeki kayıtla çakışmasın
            for (uint8_t i = 0; i < adet; i++) makro_tampon[1 + bas + i] = data[8 + i * 2] | (data[9 + i * 2] << 8);
            if (data[6]) { makro_sayi = bas + adet; makro_yaz(p, n); }
            break;
        }
        case KMT_PROFIL_YAZ: {
            uint8_t     p = data[2];
            isik_ayar_t yeni;
            if (p >= PROFIL_SAYISI) { durum = 1; break; }
            memcpy(&yeni, &data[3], sizeof(yeni));
            ayar_sinirla(&yeni);
            durum = kayit_yaz(p, &yeni) ? 0 : 1;
            if (!durum && p == profil) isik_ayar = yeni;
            break;
        }
        case KMT_PROFIL_OKU: {
            isik_ayar_t k;
            if (data[2] >= PROFIL_SAYISI || !kayit_oku(data[2], &k)) { durum = 2; break; }
            memset(data, 0, length);
            data[0] = komut; data[1] = HID_IMZA; data[3] = PROTOKOL_SUR;
            memcpy(&data[4], &k, sizeof(k));
            raw_hid_send(data, length);
            return;
        }
        case KMT_GUNCELLE:
            if (data[2] == 'B' && data[3] == 'O' && data[4] == 'O' && data[5] == 'T') {
                memset(data, 0, length);
                data[0] = komut; data[1] = HID_IMZA; data[3] = PROTOKOL_SUR;
                raw_hid_send(data, length);
                wait_ms(50);
                reset_keyboard(); // Fn+Esc ile aynı: ROM yükleyiciye geçer
            }
            durum = 1;
            break;
        case KMT_PROFIL:
            if (data[2] < PROFIL_SAYISI) profil_sec(data[2]);
            else durum = 1;
            break;
        default:
            durum = 0xFF;
            break;
    }

    memset(data, 0, length);
    data[0] = komut;
    data[1] = HID_IMZA;
    data[2] = durum;
    data[3] = PROTOKOL_SUR;
    memcpy(&data[4], &isik_ayar, sizeof(isik_ayar));
    raw_hid_send(data, length);
}

/* Fn tuşlarıyla geçici ayar değişikliği */
// Ayara göre basışı ve/veya bırakışı vuruş olarak kaydet (en eskisinin üzerine yazar)
static void vurus_kaydet(keyrecord_t *record) {
    bool basildi = record->event.pressed;
    if (isik_ayar.tetik == TETIK_BASINCA && !basildi) return;
    if (isik_ayar.tetik == TETIK_BIRAKINCA && basildi) return;
    uint8_t led[LED_HITS_TO_REMEMBER];
    uint8_t n = rgb_matrix_map_row_column_to_led(record->event.key.row, record->event.key.col, led);
    if (n == 0) return;
    uint8_t  yer = 0;
    uint16_t eski = 0;
    for (uint8_t j = 0; j < VURUS_SAYISI; j++) {
        if (!vuruslar.dolu[j]) { yer = j; break; }
        uint16_t g = timer_elapsed(vuruslar.zaman[j]);
        if (g >= eski) { eski = g; yer = j; }
    }
    vuruslar.x[yer]     = g_led_config.point[led[0]].x;
    vuruslar.y[yer]     = g_led_config.point[led[0]].y;
    vuruslar.zaman[yer] = timer_read();
    vuruslar.dolu[yer]  = 1;
    vuruslar.islendi[yer] = 0;
}

/* ---------------- Makrolar (Rec + G1..G5) ----------------
 * Kayıt: Rec'e bas (Rec yanıp söner) -> bir G tuşuna bas -> yazacaklarını yaz -> Rec'e bas (kaydedilir).
 * Hiçbir şey yazmadan Rec'e basarsan o G tuşunun makrosu silinir, G tuşu hiçbir şey yapmaz.
 * Oynatma: makrosu olan G tuşu makroyu yazar, olmayan varsayılan kısayolunu (keymap) gönderir.
 * Her G tuşu için flash'ta 128 bayt (2 sayfa): kelime0 = 0xA500 | olay sayısı, ardından en çok 63 olay.
 * Olay = tuş kodu (15 bit) | 0x8000 (basıldı). Yaklaşık 31 tuş vuruşu. */
#define MAKRO_YUVA 5
#define MAKRO_OLAY 63

uint8_t         makro_durum = MAKRO_BOSTA;
uint8_t         makro_yuva  = 0;
static uint16_t makro_tampon[MAKRO_OLAY + 1] __attribute__((aligned(4)));
static uint8_t  makro_sayi;

static uint8_t makro_uzunluk_p(uint8_t p, uint8_t n) {
    uint16_t bas = *(const uint16_t *)MAKRO_ADRES_P(p, n);
    if ((bas & 0xFF00) != 0xA500 || (bas & 0xFF) > MAKRO_OLAY) return 0;
    return bas & 0xFF;
}

uint8_t makro_uzunluk(uint8_t n) {
    uint16_t bas = *(const uint16_t *)MAKRO_ADRES(n);
    if ((bas & 0xFF00) != 0xA500 || (bas & 0xFF) > MAKRO_OLAY) return 0;
    return bas & 0xFF;
}

// Tampondaki makroyu profil p'nin G(n+1) yuvasına yaz (0 olay = makroyu sil)
static void makro_yaz(uint8_t p, uint8_t n) {
    makro_tampon[0] = 0xA500 | makro_sayi;
    for (uint8_t i = makro_sayi + 1; i <= MAKRO_OLAY; i++) makro_tampon[i] = 0;
    uint32_t adr = MAKRO_ADRES_P(p, n);
    FLASH_EraseSector(adr);
    FLASH_EraseSector(adr + 64);
    const uint32_t *w = (const uint32_t *)makro_tampon;
    for (uint8_t i = 0; i < (MAKRO_OLAY + 1) / 2; i++) FLASH_ProgramDWord(adr + i * 4, w[i]);
}

static void makro_kaydet(uint8_t n) { makro_yaz(profil, n); }

static void makro_oynat(uint8_t n) {
    uint8_t         say = makro_uzunluk(n);
    const uint16_t *o   = (const uint16_t *)MAKRO_ADRES(n) + 1;
    for (uint8_t i = 0; i < say; i++) {
        uint16_t kod = o[i] & 0x7FFF;
        if ((o[i] & 0xF000) == 0x7000) { // uygulamada eklenen bekleme: alt 12 bit x 10 ms
            wait_ms((uint32_t)(o[i] & 0x0FFF) * 10);
            continue;
        }
        if (o[i] & 0x8000) register_code16(kod);
        else unregister_code16(kod);
        wait_ms(8);
    }
    clear_keyboard(); // basılı kalan bir şey olmasın
}

// true dönerse tuş burada işlendi (keymap'e gitmesin)
static bool makro_isle(uint16_t keycode, keyrecord_t *record) {
    bool    basildi = record->event.pressed;
    uint8_t satir = record->event.key.row, sutun = record->event.key.col;
    bool    g_tusu = satir == 6 && sutun < MAKRO_YUVA;

    if (keycode == MAKRO_REC) {
        if (!basildi) return true;
        if (makro_durum == MAKRO_BOSTA) {
            makro_durum = MAKRO_SECIM;
        } else if (makro_durum == MAKRO_SECIM) {
            makro_durum = MAKRO_BOSTA; // vazgeçildi
        } else {
            makro_kaydet(makro_yuva);
            makro_durum = MAKRO_BOSTA;
        }
        return true;
    }
    if (makro_durum == MAKRO_SECIM) {
        if (g_tusu && basildi) {
            makro_yuva  = sutun;
            makro_sayi  = 0;
            makro_durum = MAKRO_KAYIT;
            clear_keyboard(); // kayıttan önce basılı kalan tuş takılı kalmasın
        }
        return g_tusu; // seçim beklerken G tuşu kendi işini yapmasın
    }
    if (makro_durum == MAKRO_KAYIT) {
        if (g_tusu) return true; // kayıt sırasında G tuşları yok sayılır
        // Sadece normal tuşlar (değiştiricili olanlar dahil) kaydedilir; Fn, katman vb. değil
        if (keycode <= QK_MODS_MAX) {
            if (makro_sayi < MAKRO_OLAY) makro_tampon[1 + makro_sayi++] = (keycode & 0x7FFF) | (basildi ? 0x8000 : 0);
            return true; // kayıt sırasında tuş bilgisayara gitmesin (ör. Discord kısayolu tetiklenmesin)
        }
        return false; // Fn vb. katman tuşları normal çalışsın
    }
    if (g_tusu && makro_uzunluk(sutun) > 0) {
        if (basildi) makro_oynat(sutun);
        return true;
    }
    return false;
}

bool process_record_user(uint16_t keycode, keyrecord_t *record) {
    vurus_kaydet(record);
    if (record->event.pressed) {
        tus_sayaci++;
        if (record->event.key.row < MATRIX_ROWS && record->event.key.col < MATRIX_COLS) {
            uint8_t l = g_led_config.matrix_co[record->event.key.row][record->event.key.col];
            if (l < SAYAC_LED && tus_sayac[l] < 0xFFFF) tus_sayac[l]++;
        }
        hiz_ilerle();
        if (hiz_kova[hiz_son & 7] < 255) hiz_kova[hiz_son & 7]++;
    }
    if (keycode == TUS_COKLU) { tus_coklu(record); return false; }
    if (makro_isle(keycode, record)) return false;
    if (keycode >= PROFIL_1 && keycode <= PROFIL_4) {
        if (record->event.pressed) profil_sec(keycode - PROFIL_1);
        return false;
    }
    if (keycode == PROFIL_GERI) {
        if (record->event.pressed) profil_geri();
        return false;
    }
    if (keycode == TEKER_MOD) {
        if (record->event.pressed) teker_isik = !teker_isik;
        return false;
    }
    if (keycode < ISIK_AC || keycode > YARI_AR) return true;
    if (!record->event.pressed) return false;
    isik_ayar_t *a = &isik_ayar;
    switch (keycode) {
        case ISIK_AC:
            if (a->mod == MOD_KAPALI) {
                a->mod = onceki_mod;
            } else {
                onceki_mod = a->mod;
                a->mod     = MOD_KAPALI;
            }
            break;
        case MOD_ONC: a->mod = (a->mod + MOD_SAYISI - 1) % MOD_SAYISI; break;
        case MOD_SON: a->mod = (a->mod + 1) % MOD_SAYISI; break;
        case SURE_AZ: a->sure = a->sure > 200 ? a->sure - 100 : 100; break;
        case SURE_CO: a->sure += 100; break;
        case PARL_AR: a->genel_parlak = a->genel_parlak < 90 ? a->genel_parlak + 10 : 100; break;
        case PARL_AZ: a->genel_parlak = a->genel_parlak > 10 ? a->genel_parlak - 10 : 0; break;
        case YARI_AZ:
            if (a->mod == MOD_DALGA) a->kalinlik = a->kalinlik > 12 ? a->kalinlik - 4 : 8;
            else a->yaricap = a->yaricap > 12 ? a->yaricap - 4 : 8;
            break;
        case YARI_AR:
            if (a->mod == MOD_DALGA) a->kalinlik += 4;
            else a->yaricap += 4;
            break;
    }
    ayar_sinirla(a);
    if (a->mod != MOD_KAPALI) onceki_mod = a->mod;
    return false;
}

/* Açılışta: kayıtlı ayar varsa onu, yoksa varsayılanı yükle; efekt hep bizim efektimiz */
/* Win kilidi ışığı (B12, LOW = yanık) */

void housekeeping_task_user(void) {
    writePin(B12, !keymap_config.no_gui);
    // Bilgisayar açılınca ya da uykudan dönünce (USB askıdan çıkıp yeniden etkinleşince) açılış animasyonu
    static bool askida = false;
    if (!acilis_aktif) acilis_stil = acilis_kayitli;
    if (USB_DRIVER.state == USB_SUSPENDED) askida = true;
    else if (askida && USB_DRIVER.state == USB_ACTIVE) { askida = false; acilis_baslat(); }
}

/* Fn katmanında işlevi olan tuşların LED'leri (keymap sabit olduğundan bir kez hesaplanır) */
static void fn_haritasi(void) {
    memset(fn_isik, 0, sizeof(fn_isik));
    for (uint8_t r = 0; r < MATRIX_ROWS; r++)
        for (uint8_t c = 0; c < MATRIX_COLS; c++) {
            uint8_t l = g_led_config.matrix_co[r][c];
            if (l == NO_LED || l >= DRIVER_LED_TOTAL) continue;
            uint16_t k = keymaps[_FN][r][c];
            if ((k != KC_TRNS && k != KC_NO) || keymaps[_TEMEL][r][c] == MO(_FN)) fn_isik[l >> 3] |= 1 << (l & 7);
        }
}

void keyboard_post_init_user(void) {
    gosterge_oku();
    acilis_oku();
    fn_haritasi();
    setPinOutput(B12);
    writePinHigh(B12);
    profil = profil_oku();
    if (!kayit_oku(profil, &isik_ayar)) isik_ayar = VARSAYILAN;
    if (isik_ayar.mod != MOD_KAPALI) onceki_mod = isik_ayar.mod;
    rgb_matrix_enable_noeeprom();
    rgb_matrix_mode_noeeprom(RGB_MATRIX_CUSTOM_SIMSEK);
    rgb_matrix_sethsv_noeeprom(0, 0, 255);
    rgb_matrix_set_speed_noeeprom(128);
    acilis_baslat();
}
