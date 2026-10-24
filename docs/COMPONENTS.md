# Asena Bileşenleri

Bu belge Asena masaüstündeki her aracı, ne iş yaptığını ve hangi fazda geleceğini tanımlar.
Fazların ayrıntısı için [ROADMAP.md](ROADMAP.md), teknik yapı için [ARCHITECTURE.md](ARCHITECTURE.md).

Bağımlılıklar ve kapsam sınırları [PLANNING.md](PLANNING.md) içindedir. Faz 0 kaynak uygulaması başladı; sonraki fazların özellikleri henüz uygulanmadı.

Etiketler: **MVP** = ilk kullanılabilir sürümde olmalı · **Sonra** = MVP sonrası · **Hazır** = mevcut bir proje entegre edilir

---

## 1. Kendi yazdığımız süreçler

| Binary | Tür | Görev | Faz |
|---|---|---|---|
| `asena-session` | CLI | Oturumu başlatır: ortam değişkenleri, Wayfire, systemd user target | 0 |
| `asena-shell` | GTK4 daemon | Panel, görev çubuğu, başlat menüsü, hızlı ayarlar, bildirimler, OSD, duvar kâğıdı | 1–3 |
| `asenactl` | CLI | Klavye kısayollarından shell'e komut gönderir (D-Bus) | 1 |
| `asena-daemon` | daemon | Ayarları sisteme uygular: renk şeması, gece ışığı, boşta kalma, klavye düzeni, kapak | 3–4 |
| `asena-lock` | GTK4 | Kilit ekranı (ext-session-lock) | 4 |
| `asena-greeter` | GTK4 | greetd için giriş ekranı | 4 |
| `asena-polkit-agent` | GTK4 | Yönetici parolası diyaloğu | 4 |
| `asena-settings` | libadwaita uygulaması | Ayarlar | 5 |
| `asena-welcome` | libadwaita uygulaması | İlk açılış turu | 6 |
| `asena-screenshot` | libadwaita uygulaması | Ekran görüntüsü (ve sonra kayıt) | 7 |

---

## 2. Bileşen ayrıntıları

### 2.1 Panel (`asena-shell`)
Ekranın altında, 48 px yüksekliğinde; pencereler panelin altına girmez (exclusive zone).

```
┌───┬────────────────────────────┬─────────────────────────────────┐
│ ◆ │ ▣  ▣•  ▣••  ▣              │ ⋯  TR  [ağ ses pil]  14:32  (3) │
└───┴────────────────────────────┴─────────────────────────────────┘
  │    Görev çubuğu                  Tepsi, düzen, durum, saat, bildirim
  Başlat
```

**MVP**
- **Başlat düğmesi:** başlat menüsünü açar/kapatır.
- **Görev çubuğu:** sabitlenmiş ve çalışan uygulamalar, uygulamaya göre gruplu.
  - Tek tık: pencere odakta değilse odakla, odaktaysa küçült. Birden çok pencere varsa pencere listesi açılır.
  - Orta tık: yeni pencere. Sağ tık: Sabitle/Kaldır, Yeni pencere, Tümünü kapat.
  - Çalışan göstergesi (nokta) ve dikkat isteyen pencere için vurgu.
  - Kaynak: `wlr-foreign-toplevel-management`. Pencerenin `app_id`'si `.desktop` dosyasıyla eşleştirilir (StartupWMClass, Flatpak ID'leri ve küçük/büyük harf farkları dahil).
