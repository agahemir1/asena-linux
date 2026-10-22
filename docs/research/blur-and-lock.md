# Blur, session-lock ve protokol araştırması

İnceleme tarihi 18 Ekim 2026. Kaynak incelemesi yapıldı, çalışan compositor/GPU
deneyi yapılmadı. labwc ve bağımlılıkları değiştirilmedi.

## Gerçek blur

Debian 13 `labwc 0.8.3-1` / wlroots 0.18 tabanının incelenen kaynaklarında
arka plan efekt protokolü veya kullanılabilir backdrop blur yolu bulunmadı.
GTK'nin kendi widget'ını bulanıklaştırması, yüzeyin arkasındaki diğer istemcileri
bulanıklaştırmaz. Yalnız duvar kâğıdı blur'u hareket eden pencereleri kapsamaz;
sürekli ekran görüntüsü alan bir taklit uygulanmadı.

[Blur My Shell](https://github.com/aunetx/blur-my-shell), GNOME Shell içinde
çalışır; statik duvar kâğıdı efekti ile arkasındaki içeriği işleyen dinamik blur'u
ayrı tanımlar. GNOME'un shell/compositor çizim altyapısına erişimi vardır.
Asena'nın GTK layer-shell yüzeyi bu altyapının dışında bir Wayland istemcisidir.
Bu nedenle aynı eklenti labwc'ye kurulamaz; benzer sonuç compositor desteğiyle
yapılabilir. GNOME'a geçmek de yalnız bir eklenti kurmak değildir:
[gtk4-layer-shell upstream'i](https://github.com/wmww/gtk4-layer-shell)
GNOME Wayland'da layer-shell desteği olmadığını belirtir.

[labwc PR #3751](https://github.com/labwc/labwc/pull/3751),
`ext-background-effect-v1` ile gerçek blur önerisidir. GitHub API incelemesinde
`state=closed`, `merged=false`, `merged_at=null` döndü. Ayrıntı
[ham durum kaydında](blur-pr.json). Bunun mevcut Debian sürümünde destek olduğu
sonucu çıkarılamaz. Deneysel yamayı sessizce kurmak veya üretim güvenilirliği
varsaymak uygun değildir.

K13 için açık seçenekler:

| Yol | Gereken çalışma | Kabul riski |
|---|---|---|
| labwc'ye sınırlı backport/özgün efekt | Render yolunun ve bağımlı wlroots değişikliklerinin incelenmesi; blur bölgesi protokolü; GTK yüzeyinden istek gönderme köprüsü | GPU maliyeti, hasar takibi, çoklu çıkış, güvenlik düzeltmeleri ve uzun süreli yama bakımı |
| Destekli compositor adayı değerlendirme | Layer-shell, toplevel, lock, output ve blur protokollerinin Debian/GTK ile aynı deneyde karşılaştırılması | Compositor değişikliği ayrı mimari karar; tema ve oturum entegrasyonu yeniden sınanır |
| Opak uyumluluk modu | Mevcut Faz 0 CSS'i | Erişilebilir geri dönüş sağlar; gerçek blur kabulünü kapatmaz |

Önerilen bir sonraki araştırma: izole Debian VM'de labwc yamasının wlroots 0.18'e
taşınabilirliğini ve GTK yüzeyiyle gerçek protokol bağını küçük bir deneyde ölçmek.
Compositor değişikliğine veya bir fork'u ürün tabanı yapmaya bu öneriyle karar
verilmiş değildir. Kaynak açık olduğu için blur yapılabilir; maliyet GTK CSS'inden
ziyade compositor render ve yüzey entegrasyonundadır.

K13 kapanış kanıtı: panel ve popup arkasında hareket eden pencereyle gerçek blur;
%100/%200 ve farklı çıkışlar; yüksek kontrast/azaltılmış saydamlık; GPU ve pixman
yolunda geri dönüş; kare süresi/boşta CPU/RSS karşılaştırması; bakım/lisans ve
gerekirse FFI istisnası incelemesi. Faz 0 bunları tamamlandı saymaz.

## Session-lock

[gtk4-layer-shell değişiklik günlüğü](https://github.com/wmww/gtk4-layer-shell/blob/main/CHANGELOG.md)
session-lock API'sini 1.1.0 ile ekler. Debian'daki C kitaplığı 1.0.4'tür.
Seçilen Rust layer-shell 0.5.0 panel binding'i kilit binding'i değildir.
labwc 0.8.3 kaynak kodunda `wlr_session_lock_manager_v1_create` bulunması
compositor tarafındaki desteğe işaret eder; çalışma zamanı protokol ilanı henüz
`wayland-info` ile kanıtlanmadı.

Faz 4 öncesi seçenekler: güvenlik düzeltmeleri incelenmiş C kitaplığı ve eşleşen
Rust session-lock binding'i için açık backport kararı veya ayrı, dar kapsamlı
session-lock istemcisi. 1.1.0'ın bulunması tek başına yeterli güvenlik kanıtı değildir;
sonraki sürümlerde kilit hatası ve monitor yaşam döngüsü düzeltmeleri de vardır.
Bu faz C API sınırını yükseltmez ve `asena-lock` çalışıyor gibi davranmaz.

## Protokol ön envanteri

Kaynak: [labwc 0.8.3 etiketi](https://github.com/labwc/labwc/tree/0.8.3),
özellikle `src/server.c`, `src/layers.c`, `src/output.c`, `src/idle.c`,
`src/session-lock.c`, `src/workspaces.c`.

- Layer-shell, foreign-toplevel, session-lock, idle-notify, output management,
  output power, gamma control, screencopy ve wlr-data-control oluşturma yolları mevcut.
- ext-workspace, bu etikette yerel protokol uygulamasıyla mevcut. Kaynakta yalnız
  COSMIC workspace var sanılmamalı.
- ext-image-copy-capture ve ext-data-control için bu etikette oluşturma yolu bulunmadı;
  wlr karşılıkları öncelikli adaydır. Ekran görüntüsü protokolü ve toplevel listesi,
  pencere geometrisinin erişilebilir olduğunu kanıtlamaz; K10 açıktır.
- Tüm ilan edilen protokoller ve sürümleri iç içe oturumda ayrıca toplanacak.

## K14 ve açık tasarım ayrıntıları

Odak varsayılanı, ayrı hacimli uygulama/zarif sembolik simge aileleri ve responsive
uygulama sözleşmesi korunuyor. `fonts-inter` ve `fonts-jetbrains-mono` Debian
indeksinde mevcut. GTK3 tema, nihai simge/imleç paketi, Flatpak tema aktarımı,
ilk dört özgün simge, panel/simge boyut aralıkları ve cam parametreleri açık.
Faz 0 bu varlıkları veya bütün ekranları üretmez ve kalite onayı varsaymaz.
