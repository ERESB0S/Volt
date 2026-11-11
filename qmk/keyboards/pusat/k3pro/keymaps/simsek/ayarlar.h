// Copyright 2026 Eresbos (github.com/ERESB0S)
// SPDX-License-Identifier: GPL-2.0-or-later
/* Işık ayarları: hem keymap.c hem rgb_matrix_user.inc kullanır.
 * Bilgisayardaki "Pusat Işık" uygulaması bu ayarları raw HID üzerinden okur/yazar.
 */
#pragma once
#include <stdint.h>
#include <stdbool.h>

enum isik_modlari {
    MOD_SIMSEK      = 0,  // kısık taban + basılan tuşun çevresi parlar
    MOD_DALGA       = 1,  // taban + basılan tuştan kalın bir dalga tüm klavyeye yayılır
    MOD_SABIT       = 2,  // sadece taban renk
    MOD_KAPALI      = 3,  // ışıklar kapalı
    // Orijinal klavyedeki animasyonların karşılıkları
    MOD_NEFES       = 4,  // nefes
    MOD_SPEKTRUM    = 5,  // spektrum: tüm klavye renk değiştirir
    MOD_RENK_CARKI  = 6,  // renk çarkı: merkez etrafında dönen renkler
    MOD_GOKKUSAGI   = 7,  // dalga: yatay akan gökkuşağı
    MOD_DIKEY       = 8,  // dikey gökkuşağı
    MOD_DAIRESEL    = 9,  // dairesel gökkuşağı
    MOD_KASIRGA     = 10, // kasırga (spiral)
    MOD_TEPKILI     = 11, // tepkili: basılan tuş yanar
    MOD_TEPKI_CIZGI = 12, // tepkili çizgi: basılan tuşun satırı ve sütunu
    MOD_DALGALANMA  = 13, // dalgalanma: ince halka
    MOD_YILDIZ      = 14, // yıldızlar
    MOD_BIRIKME     = 15, // birikme: bastıkça yanan, yavaşça sönen tuşlar
    MOD_VISOR       = 16, // visor: sağa sola gidip gelen ışık bandı
    MOD_YUKSELME    = 17, // surmount: aşağıdan yukarı yükselen ışık
    MOD_SES         = 18, // bilgisayarda çalan sese tepki (veriyi Volt gönderir); stil = titreme alanı (SES_*)
    MOD_EKRAN       = 19, // ekran rengi (ambilight): Volt ekranı 8 dikey bölgeye ayırıp renklerini gönderir
    MOD_BOYAMA      = 20, // tuş tuş boyama: her LED'in rengi flash'ta (BOYA_ADRES)
    MOD_HIZ         = 21, // yazma hızı: yavaşta taban renk, hızlandıkça parlama rengi
    MOD_SAYISI
};

typedef struct __attribute__((packed)) {
    uint8_t  mod;
    uint8_t  taban_r, taban_g, taban_b; // taban renk
    uint8_t  parla_r, parla_g, parla_b; // parlama rengi
    uint8_t  taban_parlak;              // taban parlaklığı, % (0..100)
    uint8_t  genel_parlak;              // tüm ışıkların parlaklığı, % (0..100)
    uint8_t  yaricap;                   // parlamanın yayıldığı mesafe (8..120; ~11 = 1 tuş)
    uint16_t sure;                      // parlamanın sönme süresi, ms (100..3000)
    uint8_t  titreme;                   // 1 = ilk anda şimşek titremesi; MOD_SES'te ses stili (SES_*)
    uint8_t  renk_sirasi;               // donanım kanal sırası düzeltmesi (0..5), bkz. rgb_matrix_user.inc
    uint8_t  kalinlik;                  // dalga kalınlığı (8..120; ~11 = 1 tuş)
    uint8_t  iz;                        // dalganın arkasında kalan iz uzunluğu, % (0..100)
    uint8_t  tetik;                     // TETIK_BASINCA / TETIK_BIRAKINCA / TETIK_IKISI
    uint8_t  kenar_mod;                 // KENAR_* : yan ışıkların animasyonu
    uint8_t  hiz;                       // animasyon hızı (0..255)
    uint8_t  renkli;                    // 1 = tepkili/nefes vb. modlarda tek renk yerine gökkuşağı
    // Yan ışıklar: tuşlardan bağımsız renk, parlaklık ve hız
    uint8_t  kenar_a_r, kenar_a_g, kenar_a_b; // ana renk
    uint8_t  kenar_b_r, kenar_b_g, kenar_b_b; // ikinci renk (çakma, kuyruk, geçiş)
    uint8_t  kenar_parlak;              // % (0..100)
    uint8_t  kenar_hiz;                 // 0..255
} isik_ayar_t; // 28 bayt

