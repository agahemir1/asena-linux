# Asena Etkileşim, Animasyon ve Durum Standartları (Interactions)

Asena Desktop uygulamalarındaki kullanıcı etkileşimlerini, geri bildirimleri, klavye kısayollarını ve erişilebilirlik standartlarını belirler.

---

## 1. Bileşen Durumları (Component States)

Her etkileşimli bileşen (butonlar, satırlar, anahtarlar, kartlar) aşağıdaki 5 durumu görsel ve işlevsel olarak kusursuz biçimde yansıtmalıdır:

1. **Varsayılan (Default):** Temiz, sakin nötr renkler ve net tipografi.
2. **Üzerine Gelme (Hover):**
   - Arka planda `%4` ile `%8` arası aydınlanma (`alpha(currentColor, 0.06)`).
   - İmleç standardı: Tıklanabilir alanlarda `pointer`, metin alanlarında `text`.
   - Geçiş süresi: 150ms doğrusal olmayan yumuşak renk değişimi.
3. **Basılma (Active/Pressed):**
   - Hafif içe çökme veya `%12` opaklık örtüsü (`alpha(currentColor, 0.12)`).
4. **Odaklanma (Focus):**
   - Klavyeyle gezintide (`Tab` / `Shift+Tab`) bileşenin çevresinde 2px kalınlığında, 2px mesafeli belirgin Odak Halkası (`@accent_color`) görünür.
   - Fare tıklamalarında odak halkası gizlenir (yalnızca klavye navigasyonunda gösterilir).
5. **Devre Dışı (Disabled):**
   - Opaklık `%40` seviyesine düşürülür; tüm tıklama ve hover olayları yok sayılır.

---

## 2. Klavye Kısayolları Standartları

Bütün Asena uygulamaları sistem genelinde tutarlı kısayol haritasını uygular:

| Kısayol | Standart Eylem |
|---|---|
| `Ctrl + N` | Yeni öğe (Yeni Sekme, Yeni Not, Yeni Dosya, Yeni Etkinlik) |
| `Ctrl + O` | Dosya / Belge Aç |
| `Ctrl + S` | Kaydet |
| `Ctrl + Shift + S` | Farklı Kaydet |
| `Ctrl + F` | Uygulama içi arama kutusunu odakla |
| `Ctrl + W` | Aktif sekmeyi veya belgeyi kapat |
| `Ctrl + Q` | Uygulamadan çık |
| `Ctrl + Z` / `Ctrl + Y` | Geri Al / Yinele |
| `F11` | Tam ekrana geç / Tam ekrandan çık |
| `F1` | Kısayollar ve Yardım penceresini aç |
| `Esc` | Açık arama çubuğunu, popover'ı veya modalı kapat |
| `Ctrl + +` / `Ctrl + -` / `Ctrl + 0` | Yakınlaştır / Uzaklaştır / Varsayılan Boyut |

---

## 3. Animasyon ve Hareket (Motion) Standartları

- **Açılma/Giriş:** 180ms, `cubic-bezier(0.25, 1, 0.5, 1)` (ease-out-cubic), opaklık 0 → 1 ve 8px dikey kayma.
- **Kapanma/Çıkış:** 120ms, `cubic-bezier(0.5, 0, 0.75, 0)` (ease-in-cubic), opaklık 1 → 0.
- **Durum Değişimi:** 150ms renk ve gölge geçişi.
- **Azaltılmış Hareket (Reduced Motion):**
  - GSettings `gtk-enable-animations` değeri `false` olduğunda tüm kayma ve boyut değiştirme animasyonları anında devre dışı kalır; yalnızca anlık opaklık değişimi uygulanır.

---

## 4. Geri Bildirim ve Toast Mesajları

- Başarılı arka plan işlemlerinde (dosya kopyalama bitti, not kaydedildi, ekran görüntüsü panoya alındı) ekranın alt ortasında `adw::Toast` gösterilir.
- Hatalı durumlarda kullanıcıya mutlaka ne yapması gerektiğini bildiren doğrudan ve samimi bir Türkçe açıklama sunulur.
- Geri alınabilir eylemlerde (Örn: Çöp kutusuna taşıma) toast içinde "Geri Al" butonu bulunur.

---

## 5. Erişilebilirlik (A11y) ve Ekran Okuyucu

1. Yalnızca simge içeren tüm butonlar (`flat` butonlar dahil) `accessible-role="button"` ve açıklayıcı `tooltip-text` ile `accessible-name` taşımalıdır (Örn: "Arama yap", "Kenar çubuğunu gizle").
2. Renk asla tek bilgi iletim yolu olamaz; kırmızı hata simgesinin yanında açıklayıcı metin veya ikon da yer almalıdır.
3. Kontrast oranı WCAG AA standartlarına uygun olarak metin ile zemin arasında en az 4.5:1 seviyesindedir.
