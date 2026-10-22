# Asena prompt'ları

Bu klasör, Asena'yı Claude Code ile faz faz geliştirmek için yazılmış görev prompt'larını içerir.

## Şu anki aşama: önce kurgu

**Compositor güncellemesi, 21 Ekim 2026:** Kullanıcı onayıyla compositor labwc'den Wayfire 0.9.0'a geçti ([geçiş raporu](../docs/reports/wayfire-migration.md)). Tamamlanmış faz prompt'larındaki labwc, `data/labwc`, `rc.xml`, `themerc`, `labwc --reconfigure` ve `WLR_RENDERER=pixman` ifadeleri tarih olarak kaldı; karşılıkları `data/wayfire/wayfire.ini`, daemon'ın `wayfire` modülü, Wayfire'ın dosyayı kendisi yeniden okuması ve `WLR_RENDERER_ALLOW_SOFTWARE=1`'dir.

**Durum güncellemesi, 18 Ekim 2026:** Kullanıcı Faz 0 uygulamasını açıkça onayladı;
bu faz için yeniden onay bekleme ve T0 bitmeden altyapıya başlamama koşulları
kaldırıldı. Faz 0 kaynakları oluşturuldu, QEMU/Debian doğrulaması hazırlanıyor.
Güncel sonuç [kabul raporunda](../docs/reports/phase-00.md). Aşağıdaki planlama
akışı sonraki fazlar için korunur; diğer fazlar uygulanmadı.

Kullanıcının 18 Ekim 2026 yönlendirmesiyle önce bütün fazlar yazılıp gözden geçiriliyor. Kodlama, kurulum ve paket üretimi uygulamaya geçiş istendiğinde başlayacak. Prompt'un tamamlanması, fazın uygulanması değildir. Ortak plan ve açık kararlar: [PLANNING.md](../docs/PLANNING.md).

İlk adım [T0 — Odak tasarımı](T0-tasarim.md): kullanıcı Odak yönünü, derinlikli uygulama simgelerini, zarif çizgi simgeleri ve buzlu cam blur hedefini seçti. [DESIGN.md](../docs/DESIGN.md) ortak tasarım sözleşmesidir; T0, uygulama fazlarının numarasını değiştirmez.

| Faz | Görev | Planlanan çıktı |
|---|---|---|
| 0 | [Altyapı](00-altyapi.md) | Doğrulanmış Linux ortamı, workspace, boş panel |
| 1 | [Panel](01-panel.md) | Pencere yönetimi, saat, arka plan, CLI |
| 2 | [Menü ve bildirimler](02-menu-bildirim.md) | Arama, bildirimler, OSD |
| 3 | [Hızlı ayarlar](03-hizli-ayarlar.md) | Servis modelleri, donanım kontrolleri, tepsi, daemon |
| 4 | [Oturum ve güvenlik](04-oturum-guvenlik.md) | Kilit, uyku, giriş, polkit |
| 5 | [Ayarlar Merkezi](05-ayarlar.md) | MVP panelleri, menü/panel kişiselleştirme, güvenli ekran geri alma |
| 6 | [Dağıtım](06-dagitim.md) | .deb, test deposu, ISO, kurulum, Hoş geldin |
| 7 | [Cila](07-cila.md) | Ekran görüntüsü, erişilebilirlik, ölçümler, 1.0 kapanışı |

## Nasıl kullanılır
Bu adımlar, uygulamaya geçiş onaylandıktan sonra izlenir.

1. Depoyu Linux geliştirme ortamında aç (Debian 13 VM veya WSL2). Claude Code'u da orada çalıştır ki derleyip test edebilsin.
2. Her faz için **yeni bir oturum** başlat. Fazlar birbirinin bağlamını taşımaz; gereken her şey dosyalarda durur.
3. İlk mesaj olarak şunu yaz:
   ```
   prompts/01-panel.md dosyasındaki görevi uygula. Önce planını sun.
   ```
