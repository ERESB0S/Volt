# Pusat K3 Pro – USB Işık Protokolü

Kaynak: OpenRGB'nin "EVision Keyboard" sürücüsü (Sonix tabanlı klavyelerin ortak firmware'i).
Kod kopyalanmadı; sadece protokol bilgisi buraya not edildi.

## ✅ Doğrulanan (2026-11-09, `probe` programı)

- `0x10` (özel renkleri oku) **Begin/End olmadan** çalışıyor. Klavye, istek başlığını (checksum dahil) aynen geri gönderiyor; veri bayt 8'den başlıyor.
- Checksum hesabı doğru (klavye cevap verdi, başlıklar birebir aynı).
- Okunan kayıtlı profil = klasik FPS düzeni → LED numaraları OpenRGB matrisiyle **örtüşüyor**:

| LED | Tuş | Renk |
|---|---|---|
| 22–26 | 1 2 3 4 5 | sarı |
| 44 | W | kırmızı |
| 46 | R | yeşil |
| 64–66 | A S D | kırmızı |

### Yazma testleri (2026-11-09)

| Test | Klavye cevabı | Geri okuma | Işıkta görünen |
|---|---|---|---|
| `0x11` ile Esc = 00 00 7F (Begin/End yok) | aynen yankı | Esc = 00 00 7F ✅ | değişmedi |
| Begin + End | aynen yankı | Esc = 00 00 7F | değişmedi |
| Mod = 0x14 (1 bayt, Begin/End ile) | aynen yankı | – | değişmedi |
| Mod bloğu 8 bayt (14 04 03 00 00 000000, Begin/End yok) | aynen yankı | – | değişmedi |

**Sonuç:** FF1C kanalı her komutu yankılıyor ve renk hafızasını okuyup yazabiliyoruz, ama görünen ışık hiç değişmiyor.

### Resmi uygulama farklı bir kanal kullanıyor olabilir

- `HidServ.dll` → `HidD_SetFeature` / `HidD_GetFeature` kullanıyor (feature report).
- FF1C kanalının tanımlayıcısı: rapor ID 4, 63 bayt giriş + 63 bayt çıkış, **feature yok**.
- **Arayüz 0** (ana klavye) tanımlayıcısında: usage page `0x0C`, **64 baytlık feature report**, rapor ID yok
  (`05 0C 09 00 15 80 25 7F 75 08 95 40 B1 02`).
- Arayüz 0'dan doğrudan `GetFeature` → sadece `00` dönüyor (muhtemelen önce bir istek komutu gerekiyor).

~~→ Asıl ışık protokolü büyük ihtimalle arayüz 0 üzerinden feature report ile çalışıyor.~~ **Yanlış çıktı**, aşağıya bakın.

## ✅ Resmi uygulamanın trafiği (USBPcap kaydı, 2026-11-09)

Ham paketler: `yakalama_resmi_uygulama.txt`

- Resmi uygulama da **aynı kanalı** kullanıyor: arayüz 1, output report ID 4 (SET_REPORT, wValue 0x0204, wIndex 1). Feature report hiç kullanılmadı.
- Checksum = bayt 3..63'ün **işaretsiz** toplamı (bizimkiyle aynı).
- Ayar yazarken her zaman: **Begin (01) → komut(lar) → End (02)**.

### Klavye bir V1/V2 karışımı: ek komutlar

| Kod | Komut | Not |
|---|---|---|
| `0x03` | Bilgi bloğunu oku (0x2C bayt) | `55 AA FF 02 45 0C 04 50 00 01 <aktif profil> 38 …` + desteklenen mod listesi `01–0F, 11, 10, 12, 14` |
| `0x04` | Bilgi bloğunu yaz | Profil değiştirme: okunan blok, sadece bayt 10 değiştirilerek geri yazılıyor |
| `0x05` | Ayar bloğunu oku | ofset = bayt 5–6 |
| `0x06` | Ayar bloğuna yaz | ofset = bayt 5–6 (yani "parametre no" aslında **bayt ofseti**) |

