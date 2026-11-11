# Pusat K3 Pro – Donanım haritası (orijinal firmware'den çıkarıldı)

Kaynak: `yedek/firmware/pusat_k3pro_fw_0C45-5004_v1.00_2021-12-28.bin` (Monster `Upgrade Package.exe` içinden,
checksum 74CC). Klavye açılmadı; her şey firmware kodunun çözümlenmesinden (Cortex-M0 Thumb) çıkarıldı.
Tarih: 2026-11-09.

## Çip

- **SN32F24xB** (firmware kaynağında `SN32F24xB`; SonixQMK'de SN32F248B ailesi). Cortex-M0, 64 KB flash.
- Kaynak kodda kullanılan çevre birimleri: GPIO0–3 (A–D), CT16B1 (LED PWM), USB, FLASH.
- Vektör tablosu 0x0000'da, reset 0x8894. USB tanımlayıcısı: VID 0C45, PID 5004, bcdDevice 1.00,
  "SONiX" / "PUSAT K3 PRO MECHANICAL GAMING KEYBOARD".

## Tuş matrisi – 7 satır × 20 sütun, ROW2COL

Firmware sütunları sırayla LOW sürer, satırları (pull-up) okur → QMK'de `diode_direction: ROW2COL`
(Redragon K556 portuyla aynı yöntem).

- **Sütunlar (çıkış, 20 adet, sırayla):** A0 A1 A2 A3 A4 A5 A6 A7 A8 A9 A10 A11 A12 A13 A14 B0 B1 B2 B3 B4
  (kanıt: fonksiyon 0x0F10, 20 girişli switch tablosu 0x0F5A)
- **Satırlar (giriş, pull-up, 7 adet, bit sırası):** C15 D11 D10 D9 D8 D7 B5
  (kanıt: tarama döngüsü 0x2A86–0x2AF8, RAM 0x20000126[sütun] = 7 bitlik satır maskesi)
- Ayarlar hafızasındaki tuş tablosu indeksi = `sütun*7 + satır` (profil 1: config 0x080'den 3'er bayt).

### Matris (satır × sütun). `*` = altında LED var

```
       0     1     2     3     4     5     6     7     8     9     10    11    12    13    14    15    16    17    18    19
r0   Esc*  F1*   F2*   F3*   F4*   F5*   F6*   F7*   F8*   F9*   F10*  F11*  F12*  Next  PrSc* ScLk* Paus* (?1)  Prev  Play
r1   "*    1*    2*    3*    4*    5*    6*    7*    8*    9*    0*    **    -*    Bksp* Ins*  Home* PgUp* NumL* N/*   N**
r2   Tab*  Q*    W*    E*    R*    T*    Y*    U*    I*    O*    P*    Ğ*    Ü*    ,*    Del*  End*  PgDn* N7*   N8*   N9*
r3   Caps* A*    S*    D*    F*    G*    H*    J*    K*    L*    Ş*    İ*    (#)*  Ent*  -     -     -     N4*   N5*   N6*
r4   LSft* <*    Z*    X*    C*    V*    B*    N*    M*    Ö*    Ç*    .*    (k87)* RSft* -    Up*   -     N1*   N2*   N3*
r5   LCtl* LWin* LAlt* Spc*  AltGr* Fn*  App*  N-*   RCtl* NEnt* N+*   -     -     -     Left* Down* Rght* -     N0*   N.*
r6   G1    G2    G3    G4    G5    Rec   -     -     -     -     -     -     -     -     -     M1    M2    M3    M4    MR
```

- Türkçe Q isimleri Windows Türkçe düzenine göre (HID kodları: `"`=0x35, `*`=0x2D, `-`=0x2E, Ğ=0x2F, Ü=0x30,
  `,`(r2c13)=0x31, Ş=0x33, İ=0x34, Ö=0x36, Ç=0x37, `.`=0x38, `<`=0x64).
- Ham kodlar: r0c13 `03 B5 00` (sonraki parça), r0c18 `03 B6 00` (önceki), r0c19 `03 CD 00` (oynat/duraklat),
  r0c17 `05 18 18` (bilinmiyor – muhtemelen sessiz/kilit), r5c5 `05 02 02` (Fn), r6c5 `05 19 00` (Rec),
  r6c15–18 `05 0F 00..03` (M1–M4), r6c19 `05 25 00` (MR), r3c12 HID 0x32 ve r4c12 HID 0x87: düzene göre
  aktif olmayan ISO/JIS konumları.
- G tuşları: `05 1A 0n` = klavye üstü makro yuvası n (Rec ile kaydedilen), `0D A2 0n` = uygulama makrosu.

## RGB – 7 LED satırı × 3 renk, 20 PWM sütunu

- **Sütunlar = PWM:** CT16B1 PWM0–PWM14 (A0–A14) ve PWM16–PWM20 (B0–B4). PWM15 kapalı (A15 LED satırı).
  PWMENB = PWMIOENB = 0x1F7FFF, MR24 (periyot) = 0xFF.
- **LED satır pinleri** (kesme 0x84DA, adım sayacı RAM 0x20000024, 21 adım + 1 tarama adımı):

| LED satırı | Kırmızı | Yeşil | Mavi |
|---|---|---|---|
| 0 (F satırı) | C0 | C1 | C3 |
| 1 (sayılar) | C4 | C5 | C6 |
| 2 (Q satırı) | C7 | C8 | C9 |
| 3 (A satırı) | C10 | C11 | C12 |
| 4 (Z satırı) | B6 | B7 | B8 |
| 5 (Ctrl satırı) | B9 | B10 | B11 |
| 6 (G satırı) | C13 | A15 | C14 |

  Satır pini BSET ile (HIGH) açılıyor, init'te hepsi BCLR. Renk düzlemleri: RAM 0x200001D6 (R), 0x20000262 (B),
  0x200002EE (G), her biri 7×20 bayt. Renk eşlemesi özel mod kopyalama koduyla doğrulandı (0x5D1E: bayt0→R,
  bayt1→G, bayt2→B).
- Satır 6'da (G tuşları) LED kanalı var ama özel moddaki 107 LED listesinde (tablo 0x8FF9) yok. Profil göstergesi
  ışıkları R düzleminde 0x2000037A+135..138 (satır 6, sütun 15–18 = M1–M4 üstü).
- **LED sayısı:** 107 (tablo 0x8FF9). Protokoldeki özel renk indeksi = `satır*20 + sütun`.
  Daha önce elle kalibre edilen 13 nokta (7=F7, 22=2, 28=8, 31=*, 43=E, 49=O, 52=Ü, 64=F, 70=Ş, 73=Enter,
  86=B, 91=., 95=Yukarı) bu haritayla birebir tutuyor.
- K556 ile fark: K556 6 satır/21 sütun (B5 sütun); K3 Pro'da B5 7. tuş satırı, ayrıca 7. LED satırı (C13, A15, C14).

## Ses tekerleği (döner kodlayıcı)

- **D3 ve D4** giriş (init 0x286C). Okuma 0x4764: bit0 = D3, bit1 = D4, klasik dörtlü (quadrature) çözümleme,
  sonuç RAM 0x2000005A (+1 / −1). Hangi yönün ses artırdığı **doğrulanmadı**.

## Diğer

- B12–B15 init'te HIGH yapılıyor (görevi bilinmiyor).
- 0x0F10: tüm sütunları LOW yapıp satırlara bakan "herhangi bir tuş basılı mı" kontrolü (uyku/uyandırma).
- QMK için ilk taslak değerler:
  - `matrix_pins.cols`: A0…A14, B0…B4; `matrix_pins.rows`: C15, D11, D10, D9, D8, D7, B5; ROW2COL
  - `SN32F2XX_RGB_MATRIX_ROW_PINS`: { C0, C1, C3, C4, C5, C6, C7, C8, C9, C10, C11, C12, B6, B7, B8, B9, B10, B11, C13, A15, C14 }
    (satır başına R, G, B sırası – SonixQMK sürücüsünün beklediği sıra yüklemeden önce kontrol edilecek)
  - Encoder: A = D3, B = D4
  - Aktif seviyeler: stok firmware her adımda tüm LED satır pinlerini LOW yapıp seçili satırı HIGH yapıyor;
    MR değeri = renk baytı (0–255, periyot 255). K556 QMK portu "PWM active high / RGB row active low" kullanıyor –
    bizim kartta satır **HIGH ile açılıyor** gibi görünüyor; QMK ayarı yüklemeden önce bu farka göre seçilecek.
