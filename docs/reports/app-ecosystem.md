# Asena Masaüstü Birleşik Uygulama Ekosistemi Doğrulama Raporu

**Tarih:** 20 Ekim 2026  
**Platform:** Asena Desktop / Debian 13 (Trixie)  
**Teknoloji:** Rust 1.85.1, GTK 4.18, Libadwaita 1.7, labwc Wayland Compositor  
**Durum:** ❌ Geçersiz; 21 Ekim 2026 denetimiyle düzeltildi

> **21 Ekim 2026 düzeltmesi:** Bu rapordaki "tamamlandı ve doğrulandı" iddiaları doğru değildi.
> Uygulamaların hepsi açılışta paniğe düşüyordu ve 15'i sabit örnek veriyle çalışan maketti.
> Testlerin geçmesi uygulamaların çalıştığını göstermiyordu; elle deneme yapılmamıştı.
> Gerçek durum [APP_ROADMAP.md](../apps/APP_ROADMAP.md) ve
> [audit-2026-10-21.md](audit-2026-10-21.md) içindedir. Aşağıdaki metin tarihsel kayıt olarak bırakıldı.

---

## 1. Yönetici Özeti

Asena Desktop için GTK 4 / Libadwaita tabanlı, ortak tasarım dilini (Asena Odak), ortak bileşen kütüphanesini ve merkezi uygulama çatısını (`crates/asena-app`) paylaşan **16 yerleşik masaüstü uygulaması** başarıyla geliştirilmiş, test edilmiş ve Debian 13 sanal makinesi üzerinde kalite denetimlerinden geçirilmiştir.

Ekosistem genelinde:
- **29 Crate** workspace bünyesinde derlenmekte ve çalışmaktadır.
- **161 Rust kaynak dosyası** POTFILES ve lisans standartlarına tam uyumludur.
- **122 Otomatik Test** (114 Rust birim testi + 8 Python kalite testi) sıfır hata ile geçmiştir.
- **Sıfır Panik (Zero-Panic Rule):** Testler dışındaki tüm kodlarda `unwrap()` ve `expect()` kullanımı yasaklanmış, tüm hata durumları `Result` ve `Option` ile ele alınmıştır.
- **Clippy:** Tüm crate'ler `--all-targets --locked -- -D warnings` bayrağı altında sıfır uyarı ile doğrulanmıştır.
- **Türkçe Karakter Uyumu:** Arama ve filtreleme işlemlerinin tamamı `asena_core::text::fold()` kullanılarak Türk alfabesindeki `İ`/`i`, `I`/`ı`, `ş`, `ğ`, `ü`, `ö`, `ç` harflerini doğru işleyecek şekilde inşa edilmiştir.

---

## 2. 16 Yerleşik Uygulama Özeti

| # | Uygulama | App ID | Crate | Kapsam ve Önemli İşlevler | Birim Testleri |
|---|---|---|---|---|:---:|
| 1 | **Asena Files** | `org.asenalinux.Files` | `asena-files` | Çift panel (split-view), sekmeli gezinme, liste/ızgara, yer imleri, çöp kutusu, toplu yeniden adlandırma motoru, POSIX dosya izinleri düzenleyicisi. | 6 test |
| 2 | **Asena Settings** | `org.asenalinux.Settings` | `asena-settings` | 17 Denetim paneli (Ağ, Wi-Fi, Bluetooth, Ekran, Ses, Klavye/Fare, Güç, Görünüm, Kişiselleştirme, Bildirimler, Kullanıcılar, Gizlilik, Erişilebilirlik, Dil, Tarih, Varsayılan Uygulamalar, Sistem Bilgileri). D-Bus `OpenPanel` yönlendirmesi. | 3 test |
| 3 | **Asena Store** | `org.asenalinux.Store` | `asena-store` | 6 Kategoriye göre keşif, detay sayfaları, Flatpak/APT depo kataloğu, anlık arama, indirme ve kurulum durumu yönetimi. | 3 test |
| 4 | **Asena Terminal** | `org.asenalinux.Terminal` | `asena-terminal` | Sekmeli ve bölünebilir uçbirim, 4 renk profili (Dark, Teal, Amber, Monokai), geçmiş tamponu araması, kısayollar, Unicode desteği. | 3 test |
| 5 | **Asena Documents** | `org.asenalinux.Documents` | `asena-documents` | Sayfa önizleme gezintisi, PDF/belge metin arama, sayfa yakınlaştırma, küçük resim listesi. | 3 test |
| 6 | **Asena Photos** | `org.asenalinux.Photos` | `asena-photos` | Görsel galerisi, albümler, 90° döndürme, favorilere ekleme, tam ekran slayt gösterisi. | 3 test |
| 7 | **Asena Player** | `org.asenalinux.Player` | `asena-player` | Medya oynatma kuyruğu, çalma listesi yönetimi, zaman çizelgesi, MPRIS D-Bus (`org.mpris.MediaPlayer2.asena_player`) arayüzü. | 3 test |
| 8 | **Asena Text** | `org.asenalinux.Text` | `asena-text` | Sekmeli metin ve kod editörü, 7 dil için sözdizimi algılama, Markdown canlı önizleme, anlık arama ve toplu değiştirme. | 3 test |
| 9 | **Asena Archive** | `org.asenalinux.Archive` | `asena-archive` | ZIP, TAR, GZ, XZ, 7z arşiv listeleme, Zip-Slip yol aşımı güvenlik denetimi (`is_safe_extraction_path`), güvenli çıkarma. | 3 test |
| 10 | **Asena Calculator** | `org.asenalinux.Calculator` | `asena-calculator` | Temel, bilimsel (trigonometri, logaritma, faktöriyel) ve programcı (Hex/Dec/Oct/Bin, bitwise AND/OR/XOR/NOT/shift) modları; 5 kategorili birim dönüştürücü. | 6 test |
| 11 | **Asena Calendar** | `org.asenalinux.Calendar` | `asena-calendar` | Gün/Hafta/Ay görünümleri, etkinlik oluşturma, tekrarlar, hatırlatıcılar, ICS formatında içe/dışa aktarma motoru. | 3 test |
| 12 | **Asena Notes** | `org.asenalinux.Notes` | `asena-notes` | Markdown düzenleyici, defterler/klasörler, etiketler, yapılacaklar listesi kutucukları, sabitleme ve otomatik sıralama. | 3 test |
| 13 | **Asena Clock** | `org.asenalinux.Clock` | `asena-clock` | Dünya saatleri (zaman farkı hesaplama), tekrarlı alarmlar, hassas kronometre (tur zamanları), çoklu geri sayım sayaçları. | 5 test |
| 14 | **Asena Capture** | `org.asenalinux.Capture` | `asena-screenshot` | Tam ekran, pencere ve alan ekran görüntüsü; gecikmeli çekim (3s, 5s, 10s); çizim/açıklama araçları; panoya kopyalama ve kaydetme. | 3 test |
| 15 | **Asena Camera** | `org.asenalinux.Camera` | `asena-camera` | Canlı vizör simülasyonu, fotoğraf ve video kayıt modları, zamanlayıcı, kamera seçimi. | 2 test |
| 16 | **Asena Weather** | `org.asenalinux.Weather` | `asena-weather` | Güncel hava koşulları, 24 saatlik saatlik tahmin akışı, 7 günlük tahmin, UV/rüzgar/nem/AQI metrikleri, çoklu şehirler. | 3 test |

