# Faz 6F — Yazılım mağazası ve paket arka ucu

## Bağlam
Önce oku: `CLAUDE.md`, `docs/PLANNING.md` (K08, K20, K21, K22), `docs/ARCHITECTURE.md`, `docs/DESIGN.md`, `docs/design/`, `docs/architecture/`, `docs/apps/` ve `prompts/06-dagitim.md`.

Kullanıcı 22 Ekim 2026'da PackageKit yerine Asena'nın kendi paket altyapısını seçti (K21) ve 23 Ekim'de mağazanın sürüm adayına girmesini istedi. Bu alt faz gizli duran Yazılım uygulamasını (`org.asenalinux.Store`) gerçek APT ve Flatpak işlemleriyle çalışır hale getirir. Mağaza olmadan kullanıcı arayüzden uygulama kuramaz; sürüm adayı bu alt fazı bekler.

## Hedef
Kullanıcı Yazılım'da Debian, Asena ve (açtıysa) Flathub uygulamalarını arar, ayrıntısını görür, kurar, kaldırır ve günceller. Sistem paketlerine dokunan her işlem polkit ile yetkilendirilir ve kullanıcıya önce değişiklik özeti gösterilir. Sistem güncellemeleri yeniden başlatma sırasında uygulanır; Flatpak güncellemeleri oturum içinde yapılır.

## Ön koşullar ve karar kapıları
- `rust-apt` (GPL-3.0-or-later) Debian 13'ün libapt-pkg 3.0 sürümü ve MSRV 1.85.1 ile VM'de derlenmeli. Derlenmezse dur ve sor; yerine `apt-get` alt süreci ve `APT::Status-Fd` önerilir.
- `libflatpak` 0.6 ve `libappstream` 0.3 (MIT, glib 0.20) kural 10'a göre denetlenir; daha yeni sürümler glib 0.21 istediği için kullanılmaz.
- Otomatik güvenlik güncellemeleri (önceki listede madde 13) bu alt fazda karara bağlanır: `unattended-upgrades` ile mağazanın yeniden başlatmada güncellemesi aynı APT kilidini paylaşır.
- Asena deposu (K08) yayımlanmadan Asena uygulamaları katalogda Debian paketleri gibi görünür; depo AppStream verisi 6B'de üretilir.

## Yapılacaklar

### A. Yetkili paket servisi (6F-A)
1. `crates/asena-packages`: sistem veri yolunda `org.asenalinux.Packages` adını alan, root olarak çalışan, D-Bus ile etkinleştirilen ve boşta kapanan servis. GTK içermez. Sözleşme `asena-core::dbus` içinde, ARCHITECTURE'da belgelenir.
2. Yöntemler: `Refresh`, `Plan(install, remove) → özet`, `Commit(plan_id)`, `ScheduleUpgrade`, `CancelUpgrade`; ilerleme ve sonuç sinyalleri. `Commit` yalnız aynı çağırana ait ve değişmemiş bir planı uygular; plan bayatsa reddeder.
3. Her yöntem polkit'e çağıranın sistem veri yolu adıyla (`system-bus-name` öznesi) sorulur. Eylemler `data/polkit/org.asenalinux.packages.policy` içinde: yenileme `allow_active=yes`, kur/kaldır/güncelle `auth_admin_keep`.
4. Paket adları Debian ad kuralıyla doğrulanır ve katalogda bulunmalıdır; APT'ye seçenek olarak yorumlanabilecek girdi geçmez. Temel paketleri veya `asena-desktop`'ı kaldıran planlar reddedilir.
5. Debconf soruları etkileşimsiz varsayılanlarla yanıtlanır; dpkg yarıda kalırsa servis bunu algılar ve onarımı (`dpkg --configure -a`) ayrı, yetkili bir eylem olarak sunar.

### B. Katalog (6F-B)
6. AppStream havuzu `libappstream` ile okunur: Debian DEP-11 (`appstream` paketinin APT kancası), Flathub ve Asena deposu. Arama `asena_core::text::fold`, görüntülenen listeler Türkçe yerel ayarına duyarlı sıralanır.
7. AppStream bileşeni olmayan paketler mağazada listelenmez; Yazılım bir paket yöneticisi değil uygulama mağazasıdır.
8. Ekran görüntüleri yalnız ayrıntı sayfası açılınca, ana iş parçacığını bloklamadan indirilir ve önbelleğe alınır.