enum { TETIK_BASINCA = 0, TETIK_BIRAKINCA = 1, TETIK_IKISI = 2 };
enum {
    KENAR_TUSLAR = 0, KENAR_NEFES = 1, KENAR_GOKKUSAGI = 2, KENAR_SIMSEK = 3, KENAR_KAPALI = 4, KENAR_SABIT = 5, KENAR_SPEKTRUM = 6,
    KENAR_KOMET = 7,   // çevrede dolaşan kuyruklu ışık
    KENAR_AKIS = 8,    // iki renk arasında akan geçiş
    KENAR_KALP = 9,    // kalp atışı
    KENAR_MUM = 10,    // mum gibi titreme
    KENAR_TARAMA = 11, // soldan sağa gidip gelen ışık
    KENAR_YANSIMA = 12, // basılan tuşa en yakın kenar yanar
    KENAR_SES = 13,     // ses seviyesi göstergesi: sol bas, sağ tiz
    KENAR_HIZ = 14,     // yazma hızı göstergesi: hızlandıkça alttan yukarı dolar
    KENAR_EKRAN = 15,   // ekran rengi: sol şerit ekranın sol kenarı, sağ şerit sağ kenarı (yukarıdan aşağı)
    KENAR_SISTEM = 16,  // sistem göstergesi: Volt'un gönderdiği iki ölçüm (ör. CPU, RAM) alttan yukarı dolar
    KENAR_SAYISI
};
#define LED_KENAR_ILK 122 // 122..139 (config_led.c)

extern isik_ayar_t isik_ayar;

/* Ses modu: Volt saniyede ~60 kez frekans bantlarını gönderir. Sadece RAM'de tutulur (flash'a yazılmaz). */
enum { SES_SPEKTRUM = 0, SES_NABIZ = 1, SES_DALGA = 2, SES_RENK = 3, SES_STIL_SAYISI };
#define SES_BANT 16
extern uint8_t  ses_bant[SES_BANT]; // 0 = en bas .. 15 = en tiz, 0..255
extern uint8_t  ses_seviye;         // genel ses seviyesi 0..255
extern uint8_t  ses_ton;            // baskın frekansa göre renk tonu 0..255
extern uint8_t  ses_bas;            // bas vuruşu 0..255
extern uint16_t ses_zaman;          // son paketin geldiği an (timer_read)

/* Ekran rengi: 8 bölge x RGB, soldan sağa. Sadece RAM. */
#define EKRAN_BOLGE 8
extern uint8_t  ekran_renk[EKRAN_BOLGE * 3];
#define EKRAN_KENAR 4 // her yanda yukarıdan aşağı 4 renk
extern uint8_t  ekran_kenar[2 * EKRAN_KENAR * 3]; // önce sol, sonra sağ
extern uint16_t ekran_kenar_zaman;
extern uint16_t ekran_zaman;

/* Tuş boyama: profil başına 512 bayt (8 sayfa); LED i'nin rengi +i*3, imza +508 */
#define BOYA_LED 140
#define BOYA_ADRES(p) (0xE800u + (uint32_t)(p) * 512u)
#define BOYA_IMZA 0x41594F42u // "BOYA"

/* Bildirim ışığı: kenar ışıkları adet kez yanıp söner; mikrofon kapalıyken üstteki kenar ışıkları kırmızı */
extern uint8_t  uyari_r, uyari_g, uyari_b, uyari_adet;
extern bool     uyari_tuslar; // bildirim yanıp sönmesi tuşlarda da olsun
extern uint16_t uyari_zaman;
extern bool     mikrofon_kapali;
extern uint16_t mikrofon_zaman;

