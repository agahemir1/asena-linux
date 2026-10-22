# Asena Tasarım Sistemi (Asena Design System)

Asena Desktop ekosistemindeki tüm yerleşik ve üçüncü taraf uygulamalar için merkezi tasarım sistemi spesifikasyonudur. Asena'nın onaylı **Odak** tasarım felsefesi üzerine inşa edilmiştir; GTK 4 ve Libadwaita 1.7 platformunun yeteneklerini derinlikli malzeme, ölçülü kontrast ve tutarlı hiyerarşiyle harmanlar.

---

## 1. Tasarım Felsefesi ve Görsel Kimlik

- **Odak (Focus):** Arayüz gereksiz dekoratif rozetlerden, dolgu metinlerden ve dikkat dağıtıcı animasyonlardan arındırılmıştır. İçerik ve işlev birinci plandadır.
- **Doğal Derinlik ve Malzeme:** Pencereler ve paneller sakin nötr yüzeyler üzerinde hafif kenar aydınlığı, yumuşak temas gölgeleri ve buzlu cam hissiyle ayrışır.
- **Tipografi Önceliği:** Okunabilirlik temel ölçüttür; Inter ve JetBrains Mono yazı tipleri belirli hiyerarşik kademelerde kullanılır.
- **Türkçe Birinci Sınıf:** Noktalama, harf dönüşümleri (`i/İ/ı/I`), metin uzunlukları ve yerel biçimler baştan itibaren tasarım bütçelerine dahildir.

---

## 2. Tasarım Token'ları (Design Tokens)

Tüm Asena uygulamaları sabit hex kodları yerine aşağıdaki semantik token'ları kullanır. `data/style/shell.css` yalnız libadwaita adlandırılmış renklerini (`@window_bg_color`, `@accent_bg_color`, `@popover_bg_color`, `@shade_color`, `@light_1` …) ve `alpha(currentColor, …)` kullanır; tablodaki hex değerleri yalnız fikir vermek içindir.

### 2.1 Renk Paleti (Semantik Renkler)

| Token | Libadwaita Eşdeğeri | Koyu Mod Karşılığı | Açık Mod Karşılığı | Kullanım Alanı |
|---|---|---|---|---|
| `accent-primary` | `@accent_bg_color` | `#26a69a` (Teal 500) | `#00897b` (Teal 600) | Birincil eylem butonları, seçili sekmeler, aktif toggle |
| `accent-subtle` | `@accent_color` | `rgba(38, 166, 154, 0.15)` | `rgba(0, 137, 123, 0.12)` | Seçili liste satırları, hover vurguları |
| `surface-base` | `@window_bg_color` | `#1a1e24` (Grafit Lacivert) | `#f6f8fa` (Açık Gri-Mavi) | Pencere ana arka planı |
| `surface-raised` | `@card_bg_color` | `#22272e` (Yükseltilmiş Kart) | `#ffffff` (Saf Beyaz Kart) | Kartlar, içerik listeleri, modüller |
| `surface-overlay` | `@popover_bg_color` | `#292f38` (Koyu Popover) | `#ffffff` (Beyaz Popover) | Menüler, diyaloglar, açılır kutular |
| `border-subtle` | `alpha(currentColor, 0.08)` | `rgba(255, 255, 255, 0.08)` | `rgba(0, 0, 0, 0.08)` | Kart kenarlıkları, bölücü çizgiler |
| `border-strong` | `alpha(currentColor, 0.16)` | `rgba(255, 255, 255, 0.16)` | `rgba(0, 0, 0, 0.14)` | Giriş alanları, buton sınırları |
| `text-primary` | `@window_fg_color` | `#f0f3f6` | `#1f2328` | Başlıklar, birincil içerik metinleri |
| `text-secondary` | `@dim_label` | `#8b949e` | `#656d76` | Açıklama etiketleri, üstveri, zaman bilgileri |
| `status-success` | `@success_color` | `#3fb950` | `#1a7f37` | Başarılı durumlar, indirme tamamlandı |
| `status-warning` | `@warning_color` | `#d29922` | `#9a6700` | Uyarı bildirimleri, düşük pil |
| `status-error` | `@error_color` | `#f85149` | `#cf222e` | Hatalar, silme onayları, bağlantı kaybı |

### 2.2 Geometri ve Köşe Yarıçapları (Corner Radius)

| Token | Değer | GTK / CSS Karşılığı | Kullanım |
|---|---|---|---|
| `radius-control` | `8px` | `border-radius: 8px;` | Butonlar, text entry, dropdown, switch |
| `radius-card` | `12px` | `border-radius: 12px;` | Kartlar, list kutuları, küçük önizleme panelleri |
| `radius-dialog` | `16px` | `border-radius: 16px;` | Modal pencereler, popover menüler, bildirim kutuları |
| `radius-pill` | `9999px` | `border-radius: 9999px;` | Durum hapları, arama kutuları, etiketler |

### 2.3 Boşluk ve Aralık Ölçeği (Spacing Scale)

Tüm boşluklar ve padding değerleri 4px temel ızgarasına (grid) dayanır:

```
space-xxs: 2px   (Micro-spacing)
space-xs:  4px   (İkon ile metin arası, kompakt boşluk)
space-sm:  8px   (Buton içi dikey padding, satır içi boşluk)
space-md:  12px  (Kart içi bileşenler arası boşluk)
space-lg:  16px  (Konteyner ve pencere kenar padding)
space-xl:  24px  (Büyük bölümler arası mesafe)
space-xxl: 32px  (Ana sayfa modülleri arası ayırıcı)
```

### 2.4 Tipografi Skalası

| Düzey | Boyut / Satır Yüksekliği | Ağırlık | Yazı Tipi | Kullanım |
|---|---|---|---|---|
| `title-1` | 22pt / 28pt | Bold (700) | Inter | Uygulama büyük başlıkları |
| `title-2` | 18pt / 24pt | SemiBold (600) | Inter | Sayfa/görünüm ana başlığı |
| `title-3` | 15pt / 20pt | SemiBold (600) | Inter | Kart grup başlıkları, bölüm adları |
| `body-primary` | 11pt / 16pt | Regular (400) | Inter | Genel arayüz metinleri, girdi alanları |
| `body-secondary`| 9.5pt / 14pt| Regular (400) | Inter | İkincil etiketler, dipnotlar, üstveri |
| `code-mono` | 10.5pt / 15pt| Regular (400) | JetBrains Mono | Kod editörü, terminal, loglar, dosya yolları |

---

## 3. Tema ve Malzeme Entegrasyonu

- Libadwaita `adw::StyleManager` üzerinden tek noktadan koyu/açık mod takibi yapılır.
- GTK CSS `backdrop-filter` desteklemez; pencere arkasını bulanıklaştırmak compositor işidir (K13 açık). Geçici shell yüzeyleri şimdilik `alpha(@popover_bg_color, 0.97)` gibi neredeyse opak malzeme kullanır.
- Tüm kontroller klavye odağında (`:focus-visible`) 2px kalınlığında `alpha(@accent_color, 0.7)` odak halkası gösterir.