### Ayar bloğu (profil başına, profil 1 = 0x00, profil 2 = 0x2A, profil 3 = 0x54)

| Ofset | Anlamı |
|---|---|
| +0 | mod (01–14, bkz. mod listesi; 06 = Normal açık) |
| +1 | parlaklık (0–4) |
| +2 | hız |
| +3 | yön |
| +4 | "Renkli" kutusu (rastgele/çok renkli bayrağı) |
| +5..+7 | mod rengi R, G, B |

### Kayıttaki işlemler

| Zaman | Uygulamada | Paket(ler) |
|---|---|---|
| 39.5 s | Renk = mavi, Renkli kapalı | `06 len1 off04 = 00`, `06 len3 off05 = 00 00 FF` |
| 50–53 s | Parlaklık çubuğu | `06 len1 off01 = 03,02,01,02,03,04` |
| 62.4 s | Renkli işaretlendi | `06 len1 off04 = 01` |
| 81.6 s | Renk = kırmızı | `06 off04 = 00`, `06 off05 = FF 00 00` |
| 96.3 s | Profil2'ye tıklama | `03` oku, `05` ile 3 profil oku, `11` ile özel renkler yaz, `04` ile aktif profil = 1 |

### Önceki testlerimiz neden görünmedi?

Muhtemelen: o sırada resmi uygulama arka planda açıktı ve kendi ayarlarını geri yazıyordu; ayrıca mod 0x14 (Sahil) bu klavyede özel renkleri göstermiyor olabilir. Henüz doğrulanmadı.

### Kendi programımızdan doğrulanan

- `write_test profile1` → aktif profil 2'den 1'e alındı (bilgi bloğu geri okundu: bayt 10 = 00).
- `write_test blue` → resmi uygulamanın paketiyle bayt bayt aynı (`04 0D 01 06 03 05 00 00 00 00 FF`), geri okuma: renk = 0000FF.
- **Kullanıcı onayladı: klavyedeki ışıklar maviye döndü.** 🎉 Kendi programımızdan ilk görünür değişiklik.

- `write_test rainbow` → 8 × `0x11` (Begin/End içinde) ile her satıra ayrı renk + aktif profilde mod = 0x14.
  **Kullanıcı onayladı: satırlar renkli göründü.** → Tuş başına renk çalışıyor, satır haritası (her satır 21 LED) doğru.

### Tuş başına renk tarifi

1. Begin
2. `0x11` × 8 paket (ofset 0, 0x36, … 0x17A), her biri 54 bayt RGB
3. End
4. Aktif profilin ayar bloğunda mod = `0x14` (Begin → `06 len1 off<profil> = 14` → End)

### Canlı renk

- ✅ Mod 0x14'teyken `0x11` paketleri Begin/End **olmadan** gönderilince ışık anında değişiyor.
  `anim_test` (kayan beyaz sütun, 80 ms/kare) → **kullanıcı onayladı: sütun kayıyor.**
- Hız: 8 paket/kare, her pakette cevap bekleniyor → kare başına ~104 ms, **~10 kare/sn**.
  İyileştirme fikirleri: sadece değişen paketleri göndermek, cevabı beklemeden sıradakini göndermek.
- Animasyonun son karesi: beyaz sütun 7. sütunda (LED 7, 28, 49, 70, 91, 112).
- ❌ Klavye çıkarılıp takıldı → **son kare (beyaz sütun, LED 7/28/49/70/91/112) kalıcı olarak duruyor.**
  Kullanıcı gördü: F7, 8, O, Ş, "." beyaz. Hafızadan okuma da aynı.
  → Begin/End olmadan gönderilen `0x11` paketleri de **kalıcı hafızaya yazılıyor.**
