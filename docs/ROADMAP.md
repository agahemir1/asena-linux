# Asena — Yol Haritası

Her faz `prompts/` altındaki bir prompt dosyasıyla başlar ve kabul kriterleri sağlanınca kapanır.
Bir fazın bitmesi, sonraki fazın ihtiyaç duyduğu zeminin hazır olması demektir.

**Mevcut aşama, 25 Ekim 2026:** 19 uygulama, 20 Ayarlar paneli ve Yazılım'ın
paket işlemleri kaynakta; son VM kontrolleri raporlarda kayıtlıdır.
GitHub `asena-linux` deposu için yerel dosyalar ve ilk yükleme rehberi hazırlandı.
[Uzak depo](https://github.com/agahemir1/asena-linux) kullanıcı tarafından açıldı;
ilk kaynak yüklemesi tamamlandı ve uzak `main` doğrulandı.
Debian paketleri, imzalı Asena APT deposu, gerçek ISO
ve Calamares kurulumu henüz yoktur. Görsel, fiziksel donanım ve normal giriş
kabulü açık tutulur. GitHub Actions yalnız elle başlatılacak şekilde hazırdır;
dış koşu yapılmadı. [Uygulama raporu](reports/apps-recovery.md),
[arayüz raporu](reports/ui-polish-2026-10-25.md),
[GitHub hazırlığı](reports/github-preparation.md).

**T0:** [Odak tasarımı](../prompts/T0-tasarim.md) seçildi; blur yolu onaylı
Wayfire'dır. Uygulama simgelerinin ve nihai ekranların kullanıcı görsel kabulü,
gerçek GPU ölçümleri açıktır.

| Faz | Başlık | Prompt | Çıktı | Durum |
|---|---|---|---|---|
| 0 | Geliştirme ortamı ve iskelet | [00-altyapi.md](../prompts/00-altyapi.md) | Yerel kalite ve iç içe panel geçti; fiziksel hotplug/normal giriş/CI açık | 🟨 |
| 1 | Panel ve görev çubuğu | [01-panel.md](../prompts/01-panel.md) | Toplevel/görev çubuğu/CLI kaynakları ve yerel kontroller mevcut; elle kabul açık | 🟨 |
| 2 | Başlat menüsü, bildirimler, OSD | [02-menu-bildirim.md](../prompts/02-menu-bildirim.md) | Menü/arama, Notifications 1.2, merkez ve OSD kaynakları; doğrulama sürüyor | 🟨 |
| 3 | Hızlı ayarlar ve sistem servisleri | [03-hizli-ayarlar.md](../prompts/03-hizli-ayarlar.md) | Ses/ağ/güç, medya/tepsi ve daemon kaynakları; 34 Rust + 8 Python testi geçti, donanım/görsel kabul açık | 🟨 |
| 4 | Oturum ve güvenlik | [04-oturum-guvenlik.md](../prompts/04-oturum-guvenlik.md) | Kilit/PAM, idle, greeter ve polkit ajanı; 56 Rust + 8 Python testi geçti, gerçek giriş/uyku açık | 🟨 |
| 5 | Ayarlar Merkezi | [05-ayarlar.md](../prompts/05-ayarlar.md) | 20 panel kaynakta; klavye/fare/dil daemon'la uygulanıyor; ekran düzeni (5D) VM'de geçti, gerçek monitörler açık; görsel kabul açık | 🟨 |
| 6 | Dağıtım | [06-dagitim.md](../prompts/06-dagitim.md), [06f-yazilim.md](../prompts/06f-yazilim.md) | Depo git'te (22 Ekim); `iso/` live-build iskeleti ve `scripts/build-iso.sh` hazır, ISO derlenmedi. 23 Ekim: XanMod çekirdeği, markalama ve sistem yapısı kararları VM önyükleme testinde geçti; Yazılım ve paket servisi (6F) VM'de çalışıyor ([rapor](reports/phase-06f.md)). .deb paketleri, APT deposu, Calamares ve Hoş geldin başlamadı | 🟨 |
| 7 | Cila | [07-cila.md](../prompts/07-cila.md) | Ekran görüntüsü portal yoluyla erken eklendi (21 Ekim); erişilebilirlik ve performans başlamadı | 🟨 |

Durum: ⬜ başlamadı · 🟨 sürüyor · ✅ bitti

## Kilometre taşları
- **M1 — "Günlük kullanılabilir" (Faz 0–3):** Asena oturumunda panel, menü, bildirimler ve hızlı ayarlarla gün boyu çalışılabilir. Giriş ekranı ve kilit henüz başka araçlarla sağlanır; geçici kilitleyici ayrıca doğrulanmadan kilitli günlük kullanım kabulü verilmez.
- **M2 — "Tam masaüstü" (Faz 4–5):** Giriş ekranından kapanışa kadar her şey Asena'nın.
- **M3 — "Dağıtım" (Faz 6):** ISO'dan kurulup test deposundan güncellenebilen Asena Linux sürüm adayı. Dış yayın ayrıca kararlaştırılır.
- **M4 — "1.0" (Faz 7):** Performans bütçeleri ve erişilebilirlik hedefleri karşılanmış sürüm.

## Planlanan uygulama sırası

| Faz | Alt teslimler |
|---|---|
| 0 | Ortam/protokol doğrulama → workspace ve ortak çekirdek → boş panel/oturum → CI |
| 1 | Toplevel ve uygulama eşleştirme → görev çubuğu → saat/arka plan → CLI/monitörler |
| 2 | Menü/arama → bildirim sunucusu → popup/merkez → OSD |
| 3 | 3A servis modelleri → 3B ses/güç/parlaklık → 3C ağ/BT → 3D hızlı ayarlar/medya/tepsi → 3E daemon |
| 4 | 4A kilit/PAM → 4B boşta kalma/uyku → 4C greetd → 4D polkit/oturum |
| 5 | 5A uygulama/arama → 5B tercihler → 5C bağlantılar/ses → 5D ekran geri alma → 5E sistem |
| 6 | 6A paketler → 6B test deposu → 6C canlı ISO → 6D kurulum → 6E Hoş geldin/belgeler; 6F Yazılım ve paket servisi sürüm adayına girer; kurtarma bölümü ve sıfırlama (K30) 6D'nin bölüm düzeniyle gelir |
| 7 | 7A ekran görüntüsü → 7B erişilebilirlik → 7C performans → 7D paketlenmiş sürüm kapanışı |

Pano geçmişi 1.0 kapsamı dışındadır; Faz 7'nin pano işi görüntüyü kopyalamaktır. Bir faz, kabul kriterleri ve gerekli donanım/ortam kanıtları tamamlanmadan ✅ yapılmaz. Raporlar uygulama başladığında `docs/reports/phase-NN.md` altında tutulur.

## Faz notları
Her fazın sonunda buraya 2–3 satırlık not eklenir: alınan önemli kararlar, ertelenenler, bilinen sorunlar.

<!-- Faz 0 notları -->
18 Ekim 2026: 14 crate, ortak çekirdek, monitör başına boş panel, oturum/geliştirme betikleri, veri dosyaları ve Debian CI oluşturuldu. Resmî paket indeksi ve doğrudan crate manifestleri incelendi; taşınabilir kaynak kontrolleri geçti.
WSL yok; VirtualBox'taki tek VM Windows XP ve kapalı. Linux build/fmt/clippy/test, `wayland-info`, panel ve süreç temizliği deneyi ile CI sonucu doğrulanmadı; Faz 0 açık. K03 session-lock, K13 blur ve K14 son tema/simgeler de açık.
19 Ekim 2026: QEMU 11.1.2 ve Debian 13.7 genericcloud amd64 VM'si hazırlandı, SSH bağlantısı doğrulandı. 6 GB RAM/4 vCPU/32 GB overlay kullanılıyor; geliştirme paketlerinin kurulumu ve Linux kabulü sürüyor. Kullanıcı şimdilik yerel Linux kontrolleriyle ilerlemeyi seçti.

19 Ekim yerel sonuç: Debian kalite kapısı geçti; maximized foot paneli örtmedi, %200 ölçek ve süreç/ortam temizliği doğrulandı. Fiziksel hotplug, normal giriş ve dış CI açık. Faz 1 veri/IPC uygulaması başladı; K13 çözülmedi, gerçek blur tamamlanmış değil.

19 Ekim ek ilerleme: Faz 2 hesap makinesi/pano, Türkçe arama/başlatma ve bildirim
protokol denemeleri geçti. Faz 3 native ses ve servis cache deneyleri geçti;
MPRIS/tepsi/daemon kaynakları ve protokol deneyleri eklendi; donanım/görsel kabul
açık. [Faz 3 raporu](reports/phase-03.md).

19 Ekim Faz 4: C 1.3.0 ayrı prefix'te derlendi; 53 upstream test ve Asena panel
regresyonu geçti. Debian 1.0.4 paketi korunuyor. Kilit/PAM güvenlik deneyi sürüyor;
otomatik kilit/uyku, greetd ve polkit kabulü tamamlanmadı.

20 Ekim: 44 Rust + 8 Python testi; iki çıkışlı ve Türkçe tek çıkışlı PAM/kilit
deneyi geçti. SIGKILL sonrası ekran siyah kaldı, yeni kilitleyici de PAM istedi.
Tek swayidle, süre değişiminde temizlik, otomatik kilit ve uyku bekletmesi deneyi geçti; otomatik eylemler varsayılan kapalı. Hotplug, fiziksel
klavye ve gerçek oturum/uyku kabulü açık. [Faz 4 raporu](reports/phase-04.md).

20 Ekim ek sonuç: greetd protokol worker ve giriş arayüzü eklendi; 51 Rust + 8 Python testi geçti. Ayrıcalıksız protokol sunucusunda normal ve Türkçe 640×480/22 px akış geçti. Gerçek greetd/cage/TTY, iki kullanıcı girişi ve polkit kabulü açık.

20 Ekim Faz 4D & Faz 5: Polkit yetkilendirme ajanı (`crates/asena-polkit-agent`), 15 MVP paneli içeren Ayarlar Merkezi (`crates/asena-settings`), D-Bus `OpenPanel` sözleşmesi ve Blueprint arayüzü tamamlandı. 59 Rust + 8 Python = 67 testin tamamı kalite kapısından geçti. Canlı VNC önizlemesi (labwc + asena-shell + asena-settings + wayvnc/websockify) `127.0.0.1:22380` portunda ayağa kaldırıldı. Canlı ISO üretimi kullanıcı kararıyla daha sonraki aşamalara ertelendi. [Faz 5 raporu](reports/phase-05.md).

21 Ekim denetimi: uygulamaların hepsi açılışta panikliyordu ve 15'i maketti; 20 Ekim uygulama
ekosistemi raporu geçersiz sayıldı. Çatı düzeltildi, 11 uygulama gerçek veriyle yeniden yazıldı, 5'i
gizlendi. Shell yüzeyleri (durum kümesi, Odak menüsü, denetim merkezi, bildirim kenar paneli, masaüstü)
yeniden kuruldu; kendi çizgi simge seti ve 4 hacimli inceleme simgesi eklendi. Geliştirme oturumunda portal
ortamı düzeltildi (Firefox diyalogları). Gerçek donanım, gerçek giriş ve görsel kabul açık.
[Denetim raporu](reports/audit-2026-10-21.md).

21 Ekim K13 deneyi: Wayfire 0.9.0 Dual Kawase blur'u panel, menü, hızlı ayarlar ve kenar panelinin
arkasındaki pencereleri VM'de bulanıklaştırdı; hareket sırasında yazılım renderer'da CPU yaklaşık 1,5 kat.
[Sonuç](research/blur-options.md).

21 Ekim compositor geçişi: kullanıcı onayıyla tek compositor Wayfire 0.9.0 oldu, labwc kaldırıldı.
Daemon `wayfire.ini` yazar; Saydamlık efektleri tercihi cam malzemesini ve blur eklentisini açıp kapatır;
önizleme ve test düzenekleri headless Wayfire kullanır. VM'de smoke, kilit, greeter ve Flathub
uygulaması (menüden açılış, dosya seçici ve belge portalı) geçti; gerçek GPU ve normal giriş açık.
[Geçiş raporu](reports/wayfire-migration.md).

22 Ekim Faz 5D: Ekranlar sayfası düzeni değiştirir; test, uygulama, 15 sn onay ve geri almanın tek
sahibi daemon'dır. Ayarlar kapanınca veya ekranlar değişince geri alınır; onaylı düzen ekran takımına göre
saklanır. GTK 4.18'in ölçek dönüşünde bayat ekran geometrisi ve wlroots 0.18'in modsuz çıkışta düşmesi
bulundu; shell geometriyi compositor'dan okur. Gerçek monitör ve takıp çıkarma açık.
[Faz 5 raporu](reports/phase-05.md).

22 Ekim vurgu ve klavye: daemon Flatpak uygulamaları için Settings portal arka ucu sunar (koyu tema ve
vurgu canlı); klasörler ile Dosyalar ve Ayarlar simgeleri vurguyu izler (`Asena-<vurgu>` temaları, simge
incelemesi açık); harf kısayolları ilk klavye düzeninde o harfi yazan tuşa taşınır (TR-F). wlroots 0.18'in
modsuz çıkış çökmesine yukarı akış yaması `0.18.2-3+asena1` paketiyle VM'de doğrulandı (K17).

## 23 Ekim 2026 uygulamalar ve sıfırlama devamı

Claude oturumu `059b1f01-e91b-4ea5-84d5-e042b896d9cc` kaynakları tamamlandı; 24 Ekim’de 215 Rust + 32 Python testi, derleme/Clippy/fmt ve çalışan oturum açılışları geçti: 19 uygulama, Ayarlar için üç yeni panel, Yazılım paket dosyası açma ve K30 kurtarma servisi. Faz 6D/ISO tamamlandı sayılmaz; kurulum bölüm düzeni, gerçek kurtarma açılışı ve Secure Boot kabulü açıktır. Durum ve kanıt [apps-recovery raporundadır](reports/apps-recovery.md).

## 25 Ekim 2026 kaynak deposu hazırlığı

Kullanıcı GitHub deposu için dosyaların hazırlanmasını istedi ve `asena-linux`
adını uygun buldu. Site `asenalinux.netlify.app` olarak korundu. README,
katkı rehberi, issue ve pull request şablonları, yayın dosyası/geçmiş denetimi,
ilk yükleme rehberi ve paket dağıtım planı eklendi. Manuel Debian CI hazır;
GitHub'da çalıştırılmadı. Faz 6A/6B tamamlandı sayılmaz.
[Hazırlık raporu](reports/github-preparation.md).

Aynı gün kullanıcı uzak adresi paylaştı; güncel uygulama kaynakları ve GitHub
hazırlığı ayrı commit'lerle `main` dalına yüklendi. Geçici aktarım etiketleri
temizlendi. Dış CI elle başlatılmayı bekler.
