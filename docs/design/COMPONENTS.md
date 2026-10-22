# Asena Ortak Bileşen Kütüphanesi (Component Library)

Asena Desktop ekosisteminde yer alan 16 uygulamanın tamamında yeniden kullanılan standart arayüz bileşenlerini, yapılandırma kurallarını ve en iyi pratikleri tanımlar.

---

## 1. Temel Pencere ve Navigasyon Bileşenleri

### 1.1 `AsenaHeaderBar` (`adw::HeaderBar`)
- **Kullanım:** Tüm pencerelerin en üstünde yer alır.
- **Standart Düzen:**
  - **Başlangıç (Start):** Geri butonu (alt görünümlerde), kenar çubuğu aç/kapa butonu (`sidebar-show-symbolic`).
  - **Merkez (Center):** Pencere başlığı veya çoklu sekmeli uygulamalarda `AdwViewSwitcherTitle`.
  - **Bitiş (End):** Arama butonu (`edit-find-symbolic`), birincil eylem butonu, hamburger menü (`open-menu-symbolic`), pencere kontrolleri (kapat/büyüt/küçült).
- **Kural:** Başlık çubuğuna 4'ten fazla kontrol butonu yerleştirilemez; ikincil işlemler hamburger menüye aktarılır.

### 1.2 `AsenaSplitView` (`adw::NavigationSplitView`)
- **Kullanım:** Settings, Files, Notes, Calendar, Store gibi iki sütunlu uygulamaların ana iskeletidir.
- **Davranış:**
  - Geniş ekranda (>720px): Sol taraf kenar çubuğu (sidebar), sağ taraf içerik alanı (content).
  - Dar ekranda (≤720px): Otomatik olarak tek sütuna çöker; kenar çubuğundaki seçim alt sayfayı tam ekran açar ve sol üste standart bir "Geri" butonu yerleştirir.

### 1.3 `AsenaViewSwitcher` (`adw::ViewSwitcher`)
- **Kullanım:** Calculator (modlar), Weather (günlük/saatlik), Player (müzik/video/liste) gibi yatay görünüm geçişleri.
- **Kural:** 5'ten fazla sekme bulunamaz. Sekme başlıkları kısa ve doğrudan olmalıdır.

---

## 2. İçerik ve Liste Bileşenleri

### 2.1 `AsenaActionRow` (`adw::ActionRow`) ve `AsenaPreferencesGroup`
- **Kullanım:** Ayar seçenekleri, dosya özellikleri, uygulama izinleri ve detay satırları.
- **Yapı:**
  - Sol ikon: 16px veya 24px sembolik ikon.
  - Başlık: Tek satır, net ad (Örn: "Karanlık Mod").
  - Alt etiket: Yalnızca gerekliyse ikincil durum bilgisi (Örn: "Sistem tercihini izler").
  - Sağ kontrol: `gtk::Switch`, `gtk::Button`, veya `gtk::Image` (ok simgesi).
- **Yasak:** Ayarın adını gereksizce tekrarlayan dolgu alt metinleri eklenemez.

### 2.2 `AsenaCard` (`gtk::Box` + CSS `.card`)
- **Kullanım:** Store uygulamaları, Fotoğraf albümleri, Hava durumu özet kartları, Saat alarmları.
- **Stil:** `border-radius: 12px; background: @card_bg_color; border: 1px solid alpha(currentColor, 0.08); padding: 12px;`.
- **Hover:** Hafif yüzey parlaması (`alpha(currentColor, 0.04)`) ve 150ms geçiş.

### 2.3 `AsenaEmptyState` (`adw::StatusPage`)
- **Kullanım:** Boş klasör, arama sonucu bulunamadı, not listesi boş, ağ bağlantısı yok durumları.
- **Yapı:**
  - Sembolik İkon (48px veya 64px)
  - Başlık (Örn: "Hiç Not Yok")
  - Açıklama (Örn: "Yeni bir not eklemek için '+' butonuna tıklayın.")
  - İsteğe Bağlı Eylem Butonu (Örn: "Not Oluştur")

### 2.4 `AsenaLoadingState`
- **Kullanım:** Ağdan veri çekilirken, arşiv çıkarılırken veya dosya taranırken gösterilir.
- **Yapı:** Merkezi `gtk::Spinner` + altında bilgilendirme etiketi ("Uygulamalar yükleniyor...").

### 2.5 `AsenaErrorState`
- **Kullanım:** API hatası, dosya okunamadı veya izin reddedildiğinde gösterilir.
- **Yapı:** `dialog-error-symbolic` ikonu, kırmızı hata başlığı, açıklayıcı hata metni ve "Yeniden Dene" eylem butonu.

---

## 3. Giriş ve Kontrol Bileşenleri

### 3.1 Butonlar (`gtk::Button`)
- **Birincil (`.suggested-action`):** Form onaylama, indirme, kaydetme işlemleri (Teal vurgu rengi).
- **Yıkıcı (`.destructive-action`):** Silme, sıfırlama, kalıcı kaldırma (Kırmızı uyarı rengi).
- **Düz (`.flat`):** İkincil araç çubuğu ikon butonları, iptal işlemleri.

### 3.2 Arama Kutusu (`gtk::SearchEntry`)
- **Kullanım:** Liste filtreleme, dosya arama, mağaza katalog araması.
- **Kural:** Her zaman Türkçe `asena_core::text::fold()` entegrasyonu ile çalışır. Klavyeden anlık filtreleme yapar; bekleme süresi maksimum 50ms'dir.

### 3.3 Toast Bildirimi (`adw::Toast`)
- **Kullanım:** İşlem tamamlandığında geri bildirim ("Kopyalandı", "Arşiv çıkarıldı", "Kayıt tamamlandı").
- **Özellik:** İsteğe bağlı "Geri Al" (Undo) butonu desteği taşır. 3 saniye sonra kendiliğinden kaybolur.

---

## 4. Diyalog ve Bildirim Standartları

- `adw::AlertDialog` modal uyarılar için kullanılır.
- Tehlikeli işlemler (dosya silme, verileri temizleme) yıkıcı butonla iki adımlı onay gerektirir.
- "Hakkında" penceresi standart `adw::AboutDialog` bileşeniyle üretilir; sürüm, lisans (GPL-3.0), katkıcılar ve Türkçe çevirmen bilgisi eksiksiz doldurulur.
