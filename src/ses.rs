// Copyright 2026 Eresbos (github.com/ERESB0S)
// SPDX-License-Identifier: GPL-3.0-or-later
//! Bilgisayarda çalan sesi (hoparlöre giden ses, WASAPI "loopback") dinler ve
//! saniyede ~60 kez 16 frekans bandına ayırır. Mikrofon kullanılmaz.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use windows::Win32::Media::Audio::{
    AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK, IAudioCaptureClient, IAudioClient,
    IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator, WAVEFORMATEX, WAVEFORMATEXTENSIBLE, eConsole, eRender,
};
use windows::Win32::System::Com::{CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree};

pub const BANT: usize = 16;
const N: usize = 1024; // FFT boyu (48 kHz'de ~21 ms)

/// Klavyeye gidecek bir ses karesi (hepsi 0..255).
#[derive(Clone, Copy, Default)]
pub struct SesKare {
    pub bant: [u8; BANT], // 0 = en bas .. 15 = en tiz
    pub seviye: u8,       // genel seviye
    pub ton: u8,          // baskın frekansa göre renk tonu (bas kırmızı .. tiz mor)
    pub bas: u8,          // bas vuruşu
}

/// Arka planda ses dinler. `aktif` false iken ses aygıtı kapalı kalır.
/// Her karede `gonder` çağrılır; sessizlikte sıfırlar gönderilir.
pub fn baslat<F: FnMut(&SesKare) + Send + 'static>(aktif: &'static AtomicBool, mut gonder: F) {
    std::thread::spawn(move || {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        }
        let mut analiz = Analiz::new();
        loop {
            if !aktif.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(200));
                continue;
            }
            // Aygıt değişir veya hata olursa baştan aç
            if let Err(_) = unsafe { dinle(aktif, &mut analiz, &mut gonder) } {
                std::thread::sleep(Duration::from_millis(1000));
            }
        }
    });
}

unsafe fn dinle<F: FnMut(&SesKare)>(aktif: &AtomicBool, analiz: &mut Analiz, gonder: &mut F) -> windows::core::Result<()> {
    unsafe {
        let numaralayici: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let aygit = numaralayici.GetDefaultAudioEndpoint(eRender, eConsole)?;
        let istemci: IAudioClient = aygit.Activate(CLSCTX_ALL, None)?;
        let bicim = istemci.GetMixFormat()?;
        let wf: WAVEFORMATEX = *bicim;
        let kanal = wf.nChannels.max(1) as usize;
        let bit = wf.wBitsPerSample;
        let hz = wf.nSamplesPerSec as f32;
        // Ortak modda karışım biçimi neredeyse her zaman 32 bit kayan nokta; 16 bit tamsayıyı da destekle
        let kayan = if wf.wFormatTag == 3 {
            true
        } else if wf.wFormatTag == 0xFFFE {
            let ext: WAVEFORMATEXTENSIBLE = *(bicim as *const WAVEFORMATEXTENSIBLE);
            let alt = ext.SubFormat;
            alt.data1 == 3
        } else {
            false
        };
        let sonuc = istemci.Initialize(AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_LOOPBACK, 200_000, 0, bicim, None);
        CoTaskMemFree(Some(bicim as *const _));
        sonuc?;
        let yakalayici: IAudioCaptureClient = istemci.GetService()?;
        istemci.Start()?;
        analiz.hz = hz;

        let kare_suresi = Duration::from_millis(16);
        let mut son = Instant::now();
        let mut aygit_kontrol = Instant::now();
        let ilk_aygit = aygit_id(&aygit);
        let sonuc = (|| -> windows::core::Result<()> {
            while aktif.load(Ordering::Relaxed) {
                loop {
                    let paket = yakalayici.GetNextPacketSize()?;
                    if paket == 0 {
                        break;
                    }
                    let mut veri: *mut u8 = std::ptr::null_mut();
                    let mut kare: u32 = 0;
                    let mut bayrak: u32 = 0;
                    yakalayici.GetBuffer(&mut veri, &mut kare, &mut bayrak, None, None)?;
                    let sessiz = bayrak & (AUDCLNT_BUFFERFLAGS_SILENT.0 as u32) != 0;
                    for k in 0..kare as usize {
                        let mut t = 0.0f32;
                        if !sessiz && !veri.is_null() {
                            for c in 0..kanal {
                                let i = k * kanal + c;
                                t += if kayan && bit == 32 {
                                    *(veri as *const f32).add(i)
                                } else if bit == 16 {
                                    *(veri as *const i16).add(i) as f32 / 32768.0
                                } else {
                                    0.0
                                };
                            }
                        }
                        analiz.ekle(t / kanal as f32);
                    }
                    yakalayici.ReleaseBuffer(kare)?;
                }
                let gecen = son.elapsed();
                if gecen >= kare_suresi {
                    son = Instant::now();
                    gonder(&analiz.kare());
                } else {
                    std::thread::sleep((kare_suresi - gecen).min(Duration::from_millis(4)));
                }
                // Varsayılan çıkış aygıtı değiştiyse (ör. kulaklık takıldı) yeniden aç
                if aygit_kontrol.elapsed() > Duration::from_secs(2) {
                    aygit_kontrol = Instant::now();
                    let simdiki = numaralayici.GetDefaultAudioEndpoint(eRender, eConsole).ok().and_then(|a| aygit_id(&a));
                    if simdiki != ilk_aygit {
                        return Err(windows::core::Error::empty());
                    }
                }
            }
            Ok(())
        })();
        let _ = istemci.Stop();
        sonuc
    }
}