- **Sistem tepsisi:** StatusNotifierItem (Discord, Steam, nm-applet vb.), dbusmenu menüleriyle.
- **Klavye düzeni göstergesi:** TR / TR-F / US. *(Henüz yok.)*
- **Durum düğmesi:** ağ (Wi-Fi gücü/kablolu/bağlanıyor/kapalı), Bluetooth (açıkken), ses düzeyi ve pil simgeleri tek düğmede; rahatsız etme açıkken onun simgesi de görünür. Servis yoksa simge gizlenir. Tıklayınca Hızlı Ayarlar açılır. *(21 Ekim'de uygulandı.)*
- **Saat/tarih:** iki satır (saat ve kısa tarih); tıklayınca bildirim kenar paneli açılır. Okunmamış bildirim sayısı saatin yanında sade sayı olarak görünür.
- Her monitörde bir panel; monitör takılıp çıkarıldığında kendini güncelle.

**Faz 5:** Ayarlar Merkezi'nden paneli alt/üst/sol/sağ kenara taşıma; başlangıç/orta/son hizalama; uyumlu panel ve simge boyutu; sabitlenmiş uygulama sırası; tüm/seçili monitör kapsamı. Açılır yüzeylerin yönü ve exclusive zone aynı yerleşim modelini izler.

**Sonra:** otomatik gizleme, pencere önizlemeleri, çalışma alanı göstergesi.

### 2.2 Başlat menüsü (`asena-shell`)
**MVP**
- Super tuşuna basıp bırakınca veya Başlat düğmesiyle açılır; Esc ya da dışarı tıklayınca kapanır.
- **Arama** (açılır açılmaz yazılabilir):
  - Uygulamalar: Name, GenericName, Keywords alanlarında arar.
  - Ayarlar panelleri (Faz 5 ile).
  - Hesap makinesi: `2+2*3` yazınca sonucu gösterir.
  - Komut çalıştırma: `>` ile başlayan girişler.
  - **Türkçe duyarlı eşleşme:** `i/İ/ı/I` ve `ç ğ ö ş ü` normalize edilir; "ayarlar", "AYARLAR" ve "Ayarlar" aynı sonucu verir, "isik" yazınca "Işık" bulunur.
  - Sıralama: önek eşleşmesi > kelime başı > alt dize > bulanık eşleşme; sık kullanılanlar öne çıkar.
- **Sabitlenmiş uygulamalar:** varsayılan Odak menüsünde satırlar; seçilen menü tipine göre ızgara/iki bölümlü sunum. Veri kaynağı aynıdır.
- **Tüm uygulamalar:** Türkçe harf başlıklı alfabetik liste; harf başlığı menü içinde harf ızgarasını açar (liste daralmaz).
- **Üst satır:** kullanıcı avatarı ve adı (AccountsService), Ayarlar, kilit ve güç menüsü (Askıya al, Oturumu kapat, Yeniden başlat, Kapat; logind). Arama alanı panele bakan kenardadır.
- Tamamen klavyeyle kullanılabilir.

**Sonra:** son kullanılan dosyalar, dosya araması, sürükle-bırakla sabitleme sırası, kategori görünümü.

**Faz 5:** Ayarlar Merkezi'nden Odak kompakt liste, klasik iki bölümlü ve ızgara menü tipleri arasında geçiş. Arama, sabitlemeler ve klavye davranışı korunur; panel kenarına göre menü ekran içine açılır.

### 2.3 Hızlı Ayarlar (`asena-shell`)
**MVP**
- **Kaydırıcılar:** ses (çıkış cihazı seçimi dahil), parlaklık (donanımda varsa).
- **Bağlantı satırları:** Wi-Fi ve Bluetooth (anahtar + yerinde açılan ağ/aygıt listesi, parolayla bağlanma), kablolu durum.
- **Modlar:** rahatsız etme, koyu stil, gece ışığı, güç profili (hap düğmeler). Mikrofon kısayolu henüz yok.
- Pil yüzdesi ve şarj durumu (kalan süre henüz yok).
- **Medya kartı (MPRIS)** 21 Ekim'den beri bildirim kenar panelindedir.
- Ekran görüntüsü, Ayarlar ve güç menüsü kısayolları.
- **Donanım yoksa ilgili öğe gizlenir.** Örneğin Bluetooth adaptörü yoksa Bluetooth döşemesi görünmez; shell çökmez.

**Sonra:** VPN, hotspot, uçak modu, ekran kaydı, ekran döndürme.

### 2.4 Bildirimler (`asena-shell`)
**MVP**
- `org.freedesktop.Notifications` sunucusu (spec 1.2).
  - Desteklenenler: actions, body, body-markup (güvenli Pango alt kümesine temizlenir), icon-static, persistence, image-data, urgency, replaces_id.
- **Açılır bildirimler:** sağ altta, panelin üstünde; en fazla 3 tane üst üste. Kritik bildirimler kapatılana kadar kalır, fare üzerindeyken süre durur.
- **Bildirim kenar paneli** (saate tıklayınca ekranın sağından boydan boya kayar, Budgie Raven yaklaşımı): gün ve tarih, takvim, MPRIS medya kartı, uygulamaya göre gruplu bildirim geçmişi, Tümünü temizle, Rahatsız etme.

**Faz 5:** uygulama başına bildirim izni.

**Sonra:** kilit ekranında bildirim sayısı.

### 2.5 OSD (`asena-shell`)
Ses, parlaklık, mikrofonun kapatılması ve klavye düzeni değişikliğinde alt ortada ~1,5 sn görünür. Tıklamaları engellemez (girdi bölgesi boş).

### 2.6 Duvar kâğıdı (`asena-shell`)
Her monitörde bir arka plan yüzeyi. Resim veya düz renk; doldur, sığdır ya da ortala modları. Değişimde yumuşak geçiş. Varsayılan Asena duvar kâğıtlarının açık ve koyu sürümleri olur.

**Masaüstü simgeleri (21 Ekim):** panel monitöründe Ev ve XDG Masaüstü klasörü içeriği; çift tık/Enter açar, F2 yeniden adlandırır, Delete çöpe taşır. Boş alanda sağ tık: yeni klasör, uçbirimde aç, Dosyalar'da aç, simgeleri göster, arka planı değiştir, ekran ayarları, masaüstü ve panel ayarları. Diğer monitörlerde yalnız sağ tık menüsü vardır. Simge konumlarının kalıcılığı ve sürükle-bırak sonraki adaydır.

### 2.7 `asenactl`
Kısayolların çağırdığı komut satırı aracı:
```
asenactl menu toggle
asenactl quick-settings toggle
asenactl notifications toggle
asenactl volume up|down|mute
asenactl mic mute
asenactl brightness up|down
asenactl screenshot area|window|screen
asenactl lock
asenactl reload-style            # geliştirme: CSS'i canlı yeniden yükle
```

### 2.8 `asena-daemon`
- **Renk şeması ve vurgu rengi:** `org.asenalinux.desktop` ayarlarını `org.gnome.desktop.interface`'e yansıtır, vurguya göre simge temasını (`Asena-<vurgu>`) seçer ve Flatpak uygulamaları için kendi Settings portal arka ucunda `color-scheme` ile `accent-color` sunar. *(Portal ve simge teması 22 Ekim'de uygulandı.)*
- **Gece ışığı:** MVP'de `wlsunset` sarmalanır, sonra yerli `wlr-gamma-control` uygulamasına geçilir.
- **Boşta kalma:** sırayla karart → kilitle → ekranı kapat → askıya al. MVP'de `swayidle`, sonra yerli `ext-idle-notify`.
- **Klavye düzeni:** GSettings'ten Wayfire `[input]` bölümüne `xkb_layout`, `xkb_options` ve yineleme değerlerini yazar; Wayfire dosyayı kendisi yeniden okur. *(21 Ekim'de uygulandı.)*
- **Oturum dili:** `~/.config/environment.d/90-asena-language.conf` yazar; bir sonraki girişte geçerli.
- **Kapak ve uyku:** logind inhibitörü tutar; uykuya girmeden önce ekranı kilitler.
- **Wayfire yapılandırması:** `~/.config/asena/wayfire.ini` dosyasını `data/wayfire/wayfire.ini` şablonu ve GSettings'ten üretir (klavye, fare, dokunmatik yüzey). Saydamlık kapalıysa `blur`, animasyonlar kapalıysa `animate` eklentisini çıkarır.