- ❌ `0x12` (V2 anlık renk): cevapta hata baytı (7) = `FF` → **komut desteklenmiyor**, hafıza değişmedi.
- Süre ölçümü (`write_test timing`, 10'ar paket ortalaması):

| Paket | Süre |
|---|---|
| `0x10` renk okuma | 3.0 ms |
| `0x05` ayar okuma | 3.0 ms |
| `0x11` renk yazma (aynı veri) | 8.5 ms |
| `0x11` animasyon sırasında | ~13 ms |

  Yazma, okumadan 5–10 ms daha uzun sürüyor; bu, her pakette flash'a yazma süresiyle uyumlu.
  **Sonuç: bu firmware'de RAM'e giden bir canlı renk yolu bulunamadı. Sürekli animasyon flash'ı yıpratır.**

### ❌ Şimşek demosu (`simsek`, 2026-11-09): yazmayı engelliyor

- Program çalışırken kullanıcı yazı yazamadı: **bazı tuş basışları klavye tarafından algılanmadı.** Kullanıcı Ctrl+C ile durdurdu.
- Açıklama: her `0x11` paketi flash'a yazılırken klavyenin işlemcisi (flash'tan kod çalıştıran Cortex-M0) birkaç ms duruyor
  ve o sırada tuşları taramıyor. Bu, "her paket flash'a yazılıyor" sonucunu bir kez daha doğruluyor.
- **Sonuç: tuşa basınca tepki veren her türlü bilgisayar tarafı efekt bu klavyede kullanılamaz** (hem flash yıpranması hem tuş kaçırma).
  Bu tür testler bir daha yapılmayacak.

### Flash yıpranması tahmini

- Test sırasında: ~880 kare × 8 paket. Paketlerin her biri bir flash sayfası silme+yazma ise, bu sayfalar ~880 kez yazıldı.
- Bu tür mikrodenetleyicilerde flash ömrü genelde 10.000–100.000 yazma döngüsü (bu çipin kesin değeri bilinmiyor).
  → Testler ömrün kabaca %1–9'unu kullanmış olabilir. Klavye şu an sorunsuz.
- 10 kare/sn sürekli animasyon: 10.000 döngüye ~17 dakikada, 100.000'e ~3 saatte ulaşır → **kullanılamaz.**

### Ayar hafızasının tamamı (`write_test dump`, 0x05 ile 0x000–0x1007 okunabiliyor)

Döküm: `yedek/ayarlar_once.bin` (G tuşu atamaları kaybolduktan sonra, kullanıcı yeniden ayarlamadan önce).

| Ofset | İçerik (tahmin) |
|---|---|
| 0x000 / 0x02A / 0x054 | Profil 1 / 2 / 3 ışık ayarları (mod, parlaklık, hız, yön, renkli, RGB…) |
| 0x080–0x22F | Profil 1 tuş atama tablosu: tuş başına 3 bayt `[tür, alt tür, kod]` |
| 0x280–0x42F | Profil 2 tuş atama tablosu |

Tuş atama örnekleri: `02 02 29` = normal tuş, HID kodu 0x29 (Esc); `02 02 35` = `"`/~;
`0D A2 01` … `0D A2 05` = **G1…G5 → makro 1…5**; `05 0F xx` / `03 B5 00` = medya/özel tuşlar.
Makroların içeriği (ör. CTRL + Num *) bu bölgede görünmüyor → başka bir hafıza alanında veya başka bir komutla tutuluyor.

### Güvenli kullanım kuralı (uygulamada uygulanacak)

