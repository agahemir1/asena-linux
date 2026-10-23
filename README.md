# Asena Linux

Debian 13 üzerinde Rust, GTK4/libadwaita ve Wayfire kullanan Wayland masaüstü.
Varsayılan tasarım dili Odak. Faz 0 kaynakları oluşturuldu; Linux kabulü bekleniyor.

Debian 13 VM veya WSL2/WSLg içinde:

```sh
bash scripts/setup-dev.sh
bash scripts/compile-data.sh
export GSETTINGS_SCHEMA_DIR="$PWD/target/schemas"
cargo build --workspace
bash scripts/check-quality.sh
bash scripts/dev-session.sh
```

Debian derleyicisini seçmek için gerekirse `export PATH=/usr/bin:/bin:$PATH` kullanın.
Wayfire OpenGL ES ister; GPU yoksa `WLR_RENDERER_ALLOW_SOFTWARE=1 bash scripts/dev-session.sh` deneyin.
İki sanal çıkış: `WLR_WL_OUTPUTS=2 bash scripts/dev-session.sh`.
İç içe pencereyi kapatmak oturumu sonlandırır; mevcut Wayfire ayarlarınız kullanılmaz.

[Geliştirme](docs/DEVELOPMENT.md), [Faz 0 raporu](docs/reports/phase-00.md),
[yol haritası](docs/ROADMAP.md), [tasarım](docs/DESIGN.md).

Web sitesi `website/` altında ([website/README.md](website/README.md)); adresi
`asenalinux.netlify.app` olacak, henüz yayımlanmadı.
GPL-3.0-or-later; [lisans](LICENSE).