### 2.9 Kilit ekranı (`asena-lock`)
- `ext-session-lock-v1` kullanır: kilitleyici çökse bile oturum kilitli kalır.
- Her monitörde bir yüzey; parola alanı odaktaki monitörde.
- Saat/tarih, avatar, Caps Lock uyarısı, klavye düzeni. Yanlış parolada sallanma animasyonu.
- Kimlik doğrulama PAM ile (`/etc/pam.d/asena-lock`).
- **Sonra:** medya kontrolleri, içerik göstermeyen bildirim sayısı.

### 2.10 Giriş ekranı (`asena-greeter`)
- greetd IPC ile çalışır, `cage` içinde açılır.
- Kullanıcı listesi (AccountsService), son kullanıcıyı hatırlar, oturum seçimi, klavye düzeni, güç düğmeleri.
- Masaüstüyle aynı tasarım dili ve duvar kâğıdı.

### 2.11 Polkit ajanı (`asena-polkit-agent`)
- libpolkit-agent-1 ile oturuma kaydolur. Diyalog: eylemin açıklaması, kimlik seçimi (birden fazla yönetici varsa), parola, hata ve yeniden deneme.
- Kendi ajanı hazır olana kadar geçici olarak mevcut bir ajan (ör. `mate-polkit`) kullanılır.

