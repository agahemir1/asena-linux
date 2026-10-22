# K13 için sonraki teknik adım

19 Ekim 2026. Kullanıcı yalıtılmış VM deneyini seçti. Varsayılan compositor değişikliği yapılmadı.

Mevcut Debian labwc 0.8.3 üzerinde panel, toplevel, IPC ve arka plan çalışıyor.
Gerçek blur için render katmanında ek destek gerekli. Shell kodu labwc API'si
kullanmıyor; aynı layer-shell ve foreign-toplevel protokollerini kullanan başka
bir compositor ile işlevleri yeniden sınamak mümkün.

| Seçenek | Somut çalışma | Sınır |
|---|---|---|
| Yalıtılmış Wayfire deneyi | Debian paketini yalnız geliştirme VM'sinde ayrı config ile çalıştır; mevcut Asena binary'si, panel/popup blur, protokoller, ölçek ve kaynak kullanımını karşılaştır | Varsayılan oturum değiştirilmez. Deney geçmeden üretim seçimi yapılmaz |
| labwc yaması | Kapanmış upstream çalışmasının uygulanabilirliğini ve render değişikliğini incele; sınırlı backport maliyeti çıkar | Asena'nın compositor yaması ve güvenlik güncellemeleri için bakım sorumluluğu doğar; şu an hazır doğrulanmış yama yok |
| Kararı açık tut | Compositor bağımsız modeller ve opak geliştirme görünümü üzerinde devam et | Gerçek cam ve görsel faz kabulü tamamlanamaz |