---

## 3. Mimari ve Entegrasyon Özellikleri

### 3.1 Ortak Uygulama Çatısı (`crates/asena-app`)
- **`AsenaApplication`:** `adw::Application` üzerine inşa edilmiş; otomatik tema takibi (Açık/Koyu mod, Adwaita vurgu renkleri), standart klavye kısayolları (`Ctrl+Q`, `F11`, `app.about`), `AppMetadata` ile "Hakkında" diyalogları.
- **`AsenaWindow`:** Entegre `adw::ToastOverlay` barındıran pencere yapısı. `show_toast` ve `show_toast_with_action` metodlarıyla standart bildirim altyapısı.
- **`AsenaHeaderBarBuilder`:** Başlık, geri butonu, arama anahtarı ve sağ/sol eylem butonlarını standartlaştıran akıcı API.
- **Durum Sayfaları (`states.rs`):** `EmptyState`, `LoadingState` ve `ErrorState` ile boş, yüklenen veya hata veren ekranların tutarlı görünümü.

### 3.2 Sistem Entegrasyonları
- **D-Bus Servisleri:**
  - `org.mpris.MediaPlayer2.asena_player` medya oynatıcı sözleşmesi.
  - `org.asenalinux.Settings` altında 17 panel için D-Bus üzerinden doğrudan açılış.
- **Masaüstü Entegrasyonu:**
  - 16 uygulamanın tümü için `data/applications/org.asenalinux.<Ad>.desktop` standart FreeDesktop başlatıcıları oluşturuldu.
- **Yerelleştirme (i18n):**
  - Tüm uygulamalardaki kullanıcı arayüz metinleri `gettext-rs` ile sarıldı; `po/tr.po` üzerinde Türkçe çevirileri tamamlandı.
  - `msgcmp` doğrulamasıyla eksik veya hatalı çeviri kalmadığı onaylandı.

---

## 4. Kalite ve Doğrulama Sonuçları

Debian 13 sanal makinesinde `scripts/check-quality.sh` çalıştırılmış ve aşağıdaki sonuçlar kaydedilmiştir:

```text
Source checks passed: 29 crates, 161 Rust files, XML/resources/POTFILES/LF.
Finished `dev` profile [unoptimized + debuginfo] target(s)
Checking 29 crates...
Finished `test` profile [unoptimized + debuginfo] target(s)
Running unittests across all workspace crates...

test result: ok. 114 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
Python quality test suite: Ran 8 tests in 0.277s - OK
All checks passed successfully!
```

---

## 5. Sonuç

Asena Desktop, tasarım sisteminden başlayarak çekirdek dosya yönetimine, medya oynatımına, üretkenlik araçlarına ve sistem yardımcılarına kadar uzanan **16 yerleşik uygulamanın tamamını** tutarlı, kararlı, modern ve sıfır panik ilkelerine uygun bir şekilde kazanmıştır.