### 2.12 Ayarlar Merkezi (`asena-settings`)
**MVP panelleri**

| Panel | Kaynak / mekanizma |
|---|---|
| Ağ | NetworkManager: Wi-Fi listesi, ayrıntılar, unutma, Ethernet durumu |
| Bluetooth | BlueZ: tarama, eşleştirme (Agent1), bağlanma, kaldırma |
| Ekranlar | wlr-output-management: birincil ekran, çözünürlük, yenileme hızı, ölçek, döndürme, birincil ekrana göre konum ve ekranı kapatma. Uygula ile değişiklik hemen gelir; onaylanmazsa, Ayarlar kapanır veya çökerse ya da ekranlar değişirse 15 sn içinde geri alınır. Onaylanan düzen aynı ekran takımı için `~/.config/asena/displays.json` dosyasına kaydedilir ve daemon başlarken ya da o ekranlar yeniden bağlanınca uygulanır. *(22 Ekim'de uygulandı.)* |
| Ses | Çıkış/giriş cihazları, düzeyler, uygulama başına ses |
| Görünüm | Açık/koyu, vurgu rengi (libadwaita paleti), duvar kâğıdı, metin boyutu |
| Masaüstü ve Panel | Menü tipi, panel kenarı, hizalama, panel/simge boyutu, sabitlenmiş uygulama sırası, monitör kapsamı, saat biçimi; canlı önizleme ve düzeni varsayılana döndürme |
| Bildirimler | Rahatsız etme, uygulama başına aç/kapa |
| Güç | Güç profili, ekran kapanma ve askıya alma süreleri, kapak davranışı |
| Klavye | Düzenler (TR-Q, TR-F, US…), geçiş kısayolu, tekrar hızı |
| Fare ve Dokunmatik Yüzey | Hız, doğal kaydırma, dokunarak tıklama |
| Kullanıcılar | AccountsService: ad, avatar, parola |
| Tarih ve Saat | timedated: NTP, saat dilimi, 24 saat biçimi |
| Bölge ve Dil | localed: sistem dili, biçimler |
| Varsayılan Uygulamalar | Web, e-posta, müzik, video, fotoğraf, dosyalar, terminal |
| Hakkında | hostnamed, `/etc/os-release`, donanım özeti, Asena sürümü |

**Sonra:** Yazıcılar, Erişilebilirlik, Gizlilik, Uygulamalar (Flatpak izinleri), Paylaşım, Kısayol düzenleyici, Kurtarma (2.16).

### 2.13 Hoş geldin (`asena-welcome`)
İlk oturum açılışında başlar. Adımlar: Görünüm (açık/koyu, vurgu) → Panel hizası → Flathub'ı etkinleştir → Temel kısayollar → Bitti.

### 2.14 Ekran görüntüsü (`asena-screenshot`)
- Alan, pencere veya tüm ekran yakalama; gecikmeli çekim.
- Görüntü panoya kopyalanır ve `~/Resimler/Ekran Görüntüleri` dizinine kaydedilir; "Aç" ve "Klasörü göster" düğmeli bir bildirim gelir.
- 21 Ekim uygulaması `org.freedesktop.portal.Screenshot` kullanır (Wayfire'da xdg-desktop-portal-wlr); `asenactl screenshot` ve Print tuşu aynı yoldan kaydeder ve bildirim gönderir. İşaretleme ve ekran kaydı sonraki adaydır.

### 2.15 Pano geçmişi (Sonra)
1.0 kapsamına dahil değildir. Faz 7'deki pano işi ekran görüntüsünü panoya kopyalamaktır; geçmiş için ayrı kapsam kararı ve prompt gerekir.

Super+V ile açılır. `ext-data-control` (veya `wlr-data-control`) kullanır; metin ve görsel tutar, en fazla 50 öğe. Parola yöneticisinden gelen içerik kaydedilmez. Geçmiş yalnızca bellekte tutulur, diske yazılmaz.

### 2.16 Kurtarma ve sıfırlama (planlandı, K30)
1.0 kapsamındadır; bölüm düzeni 6D kurulumunda gelir. Karar ve açık sorular [PLANNING.md](PLANNING.md) K30 kaydındadır.
- Ayarlar › Sistem › Kurtarma: Asena ayarlarını sıfırla (yetkisiz, kullanıcı başına), Bu bilgisayarı sıfırla ("Dosyalarımı koru" veya "Her şeyi kaldır"), sistem raporu.
- Kurtarma bölümü: ISO'nun canlı imajı; GRUB'da her zaman "Asena Kurtarma" girdisi, kurulu sistem açılmasa da sıfırlama yapılabilir.
- Oturumdan başlatma polkit korumalı sistem servisiyle yapılır (öneri `org.asenalinux.Recovery`); sıfırlamayı kurtarma ortamındaki araç yapar.
- Disk arızasına karşı koruma değildir; o iş Yedekleme uygulamasınındır (K31).

---

## 3. Entegre edilen hazır bileşenler

| Alan | Seçim | Not |
|---|---|---|
| Compositor | **Wayfire 0.9** | Dual Kawase blur, sunucu tarafı başlıklar, kısayollar ve animasyon `data/wayfire/wayfire.ini` içinde |
| Giriş daemon'u | greetd + cage | Greeter bizim |
| Portallar | xdg-desktop-portal-wlr + xdg-desktop-portal-gtk | `asena-portals.conf` ile eşlenir |
| Ses | PipeWire + WirePlumber + pipewire-pulse | |
| Ağ | NetworkManager | |
| Bluetooth | BlueZ | |
| Güç | UPower, power-profiles-daemon | |
| Oturum | systemd-logind | |
| Kullanıcılar | AccountsService | |
| Uygulama dağıtımı | APT + Flatpak/Flathub | |
| Yazı tipleri | Inter (arayüz), JetBrains Mono (sabit genişlik) | |
| Simgeler | Adwaita (sembolik) + Asena marka simgeleri | |
| Kurulum | Calamares (Qt — bilinçli istisna) | Sonra: GTK4 ile kendi kurulum sihirbazımız |

## 4. Varsayılan uygulamalar (GTK4/libadwaita)

Shell olgunlaşana kadar uygulamaları yeniden yazmıyoruz; GNOME'un GTK4 uygulamaları tasarım dilimizle zaten uyumlu.
**Faz 0'da her paketin Debian 13'te bulunduğunu doğrula.**

| İhtiyaç | Uygulama | Yedek |
|---|---|---|
| Dosyalar | Nautilus | — |
| Terminal | Ptyxis | GNOME Console |
| Metin düzenleyici | GNOME Text Editor | — |
| Resim görüntüleyici | Loupe | — |
| Belge görüntüleyici | Papers | Evince |
| Video | Celluloid | — |
| Hesap makinesi | GNOME Calculator | — |
| Yazılım merkezi | Asena Yazılım: kendi APT ve Flatpak arka ucu, PackageKit yok (K21, [Faz 6F](../prompts/06f-yazilim.md)) | — |
| Sistem izleyici | Asena Sistem İzleyici (K31, planlandı) | — |
| Arşiv | File Roller | — |
| Tarayıcı | Firefox ESR | — |

**Kendi yazılabilecek uygulama adayları (Faz 7 sonrası):** dosya yöneticisi, GTK4 kurulum sihirbazı. Yazılım merkezi 23 Ekim'de 1.0 kapsamına alındı (K21). Sistem İzleyici, Yedekleme ve Diskler 23 Ekim'de K31 ile kendi uygulamalarımız olarak planlandı; kendi tarayıcı, güvenlik ve sürücü yöneticisi uygulaması yapılmaz.

18 Ekim 2026 resmî trixie/main amd64 indeksi kontrolü: bütün aday ve yedek paketler
mevcut. Bu, uygulamaların kurulup çalıştırıldığı anlamına gelmez.
[Sürümler ve indeks özeti](research/debian13-packages.json).

| Uygulama | Debian 13 paket adı |
|---|---|
| Nautilus | `nautilus` |
| Ptyxis / GNOME Console | `ptyxis` / `gnome-console` |
| GNOME Text Editor | `gnome-text-editor` |
| Loupe | `loupe` |
| Papers / Evince | `papers` / `evince` |
| Celluloid | `celluloid` |
| GNOME Calculator | `gnome-calculator` |
| GNOME System Monitor | `gnome-system-monitor` |
| File Roller | `file-roller` |
| Firefox ESR | `firefox-esr` (security deposundaki aday sürüm ayrıca Linux'ta toplanacak) |