/// Aygıt kimliği (karşılaştırmak için); Windows'un ayırdığı belleği serbest bırakır.
unsafe fn aygit_id(a: &IMMDevice) -> Option<String> {
    unsafe {
        let p = a.GetId().ok()?;
        let s = p.to_string().ok();
        CoTaskMemFree(Some(p.0 as *const _));
        s
    }
}

/// Örnekleri biriktirir, FFT ile bantlara ayırır, otomatik kazanç ve yumuşatma uygular.
struct Analiz {
    hz: f32,
    tampon: Vec<f32>, // son N örnek (halka)
    yaz: usize,
    pencere: Vec<f32>,
    tepe_db: f32,
    onceki: [f32; BANT],
    bas_yavas: f32,
    bas_cikis: f32,
    ton: f32,
}

impl Analiz {
    fn new() -> Self {
        let pencere = (0..N).map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / (N - 1) as f32).cos()).collect();
        Analiz {
            hz: 48000.0,
            tampon: vec![0.0; N],
            yaz: 0,
            pencere,
            tepe_db: -60.0,
            onceki: [0.0; BANT],
            bas_yavas: 0.0,
            bas_cikis: 0.0,
            ton: 0.0,
        }
    }

    fn ekle(&mut self, v: f32) {
        self.tampon[self.yaz] = v;
        self.yaz = (self.yaz + 1) % N;
    }

    fn kare(&mut self) -> SesKare {
        // Pencereli kopya -> FFT
        let mut re = [0f32; N];
        let mut im = [0f32; N];
        for i in 0..N {
            re[i] = self.tampon[(self.yaz + i) % N] * self.pencere[i];
        }
        fft(&mut re, &mut im);

        // 16 logaritmik bant: 40 Hz .. 16 kHz
        let (alt, ust) = (40.0f32, 16000.0f32.min(self.hz * 0.45));
        let cozum = self.hz / N as f32;
        let mut db = [-120f32; BANT];
        for b in 0..BANT {
            let f0 = alt * (ust / alt).powf(b as f32 / BANT as f32);
            let f1 = alt * (ust / alt).powf((b + 1) as f32 / BANT as f32);
            let i0 = ((f0 / cozum) as usize).max(1);
            let i1 = ((f1 / cozum) as usize).max(i0 + 1).min(N / 2);
            let mut guc = 0.0f32;
            for i in i0..i1 {
                guc += re[i] * re[i] + im[i] * im[i];
            }
            guc /= (i1 - i0) as f32;
            // Tiz bantlar doğal olarak zayıf: hafif eğimle dengele
            db[b] = 10.0 * (guc + 1e-12).log10() + b as f32 * 1.8;
        }

        // Otomatik kazanç: en güçlü bandı takip eder, yavaşça düşer (~6 dB/sn)
        let en = db.iter().cloned().fold(-120.0, f32::max);
        self.tepe_db = if en > self.tepe_db { en } else { (self.tepe_db - 0.1).max(-40.0) };
        let sessiz = en < -70.0;
        let aralik = 45.0;

        let mut k = SesKare::default();
        let mut toplam = 0.0;
        let mut agirlik = 0.0;
        for b in 0..BANT {
            let mut v = if sessiz { 0.0 } else { ((db[b] - (self.tepe_db - aralik)) / aralik).clamp(0.0, 1.0) };
            v = v * v; // daha belirgin hareket
            // Hızlı yüksel, yavaş düş
            let o = self.onceki[b];
            v = if v > o { v } else { o * 0.86 + v * 0.14 };
            self.onceki[b] = v;
            k.bant[b] = (v * 255.0) as u8;
            toplam += v;
            agirlik += v * b as f32;
        }
        let ort = toplam / BANT as f32;
        k.seviye = ((ort * 1.8).min(1.0) * 255.0) as u8;

        // Bas vuruşu: ilk 3 bandın ani yükselişi
        let bas = (self.onceki[0] + self.onceki[1] + self.onceki[2]) / 3.0;
        self.bas_yavas = self.bas_yavas * 0.95 + bas * 0.05;
        let vurus = ((bas - self.bas_yavas) * 3.0 + bas * 0.35).clamp(0.0, 1.0);
        self.bas_cikis = if vurus > self.bas_cikis { vurus } else { self.bas_cikis * 0.85 };
        k.bas = (self.bas_cikis * 255.0) as u8;

        // Renk tonu: ağırlık merkezi bas tarafındaysa kırmızı, tize doğru mavi/mor
        if toplam > 0.05 {
            let merkez = agirlik / toplam / (BANT - 1) as f32;
            self.ton = self.ton * 0.9 + merkez * 0.1;
        }
        k.ton = (self.ton * 200.0) as u8;
        k
    }
}

/// Yerinde, yinelemeli radix-2 FFT (N = 2'nin kuvveti).
fn fft(re: &mut [f32; N], im: &mut [f32; N]) {
    let mut j = 0;
    for i in 1..N {
        let mut bit = N >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut boy = 2;
    while boy <= N {
        let aci = -2.0 * std::f32::consts::PI / boy as f32;
        let (wr, wi) = (aci.cos(), aci.sin());
        let mut i = 0;
        while i < N {
            let (mut cr, mut ci) = (1.0f32, 0.0f32);
            for k in 0..boy / 2 {
                let (a, b) = (i + k, i + k + boy / 2);
                let tr = re[b] * cr - im[b] * ci;
                let ti = re[b] * ci + im[b] * cr;
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
                let yeni = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = yeni;
            }
            i += boy;
        }
        boy <<= 1;
    }
}