Öneri: önce yalıtılmış Wayfire deneyi. Bu, varsayılanı değiştirmeden çalışan bir
karşılaştırma üretir. Debian trixie'de [Wayfire 0.9.0-5](https://packages.debian.org/trixie/wayfire)
bulunuyor. Aynı v0.9.0 etiketinde [blur render eklentisi](https://github.com/WayfireWM/wayfire/blob/v0.9.0/plugins/blur/blur.cpp),
[foreign-toplevel](https://github.com/WayfireWM/wayfire/blob/v0.9.0/plugins/protocols/foreign-toplevel.cpp)
ve [session-lock](https://github.com/WayfireWM/wayfire/blob/v0.9.0/plugins/protocols/session-lock.cpp)
kaynakları mevcut. Bunlar kaynak ön incelemesidir; Asena GTK yüzeylerinde çalışan
blur veya güvenli kilit kanıtı değildir. Debian GTK layer-shell 1.0.4'ün eksik
session-lock istemci API'sini değiştirmez.

Deneyde gerekli kanıtlar: panel arkasında hareketli pencere, yazı/simgelerin net
kalması, açık/koyu ve opak geri dönüş, iki çıkış/%200 ölçek, toplevel istekleri,
session-lock protokol envanteri ve ayrı güvenlik incelemesi, GPU/yazılım renderer
sınırları, boşta CPU/RSS ve kare süreleri. VM'nin yazılım renderer sonucu gerçek
entegre GPU kabulü sayılmaz. Sonuç sonrasında varsayılan compositor için ayrı
mimari karar gerekir.

İlk VM çalıştırmasında cloud çekirdeği DRM aygıtı sunmadığı için GLES başlamadı.
Sonraki generic çekirdekle bochs `/dev/dri/card0` mevcut. Önceki kısmi görüntüde
desen yumuşaması görüldü; orta tıkla kapatma A/B görüntüleri aynı olduğundan bu
kontrollü blur kanıtı sayılmadı. Deney betiği iki ayrı compositor başlangıcına,
ikincisinde blur eklentisinin tamamen kaldırılmasına göre düzeltildi.

20 Ekim son tekrarında GLES/GBM `DRM_IOCTL_MODE_CREATE_DUMB: Permission denied`
verdi; iki headless çıkış enabled=false kaldı ve screencopy başarısız oldu.
Normal kullanıcıyla kontrollü A/B henüz geçmedi. Eski görüntüler bu son koşunun
başarısı değildir. Betik önceki görüntüleri yeni denemeden önce temizler.
TTY/DRM oturumunda render erişimi ayrıca araştırılacak; varsayılan labwc korundu.

## 21 Ekim 2026: Dual Kawase

Kullanıcı Dual Kawase blur'u sordu. Dual Kawase, görüntüyü küçültüp büyütürken her
geçişte birkaç örnek alan ucuz ve kaliteli bir blur algoritmasıdır. Compositor'ün
render geçişinde çalışır. Panel bir Wayland istemcisi olduğu için arkasındaki
pencerelerin piksellerini okuyamaz. GTK CSS ile ya da shell kodunda yapılamaz.

| Yol | Kaynakta görülen | Asena için anlamı |
|---|---|---|
| Wayfire 0.9.0 blur eklentisi | [metadata/blur.xml](https://github.com/WayfireWM/wayfire/blob/v0.9.0/metadata/blur.xml): `method` seçenekleri box, gaussian, kawase, bokeh; varsayılan kawase (`kawase_offset` 1.7, `kawase_degrade` 3, `kawase_iterations` 2). Hangi yüzeylerin bulanıklaşacağını `blur_by_default` ölçütü seçer, varsayılanı `type is "toplevel"` | Debian 13 paketinde var, yama bakımı gerekmez. Onaylı VM deneyi bu yolu sınar. Layer-shell panelin ölçütle eşleştiği deneyde gösterilmeli |
| labwc + SceneFX | [SceneFX](https://github.com/wlrfx/scenefx) `blur1.frag`/`blur2.frag` Dual Kawase küçültme/büyütme geçişleridir. SceneFX 0.5.0 wlroots `>=0.20.0, <0.21.0` ister. README'nin işaret ettiği labwc yaması [labwc-blur](https://github.com/grigio/labwc/releases) olarak dağıtılıyor: "labwc 0.20.2 + ext-background-effect-v1 yama yığını", yalnız Arch paketi. Upstream [PR #3751](https://github.com/labwc/labwc/pull/3751) kapandı, birleşmedi | Debian 13'te labwc 0.8.3 ve wlroots 0.18 var. Bu yol üç bileşeni (labwc, wlroots, SceneFX) Asena'nın paketleyip güvenlik güncellemelerini taşıması demektir. Compositor değişikliğidir |
| Duvar kâğıdı blur'u (statik) | Shell, duvar kâğıdının bulanık kopyasını panel ve menünün arkasına çizebilir | Compositor gerektirmez ama arkadaki pencereleri bulanıklaştırmaz. K13 kabulünü kapatmaz |

[ext-background-effect-v1](https://wayland.app/protocols/ext-background-effect-v1) Mayıs 2025'te
wayland-protocols'a staging olarak girdi. İstemcinin blur bölgesini istemesini sağlar
ama compositor uygulamadıkça etkisi yoktur. GTK 4.18 bu protokolü göndermez.

Öneri değişmedi: önce Wayfire deneyi `method = kawase` ile tamamlanır. Kalan engel VM'deki
GLES/DRM render izni; yazılım renderer sonucu GPU performans kabulü sayılmaz.

## 21 Ekim 2026: Wayfire Dual Kawase VM deneyi

Kullanıcı deneyin Kawase ile tamamlanmasını onayladı. Varsayılan oturum labwc olarak kaldı;
Wayfire yalnız geçici çalışma dizininde, özel D-Bus ile çalıştı.

**Render engelinin nedeni.** VM'ye sonradan `vgem` modülü yüklenmiş ve `/dev/dri/card0` vgem
olmuştu. wlroots ayırıcısı vgem'in render düğümüne geçiyor, llvmpipe orada dumb buffer
oluşturamıyordu (`DRM_IOCTL_MODE_CREATE_DUMB: Permission denied`). Betik artık ilk PCI
görüntü aygıtını (bochs, `card1`) seçiyor. Wayfire headless arka uçla GLES2/llvmpipe üzerinde
iki çıkışı açtı ve screencopy çalıştı. Ayrıca Wayfire 0.9 çıkış bölümlerini `[output:İSİM]`
biçiminde bekliyor; eski `[HEADLESS-1]` bölümleri hiç uygulanmıyordu.

**Kurulum.** [run.sh](../../experiments/wayfire/run.sh) aynı Asena shell binary'sini iki kez
başlatır: `blur` eklentisiyle ve eklenti çıkarılmış aynı yapılandırmayla. Blur kuralı
layer-shell ad alanlarıyla eşleşir (`asena-panel`, `asena-menu`, `asena-quick-settings`,
`asena-notification-center`). Wayfire layer yüzeyinin `app_id` değerini ad alanından aldığı
için shell kodu değişmedi. Test malzemesi panel ve yüzeylerde %60 opaklıktır
([glass.css](../../experiments/wayfire/glass.css)). Arkada 16 piksellik siyah/beyaz şerit
duvar kâğıdı ve tam genişlikte blok desenli bir foot penceresi vardır. Pencere Wayfire IPC ile
panelin altına taşınır, sonra menü, hızlı ayarlar ve kenar paneli onun üstünde açılır.
Ölçüm ve IPC yardımcısı [probe.py](../../experiments/wayfire/probe.py) yalnız standart
kitaplık kullanır.

**Ölçüt.** Yüzey içeriği olmayan bölgelerde yatay komşu pikseller arası parlaklık farkının
karekök ortalaması. Keskin şerit yüksek, bulanık arka plan düşük değer verir. İki
görüntüde yüzey içeriği aynı olduğu için fark arka plandan gelir.

| Bölge | Blur açık | Blur kapalı |
|---|---|---|
| Panel, duvar kâğıdı üstünde | 2,04 | 25,89 |
| Panel, arkasında pencere | 2,71 | 16,09 |
| Başlat menüsü, pencere üstünde | 2,78 | 16,86 |
| Hızlı ayarlar, pencere üstünde | 2,75 | 16,34 |
| Bildirim kenar paneli, pencere üstünde | 2,68 | 15,49 |
| Panel, %200 ölçekli çıkış | 2,03 | 25,59 |

Görüntüler ve ham değerler: [evidence/k13-wayfire](../reports/evidence/k13-wayfire/)
(örnek: [menü açık](../reports/evidence/k13-wayfire/menu-on.png),
[menü kapalı](../reports/evidence/k13-wayfire/menu-off.png)).

**Kaynak kullanımı** (Wayfire süreci, llvmpipe, 4 vCPU VM, üç koşu):

| Durum | Blur açık | Blur kapalı |
|---|---|---|
| Boşta, 5 saniye | %0,0 CPU | %0,0 CPU |
| Pencere panelin arkasında 60 adımda hareket ederken (~3 saniye) | %44 ile %56 CPU | %28 ile %33 CPU |
| RSS | ~170 MB | ~170 MB |

Boşta ek maliyet görülmedi; blur yalnız hasar olan karelerde çalışıyor. Hareket sırasında
yazılım renderer'da CPU yaklaşık 1,5 katına çıktı. Bu VM ölçümü GPU kabulü değildir.

**İşlev ve protokoller.** Toplevel denemesi (simge durumuna küçült, etkinleştir, kapat) geçti.
Asena'nın kullandığı protokollerin hepsi Wayfire'da var: layer-shell, foreign-toplevel,
ext-session-lock, xdg-activation (`xdg-activation` eklentisiyle), screencopy, idle-notify,
output-management, output-power, gamma-control, fractional-scale. labwc'de olup bu Wayfire
yapılandırmasında olmayanlar: ext-workspace, ext-foreign-toplevel-list, cursor-shape,
security-context, tearing-control, input-method-v2/text-input-v3. Asena şu an bunları
kullanmıyor; security-context Flatpak yalıtımı, input-method girdi yöntemleri için ileride
gerekir. Ayrıntı: [protocols-compare.txt](../reports/evidence/k13-wayfire/protocols-compare.txt).

**Gözlemler.**
- Varsayılan Kawase değerleri (`offset` 1,7, `degrade` 3, `iterations` 2) orta güçte blur
  veriyor; geniş şeritler yumuşak bant olarak seçiliyor. Buzlu cam hedefi için daha güçlü
  değerler ve bunların maliyeti ayrıca ölçülmeli.
- Blur fiziksel piksel üzerinden hesaplanıyor. %200 ölçekte mantıksal yarıçap yarıya iniyor;
  Wayfire'da ölçeğe göre ayrı ayar yok.
- Wayfire karıştırma shader'ı yüzeyin alfa değerini kullanıyor: tamamen saydam alanlar
  bulanıklaşmıyor, düşük alfalı gölge kenarları kısmen bulanıklaşıyor. Gölgeler bu malzemeye
  göre ayarlanmalı.
- Blur kapalıyken %60 opaklıkta menü ve hızlı ayarlar metni şeritler üstünde zor okunuyor.
  Opak geri dönüş için ayrı opaklık değeri gerekir.
- Pencere başlıklarını Wayfire'ın `decoration` eklentisi çiziyor; labwc'deki Asena başlık
  teması burada yok.
- Yüzeyler açılıp kapanırken wlroots `A configure is sent to an uninitialized
  wlr_layer_surface_v1` günlüğü yazıyor; yüzeyler doğru çalıştı, neden araştırılmadı.
- Shell menü ve panelleri her zaman GDK'nın ilk monitöründe açıyor; tıklanan panelin
  monitöründe açmıyor. Bu labwc'de de geçerli bir çoklu monitör hatasıdır.

**Doğrulanmayanlar.** Gerçek GPU ve DRM/TTY oturumu, kare süreleri, gerçek giriş aygıtları,
session-lock davranışı ve güvenlik incelemesi (yalnız protokolün varlığı görüldü), greetd
oturum kaydı. Daemon'un labwc modülü (klavye, fare, `rc.xml`, ortam) ile `data/labwc`
kısayol/otomatik başlatma dosyalarının Wayfire karşılığı yok.

**Karar durumu.** Deney, Dual Kawase'in Asena shell'i değiştirmeden Debian 13'ün Wayfire
paketiyle çalıştığını gösterdi. Varsayılan compositor'ü değiştirmek ayrı mimari karardır
ve kullanıcı onayı bekler. Değişiklik onaylanırsa gereken işler: daemon'a Wayfire modülü ve
`data/wayfire`, pencere başlığı teması, kısayollar, gerçek GPU ölçümü ve session-lock
güvenlik incelemesi.