- Kalıcı yazmalar (`0x06`, `0x11`, `0x04`) **seyrek** olmalı: kullanıcı ayar değiştirdiğinde, profil/uygulama değiştiğinde.
- Uygulamada bir **yazma bütçesi** olacak (ör. dakikada en fazla birkaç yazma, günlük sayaç); aşılırsa yazma ertelenir.
- Aynı veri tekrar yazılmayacak (önce oku/karşılaştır).
- Animasyonlar için klavyenin **kendi 18 dahili efekti** kullanılacak (bunlar flash'a yazmadan çalışıyor).
- V2'deki `0x12` (dinamik renk gönder) / `0x13` (bitir) komutları bu klavyede var mı?

O sırada klavyede görünen (kullanıcı): Esc, 1–5, Tab, Q W E R, A S D F G, Sol Shift, Sol Ctrl **kırmızı**.
Bu düzen resmi uygulamanın 4 profil dosyasındaki (UserDengColor_0..3, Mode0..9) hiçbir düzenle eşleşmiyor
→ büyük ihtimalle firmware'in kendi "oyun modu" hazır düzeni (tek renk, kullanıcı en son kırmızı seçmiş).
OpenRGB bu klavye ailesinde Begin/End'i **hiç kullanmıyor**.

Satır başları: 0 = Esc, 21 = `"`/~ satırı, 42 = Tab, 63 = Caps Lock, 84 = Sol Shift, 105 = Sol Ctrl (her satır 21 numara aralıklı).

## Bağlantı

- VID `0C45`, PID `5004`
- Arayüz: `MI_01`, usage page `FF1C` (Windows'ta `...MI_01&COL04`)
- Her komut **64 baytlık** bir HID output report, ardından klavyeden 64 baytlık cevap okunur.

## Paket yapısı (64 bayt)

| Bayt | Anlamı |
|---|---|
| 0 | Report ID = `0x04` |
| 1–2 | Checksum (little-endian): bayt 3..63 toplamı |
| 3 | Komut |
| 4 | Veri uzunluğu |
| 5–6 | Parametre / ofset (little-endian) |
| 7 | 0 |
| 8–63 | Veri (en fazla 0x36 = 54 bayt) |

## Komutlar

| Kod | Komut | Not |
|---|---|---|
| `0x01` | Begin (yapılandırmaya başla) | Sabit paket: `04 01 00 01 00…` |
| `0x02` | End (yapılandırmayı bitir) | Sabit paket: `04 02 00 02 00…` |
| `0x06` | Parametre yaz | bayt 5 = parametre no |
| `0x10` | Özel renkleri oku | sadece okuma, güvenli |
| `0x11` | Özel renkleri yaz | bayt 5–6 = ofset |

Sıra: **Begin → (Parametre / Renk paketleri) → End**

### Parametreler (komut 0x06)

`0x00` mod · `0x01` parlaklık (0–4) · `0x02` hız (0 hızlı – 5 yavaş) · `0x03` yön · `0x04` rastgele renk · `0x05` mod rengi (RGB) · `0x0F` polling rate (**dokunma**)

Toplu yazım: parametre `0x00`, 8 bayt veri = `mod, parlaklık, hız, yön, rastgele, R, G, B`

### Dahili modlar

`01` dalga kısa · `02` dalga uzun · `03` renk çarkı · `04` spektrum · `05` nefes · `06` sabit · `07` tepkili · `08` dalgalanma · `09` tepkili çizgi · `0A` yıldız hızlı · `0B` çiçek · `0C` dikey gökkuşağı · `0D` kasırga · `0E` birikme · `0F` yıldız yavaş · `10` visor · `11` surmount · `12` dairesel gökkuşağı · **`14` özel (per-key)**

### Tuş başına renk (komut 0x11)

- 126 LED × 3 bayt (R, G, B) = 378 bayt → 7 paket × 54 bayt
- LED sırası OpenRGB'deki 6×23 matrise göre (satır satır, boşluklar atlanmış). K3 Pro'da tuş dizilimi farklı olabilir → **test ile doğrulanacak**.

## ⚠️ Önemli kısıt: canlı mod yok

- OpenRGB bu klavye için "direct mode: yok", "otomatik kayıt: var" diyor.
- Yani renkler klavyenin **kalıcı hafızasına** yazılıyor. Saniyede onlarca kez yazmak flash belleği yıpratabilir.
- Daha yeni "EVision V2" firmware'inde canlı mod (`0xFF`) var, ama o farklı VID (`320F`) kullanıyor.

### Araştırılacaklar

1. Begin/End göndermeden yalnızca `0x11` paketleri gönderilirse renk anında değişiyor mu? (Kalıcı yazım genelde End'de olur.)
2. Klavye V2'nin "yetenekleri oku" (`0x03`) komutuna cevap veriyor mu? (Sadece okuma, güvenli.)
3. Resmi uygulamanın müzik / ekran modu var mı? Varsa USB yakalama ile RAM'e giden canlı komutu bulabiliriz.
