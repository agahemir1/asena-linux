# Asena Uygulama Yol Haritası

Asena'nın kendi GTK 4 / libadwaita uygulamalarının gerçek durumu, açık işleri ve kabul kapıları.
Ayrıntılı işlev listesi [APP_CATALOG.md](APP_CATALOG.md), denetim kaydı
[reports/audit-2026-10-21.md](../reports/audit-2026-10-21.md) içindedir.

## Durum düzeltmesi (21 Ekim 2026)
20 Ekim tarihli [app-ecosystem](../reports/app-ecosystem.md) raporu 16 uygulamayı "tamamlandı ve
doğrulandı" olarak gösteriyordu. 21 Ekim denetimi bunun doğru olmadığını buldu:

- Bütün uygulamalar açılışta paniğe düşüyordu (`asena_widgets::init()` GTK başlatılmadan
  çağrılıyordu). Hiçbiri kullanıcı tarafından açılamıyordu.
- 15 uygulama sabit örnek veriyle çalışan maketti: Uçbirim komut çalıştırmıyor, Hava Durumu,
  Mağaza, Kamera, Notlar, Takvim, Fotoğraflar, Oynatıcı, Arşiv ve Belgeler uydurma veri gösteriyordu.
  Bu, CLAUDE.md kural 12'nin ("sahte veri ve çalışmayan mockup bırakılamaz") ihlaliydi.

Kullanıcı kararıyla (21 Ekim): yalnız GTK/GIO ve mevcut Debian araçlarıyla gerçek yapılabilenler
gerçek veriye bağlandı; yeni bağımlılık isteyenler menüden gizlendi ve dürüst bir "henüz hazır değil"
penceresi gösterir.

## Uygulama durumu

| Uygulama | Kimlik | Durum | Not |
|---|---|---|---|
| Dosyalar | `org.asenalinux.Files` | 🟨 Gerçek, temel kapsam | GIO dizin listesi, Çöp, bağlı sürücüler; sürükle-bırak ve kopyala/yapıştır yok |
| Ayarlar | `org.asenalinux.Settings` | 🟨 Gerçek, temel kapsam | 17 panel gruplu; ekran düzeni daemon'un onay/geri alma akışıyla değişir |
| Metin Düzenleyici | `org.asenalinux.Text` | 🟨 Gerçek, temel kapsam | Düz metin; sözdizimi renklendirme yok |
| Notlar | `org.asenalinux.Notes` | 🟨 Gerçek, temel kapsam | Markdown dosyaları, otomatik kayıt |
| Takvim | `org.asenalinux.Calendar` | 🟨 Gerçek, temel kapsam | Yerel ICS; tekrar kuralları açılmaz, hatırlatıcı yok |
| Saat | `org.asenalinux.Clock` | 🟨 Gerçek, temel kapsam | Dünya saatleri, kronometre, zamanlayıcı; alarm yok |
| Hesap Makinesi | `org.asenalinux.Calculator` | 🟨 Gerçek | Temel/gelişmiş/programcı, birim dönüştürücü, geçmiş |
| Fotoğraflar | `org.asenalinux.Photos` | 🟨 Gerçek, temel kapsam | Resimler klasörü ızgarası ve görüntüleyici |
| Oynatıcı | `org.asenalinux.Player` | 🟨 Gerçek, temel kapsam | GTK + GStreamer; MPRIS sunucusu yok |
| Arşiv | `org.asenalinux.Archive` | 🟨 Gerçek, temel kapsam | `bsdtar` ile listeleme ve güvenli çıkarma; arşiv oluşturma yok |
| Ekran Görüntüsü | `org.asenalinux.Capture` | 🟨 Gerçek, temel kapsam | xdg-desktop-portal Screenshot; açıklama araçları yok |
| Uçbirim | `org.asenalinux.Terminal` | ⬜ Gizli | VTE (`vte4`) bağımlılığı kararı gerekiyor |
| Kamera | `org.asenalinux.Camera` | ⬜ Gizli | GStreamer kamera/PipeWire yolu gerekiyor |
| Belgeler | `org.asenalinux.Documents` | ⬜ Gizli | PDF işleme kitaplığı (poppler veya benzeri) kararı gerekiyor |
| Hava Durumu | `org.asenalinux.Weather` | ⬜ Gizli | HTTP istemcisi ve veri sağlayıcı kararı gerekiyor |
| Yazılım | `org.asenalinux.Store` | ⬜ Gizli | Kendi APT/Flatpak arka ucu (K21); plan [06f-yazilim.md](../../prompts/06f-yazilim.md) onay bekliyor |

Durum: ⬜ başlamadı veya gizli · 🟨 sürüyor · ✅ kabul edildi. Hiçbir uygulama kullanıcı görsel kabulünden
geçmediği için ✅ yoktur.

## Açık işler
1. Gizli beş uygulama için bağımlılık kararları (CLAUDE.md kural 10; her yeni crate için bakım ve
   GPL-3.0 uyumluluk denetimi).
2. Dosyalar: kopyala/kes/yapıştır, sürükle-bırak, ızgara görünümü, özellikler penceresi.
3. Oynatıcı: MPRIS sunucusu (kenar paneli medya kartı için).
4. Takvim: tekrar kuralları ve hatırlatıcılar (arka planda çalışan bir servis gerekir).
5. Saat: alarmlar (uygulama kapalıyken çalabilmesi için oturum servisi gerekir).
6. Ayarlar: ekran düzeni 22 Ekim'de eklendi; gerçek çoklu monitör ve fiziksel takıp çıkarma denenmedi.
7. Hacimli uygulama simgeleri: 16 uygulamanın hepsi çizildi (22 Ekim); klasörler onaylı, uygulama
   simgelerinin görsel onayı bekliyor ([ICONOGRAPHY.md](../ICONOGRAPHY.md)).

## Kabul kapısı (her uygulama için)
- [ ] Gerçek veri; örnek/uydurma içerik yok.
- [ ] Ana thread'de senkron IO yok; hatalar kullanıcıya eyleme dönük metinle gösterilir.
- [ ] 360 px genişlikte kullanılabilir; dar görünüm elle denendi.
- [ ] Bütün metinler `gettext` içinde, `po/tr.po` güncel.
- [ ] Mantık içeren kodun birim testi var.
- [ ] `scripts/dev-session.sh` veya önizlemede elle denendi ve rapora yazıldı.
- [ ] Kullanıcı görsel kabulü.
