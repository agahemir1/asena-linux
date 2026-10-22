# Asena Uygulama Test ve Kalite Standartları (App Testing)

Asena Desktop ekosistemindeki uygulamaların kalite standartlarını, otomatik birim testlerini ve regresyon test süreçlerini belirler.

---

## 1. Test Piramidi ve Stratejisi

```
             / \
            / UI\       Manuel & Canlı Önizleme Testleri (VNC / Wayfire)
           /-----\
          / Enteg.\     D-Bus, GSettings ve GIO Entegrasyon Testleri
         /---------\
        /  Birim    \   Saf Rust Mantık Testleri (Hesaplama, Katlama, Ayrıştırma)
       /-------------\
```

### 1.1 Birim Testleri (Unit Tests)
Arayüzden bağımsız çalışan tüm çekirdek mantık birim testleriyle doğrulanır:
- **`asena-core::text`:** Türkçe büyük/küçük harf katlama (`fold`), Unicode normalizasyonu.
- **`asena-calculator`:** Aritmetik öncelik, trigonometrik hesaplar, programcı bit işlemleri, birim çevrimleri.
- **`asena-archive`:** Arşiv format algılama, dosya listeleme ve yol güvenliği (zip-slip açığı engeli).
- **`asena-clock`:** Kronometre tur hesaplamaları, geri sayım sayaç süreleri, zaman dilimi farkları.
- **`asena-calendar`:** Tekrarlayan etkinlik kuralları (RRULE hesaplama), ICS ayrıştırıcı.
- **`asena-weather`:** JSON yanıt ayrıştırma, hava kalitesi indeksi hesaplama, önbellek zaman aşımı.

### 1.2 Entegrasyon Testleri
- GSettings şema okuma ve yazma işlemlerinin doğrulanması (`target/schemas`).
- D-Bus arayüzlerinin (`org.asenalinux.*`, MPRIS, Notifications) standart yanıtlar üretmesi.
- FreeDesktop `.desktop` dosyalarının geçerliliği (`desktop-file-validate`).
- GResource XML dosyalarının eksiksiz dosya içermesi (`xmllint`).

---

## 2. Kalite Kapısı ve Çalıştırma

Bütün kod değişiklikleri `scripts/check-quality.sh` betiği üzerinden sınanır.

### Yerel Doğrulama Komutu:
```bash
bash scripts/check-quality.sh
```

### Kalite Kapısı Aşamaları:
1. `python3 scripts/check-source.py`: Tüm crate'lerin workspace yapılandırması, POTFILES listesi, XML geçerliliği, Linux betiklerinde LF sonlandırma kontrolü.
2. `bash scripts/compile-data.sh`: GSettings şemalarının ve GResource XML dosyalarının derlenmesi.
3. `bash scripts/update-po.sh --check`: Türkçe çeviri dosyasında eksik veya fuzzy girdi kalmadığının `msgcmp` ile denetlenmesi.
4. `shellcheck`, `xmllint`, `desktop-file-validate`: Statik kod analizleri.
5. `cargo fmt --all -- --check`: Rust kod biçimlendirme denetimi.
6. `cargo build --workspace --locked`: Tüm çalışma alanının bağımlılık kilitli derlenmesi.
7. `cargo clippy --workspace --all-targets --locked -- -D warnings`: Hiçbir clippy uyarısına izin vermeyen katı linter.
8. `cargo test --workspace --locked`: Tüm birim ve entegrasyon testleri.
9. `python3 -m unittest discover -s tests`: Python entegrasyon testleri.

---

## 3. Güvenlik ve Kararlılık Standartları

- **Zero-Panic Kuralı:** Kodda `unwrap()` veya `expect()` bulunamaz (`deny(clippy::unwrap_used)`).
- **Yol Geçişi Güvenliği:** Arşiv çıkarma ve dosya yöneticisi işlemlerinde `../` gibi yol geçişi (directory traversal) saldırılarına karşı her yol `canonicalize` ve sınır kontrolünden geçirilir.
- **Bellek ve Kaynak Temizliği:** Açılan dosya tanıtıcıları, GStreamer akışları ve arka plan thread'leri pencere kapandığında güvenle sonlandırılır.