4. Planı oku; kapsam veya yaklaşım yanlışsa düzelt, sonra onay ver.
5. Bitince raporu kabul kriterleriyle karşılaştır, elle dene, commit et.
6. `docs/ROADMAP.md`'de durumu güncelle (Claude da yapar, sen kontrol et).

Büyük fazlar (3, 5, 6) tek oturumda bitmeyebilir. O zaman prompt'taki alt başlıkları sırayla ver:
```
prompts/05-ayarlar.md — sadece "Ağ" ve "Bluetooth" panellerini yap.
```

Faz 3–7'de `3A`, `4B`, `5D` gibi alt teslim kimlikleri bulunur. Bir alt teslimi uygulamak tüm fazı tamamlamaz; sıradaki grubun ön koşulları ve kabul kriterleri ayrıca sağlanır. Kilit, ekran geri alma ve dağıtım karar kapıları atlanmaz.

## Prompt anatomisi
Her faz prompt'u aynı yapıyı izler. Hangi bölümün neden var olduğu:

| Bölüm | Ne işe yarar |
|---|---|
| **Bağlam** | Modelin önce hangi dosyaları okuyacağını ve projenin neresinde olduğumuzu söyler. Tekrar yerine belgelere işaret eder. |
| **Hedef** | Fazın tek cümlelik, gözlemlenebilir sonucu. Model belirsiz anlarda buna göre karar verir. |
| **Yapılacaklar** | Numaralı, somut işler. "Ne" yapılacağını söyler; "nasıl" kısmını yalnızca mimari açıdan önemliyse dayatır. |
| **Kapsam dışı** | Modelin kendiliğinden yapmaya meyilli olduğu ama başka faza ait işler. Kapsam kaymasını önler. |
| **Kısıtlar** | Faza özgü teknik kurallar (CLAUDE.md'deki genel kurallara ek olarak). |
| **Dikkat** | Bilinen tuzaklar: modelin büyük ihtimalle yanlış yapacağı yerler. |
| **Kabul kriterleri** | İşaretlenebilir, doğrulanabilir maddeler. "İyi çalışır" yerine "şu komut şunu verir". |
| **Doğrulama** | Kriterleri kanıtlayan komutlar ve elle test adımları. |
| **Teslim** | Rapor biçimi; sonraki fazın ihtiyaç duyduğu bilgiler. |

## Yeni bir görev prompt'u yazmak
Faz dışı bir özellik veya hata için bu şablonu kopyala:

```markdown
# <Kısa başlık>

## Bağlam
Önce oku: CLAUDE.md, <ilgili docs/ dosyaları>, <ilgili kaynak dosyalar>.
<Neden bu iş? Hangi sorunu çözüyor? 2–3 cümle.>

## Hedef
<Tek cümle, gözlemlenebilir sonuç.>

## Yapılacaklar
1. …
2. …

## Kapsam dışı
- …

## Kısıtlar
- …

## Dikkat
- <Bilinen tuzak>

## Kabul kriterleri
- [ ] …

## Doğrulama
- `komut` → beklenen çıktı
- Elle: <adımlar>

## Teslim
Önce plan sun ve onay bekle. Bitince: ne yapıldı, nasıl doğrulandı, neler doğrulanamadı, sonraki adaylar.
```

## İyi prompt yazmanın kuralları
- **Doğrulanabilir yaz.** "Hızlı olsun" yerine "menü 100 ms altında açılsın, ölçümü logla".
- **Nedenini söyle.** "X yasak" yerine "X yasak, çünkü Y". Model kuralın amacını bilirse sınır durumlarda doğru karar verir.
- **Tuzakları önceden söyle.** Türkçe `İ`, ana thread bloklama, layer-shell klavye odağı gibi konular her seferinde hatırlatılmalı.
- **Belgelere işaret et, kopyalama.** Tek gerçek kaynağı `docs/` olsun; prompt'lar kısa kalsın.
- **Bir oturum = bir faz (veya alt başlık).** Uzun oturumlarda bağlam bulanıklaşır.