### C. Flatpak (6F-C, K22)
9. Kullanıcı kurulumları `libflatpak` ile uygulama sürecinde yapılır; sistem kurulumları Flatpak'ın kendi polkit yardımcısını kullanır. Kurulumdan önce izinler (dosya sistemi, ağ, aygıtlar) gösterilir.
10. Flathub yalnız açık seçimle eklenir (Hoş geldin veya Yazılım ayarı); ağ hatasında tekrar dene ve atla vardır.
11. K22'nin diğer işleri bu alt fazda kapanır: `gnome-keyring` paket listesine girer; Background, Inhibit ve GlobalShortcuts portalları VM'de denetlenir, eksikler daemon arka ucu olarak eklenir; Asena simge teması Flatpak uzantısı ve adw-gtk3 Flatpak teması için yol belirlenir.

### D. Güncellemeler (6F-D)
12. `asena-daemon` günde bir `Refresh` ister ve güncelleme varsa tek bildirim gösterir. Ölçülü (metered) bağlantıda indirme yapılmaz.
13. Sistem güncellemeleri önce indirilir, sonra systemd `system-update.target` ile yeniden başlatmada uygulanır; başarısızlıkta sistem önceki paketlerle açılır ve sonuç bir sonraki oturumda gösterilir. XanMod çekirdeği (K20) aynı yoldan güncellenir.
14. Flatpak güncellemeleri oturum içinde uygulanır; çalışan uygulama yeniden başlatılınca yeni sürüme geçer.

### E. Ön yüz (6F-E)
15. Sayfalar: Keşfet (kategoriler), Arama, Kurulu, Güncellemeler, uygulama ayrıntısı. DESIGN'in metin ve düzen kuralları uygulanır; dolgu açıklama, rozet ve üçlü kart yok. Dar pencerede tek sütun.
16. Her yetkili işlemden önce kurulacak, kaldırılacak ve indirilecek boyut özeti gösterilir; ilerleme iptal edilebilir olduğu sürece iptal düğmesi vardır.
17. Kaynak gösterimi: Debian, Asena ve Flathub ayrı ve açık yazılır; aynı uygulama iki kaynakta varsa kullanıcı seçer.

## Uygulama sırası
`6F-A → 6F-B → 6F-C → 6F-D → 6F-E`. Servis sözleşmesi ve polkit politikası ön yüzden önce birim testleriyle biter.

## Kapsam dışı
- PackageKit arka ucu, Snap, AppImage, yorum/puan sistemi, ücretli içerik.
- Üçüncü taraf APT depolarını arayüzden ekleme.
- Asena deposunun dış yayını (K08, ayrı onay).

## Kabul kriterleri
- [ ] Yetkisiz çağıran kur/kaldır/güncelle yapamaz; yetkili kullanıcı parola sonrası `auth_admin_keep` süresinde tekrar sorulmaz.
- [ ] Temel paketleri veya `asena-desktop`'ı kaldıran plan reddedilir; seçenek benzeri paket adı reddedilir.
- [ ] VM'de Debian'dan küçük bir uygulama kurulur ve kaldırılır; işlem yarıda kesilince onarım yolu çalışır.
- [ ] Flathub açıkken bir Flatpak uygulaması kullanıcı kurulumu olarak kurulur, açılır, güncellenir ve kaldırılır.
- [ ] Yeniden başlatmada güncelleme VM'de uygulanır; başarısız paket senaryosunda sistem açılır ve sonuç gösterilir.
- [ ] Arama Türkçe karakterlerle doğru sonuç verir; listeler Türkçe sıralanır.
- [ ] Ana iş parçacığı bloklanmaz; servis yokken Yazılım çökmeden durumu gösterir.
- [ ] Yeni metinler çevrilebilir, `po/tr.po` güncel; ARCHITECTURE, APP_CATALOG ve APP_ROADMAP güncel.

## Doğrulama
- Birim testleri: paket adı doğrulama, plan karşılaştırma ve bayat plan reddi, polkit eylem eşlemesi, AppStream katalog birleştirme ve sıralama.
- VM: geçici bir test hesabıyla polkit akışı; anlık görüntülü test diskinde yeniden başlatmada güncelleme.
- Gerçek donanım ve gerçek ağ kesintisi senaryoları ayrıca raporlanır; yapılamayanlar açıkça yazılır.

## Teslim
Önce bu planın uygulama sırasını ve karar kapılarının sonucunu sun. Bitince `docs/reports/phase-06f.md`: denenenler, kanıtlar, açık kalanlar ve sonraki adaylar.