/* Göstergeler (tüm profiller için ortak, flash'ta GOSTERGE_ADRES):
 * kilit açıkken kendi tuşu, Fn basılıyken sadece Fn ile işlevi olan tuşlar gösterge renginde yanar */
enum { GOSTERGE_CAPS = 1, GOSTERGE_NUM = 2, GOSTERGE_SCROLL = 4, GOSTERGE_FN = 8 };
extern uint8_t gosterge_bit, gosterge_r, gosterge_g, gosterge_b;
extern uint8_t fn_isik[]; // LED başına 1 bit: Fn katmanında işlevi var
#define LED_CAPS 56
#define LED_NUM 33
#define LED_SCROLL 14
#define LED_WIN    92 // Fn + Win ile Windows tuşu kilitliyken gösterge renginde yanar

// Açılış animasyonu: klavye bilgisayara bağlanınca (ilk açılış, bilgisayar açılışı, uykudan dönüş) soldan sağa ışık dalgası
// Her LED kendi sırası gelince (t0) parlak renkte yanar, ACILIS_KUYRUK ms içinde asıl efekte döner
#define ACILIS_SURE   1250 // ms
#define ACILIS_KUYRUK 400  // ms
enum { ACILIS_KAPALI, ACILIS_DALGA, ACILIS_MERKEZ, ACILIS_YUKARI, ACILIS_GOKKUSAGI, ACILIS_KIVILCIM, ACILIS_PERDE, ACILIS_STIL_SAYISI };
// Sistem göstergesi (KENAR_SISTEM): sol ve sağ şeridin doluluğu 0..255, sadece RAM
extern uint8_t  sistem_sol, sistem_sag;
extern uint16_t sistem_zaman;
extern bool     acilis_aktif;
extern uint16_t acilis_zaman;
extern uint8_t  acilis_stil;
// Num Lock kapalıyken işlevi değişen sayı tuşları (7 8 9 4 5 6 1 2 3 0 .)
#define NUMPAD_SAYI(i) (((i) >= 53 && (i) <= 55) || ((i) >= 70 && (i) <= 72) || ((i) >= 88 && (i) <= 90) || (i) == 105 || (i) == 106)

/* Yazma hızı: son 2 sn'deki vuruş/sn x 10 (0..255) */
uint8_t yazma_hizi(void);

/* Kendi tuş vuruşu takibimiz (QMK'nınki hem basışta hem bırakışta kaydediyor) */
#define VURUS_SAYISI 16
typedef struct {
    uint8_t  x[VURUS_SAYISI], y[VURUS_SAYISI];
    uint16_t zaman[VURUS_SAYISI]; // timer_read() değeri
    uint8_t  dolu[VURUS_SAYISI];
    uint8_t  islendi[VURUS_SAYISI]; // birikme modu bu vuruşu saydı mı
} vurus_t;
extern vurus_t vuruslar;

/* Makro kaydı durumu (Rec tuşu); efekt Rec ve seçili G tuşunu yanıp söndürür */
enum { MAKRO_BOSTA = 0, MAKRO_SECIM = 1, MAKRO_KAYIT = 2 };
extern uint8_t makro_durum;
extern uint8_t makro_yuva;
#define LED_G1 111  // G1..G5 = 111..115 (config_led.c)
#define LED_REC 116
#define LED_M1 117  // M1..M4, MR = 117..121 (işlevleri yok, ışıkları kapalı)
uint8_t makro_uzunluk(uint8_t n);
#define LED_TEKER 108 // r0c17 geçiş tuşu: sadece ışık modunda yanar
extern bool teker_isik;

/* Profiller: M1..M4 (LED_M1..LED_M1+3) seçili profili gösterir, MR (LED_M1+4) geri alınca kısa yanar */
extern uint8_t  profil;
extern uint16_t mr_zaman;
extern bool     mr_goster;

/* Kenar ışığı tanıma: uygulama tek bir kenar LED'ini (0..17) beyaz yakar, diğerleri söner; 0xFF = kapalı */
extern uint8_t  kenar_test;
extern uint16_t kenar_test_zaman;
