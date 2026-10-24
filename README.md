# Asena Linux

Asena, Debian 13 tabanlı bir Linux dağıtımı ve Wayfire üzerinde çalışan
Rust, GTK4 ve libadwaita masaüstüdür. Alt panel, başlat menüsü, Ayarlar,
Yazılım ve kendi uygulamalarını içerir. Varsayılan tasarım dili Odak'tır.

[Web sitesi](https://asenalinux.netlify.app) · [Yol haritası](docs/ROADMAP.md) ·
[Katkıda bulunma](CONTRIBUTING.md)

## Geliştirme durumu

Proje geliştirme aşamasındadır. 19 uygulama ve 20 Ayarlar paneli kaynakta
bulunur. Son Linux VM kontrolleri ve sınırları [uygulama raporunda](docs/reports/apps-recovery.md)
ve [25 Ekim arayüz raporunda](docs/reports/ui-polish-2026-10-25.md) kayıtlıdır.
Donanım ve kullanıcı görsel kabulü tamamlanmamıştır.

Henüz indirilebilir Asena ISO'su, yayımlanmış Asena Debian paketleri veya
çalışan uzak Asena APT deposu yoktur. `iso/` canlı imaj hazırlığını taşır.
Kurulum ve kurtarma kabulü tamamlanmadan kullanılabilir sürüm ilan edilmez.

## Debian 13'te geliştirme oturumunu aç

Derleme ve masaüstü denemeleri Debian 13 VM'de veya uygun WSL2 ortamında yapılır.
Windows'ta Rust masaüstü derlenmez. Temiz bir Debian 13 geliştirme ortamında:

```sh
bash scripts/setup-dev.sh
export PATH=/usr/bin:/bin:$PATH
source scripts/native-env.sh
bash scripts/check-quality.sh
bash scripts/dev-session.sh
```

Kurulum betiği geliştirme bağımlılıklarını ve onaylı gtk4-layer-shell 1.3.0
backport'unu `target/native` altında hazırlar. Doğrudan Cargo komutlarından
önce `scripts/native-env.sh` yüklenir.

GPU yoksa `WLR_RENDERER_ALLOW_SOFTWARE=1 bash scripts/dev-session.sh` kullanın.
İki sanal çıkış için `WLR_WL_OUTPUTS=2 bash scripts/dev-session.sh` çalıştırın.
İç içe pencereyi kapatmak geliştirme oturumunu sonlandırır.

## Belgeler

- [Geliştirme ve VM denemeleri](docs/DEVELOPMENT.md)
- [Mimari](docs/ARCHITECTURE.md) ve [tasarım](docs/DESIGN.md)
- [Uygulama durumu](docs/apps/APP_ROADMAP.md)
- [GitHub deposunu yayımlama](docs/GITHUB.md)
- [Paket ve uygulama dağıtım planı](packaging/README.md)
- [Web sitesini geliştirme ve yayımlama](website/README.md)

## Lisans

Asena kaynak kodu [GPL-3.0-or-later](LICENSE) lisanslıdır. Duvar kâğıtlarının
kaynak ve lisans bilgileri [ayrı belgede](data/wallpapers/README.md) bulunur.
`vendor/` altındaki üçüncü taraf kod kendi lisans bildirimlerini korur.
