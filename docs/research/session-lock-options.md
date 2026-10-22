# K03: Faz 4 GTK kilidi için dar API kararı

19 Ekim 2026. Kullanıcı A seçeneğini onayladı: yalnız C 1.3.0 backport, önce VM doğrulaması.
VM'de `target/native/gtk4-layer-shell-1.3.0` prefix'ine kuruldu; Debian'ın 1.0.4
paketi ve varsayılan compositor korunuyor. 53 upstream test ve Asena panel/servis
regresyonu geçti. Kaynak SHA256:
`1ebb01ab14e98afd1727f68f64981c37bd23305b1f131f5667c02b94cf593192`.
MIT lisansı kaynakla birlikte saklanır. Tekrar üretim:
`scripts/backport-layer-shell.sh`, etkinleştirme: `source scripts/native-env.sh`.
Ürün .deb backport paketi henüz üretilmedi; bu sonuç kilit/PAM kabulü değildir.

## Doğrulanan uyumluluk

- Debian C kitaplığı gtk4-layer-shell 1.0.4 session-lock API'sini içermiyor.
  labwc 0.8.3 gerçek protokol envanterinde ext-session-lock-v1 ilan ediyor.
- Upstream 1.1.0 kilit API'sini ekledi; 1.2.0 monitör yaşam döngüsünü değiştirdi;
  1.3.0 başarısız kilit isteğinde use-after-free düzeltmesi içeriyor.
  [Değişiklik günlüğü](https://github.com/wmww/gtk4-layer-shell/blob/main/CHANGELOG.md),
  [1.3.0](https://github.com/wmww/gtk4-layer-shell/releases/tag/v1.3.0).
- gtk4-session-lock **0.1.2**: GTK/GDK 0.9, GLib 0.20, MSRV 1.70, MIT.
  Mevcut workspace ailesine uyan sürüm bu. Daha yeni 0.2/0.3 GTK 0.10;
  0.4 GTK 0.11 gerektiriyor. En yeni binding'e toplu geçiş önerilmiyor.
- gtk4-session-lock-sys 0.1.1, gtk4-layer-shell-0 pkg-config sürümünü yalnız
  `>=1` denetliyor. Bu yetersizdir; Asena kilit derlemesinde ayrıca `>=1.3.0`
  zorunlu tutulmalı. Cargo'nun link aşamasına bırakılmamalı.

## Somut seçenekler

| Seçenek | Değişiklik | Kapsam / bedel |
|---|---|---|
| A: dar backport | Yalnız gtk4-layer-shell C tabanını 1.3.0'a yükselt; GTK 4.18, adwaita 1.7 ve Rust 1.85.1 kalsın. Lock için Rust 0.1.2 binding ve açık native minimum denetimi kullan. | Debian 13 için ayrı paket/bağımlılık bakımı; VM'de önce yalıtılmış kurulum ve kilit hata/monitör/çökme testi gerekir. C kitaplığı normal paneli de etkilediğinden panel regresyonu zorunlu. |
| B: sınırı koru | 1.0.4 kalsın; geçici swaylock istemcisi ve hazır onayıyla koordinasyon uygulansın. | Özgün GTK Asena kilidi tamamlandı sayılamaz. swaylock'un PAM ve hazır FD davranışı ayrıca doğrulanır; kendi kilit ekranı daha sonraki karara kalır. |

Öneri A. Bu, labwc'den Wayfire'a geçiş değildir. GNOME/Mutter'a geçiş, yeni GTK
API'si veya bütün Rust GTK ailesini yükseltme önerilmez. PAM/polkit ve kilit hazır
durum makinesi yine ayrı güvenlik incelemesi ve VM kabulü gerektirir.

## Onaydan sonraki uygulama sınırı

Önce kaynak/paket checksum ve lisans kaydı, ayrı VM geri dönüş noktası, 1.3.0
derlemesi/panel regresyonu; ardından lock binding'i ve PAM. Sistem display manager
otomatik değiştirilmez. Tüm ekranların kapanması, monitor hotplug, yanlış parola,
kilitleyici çökmesi ve uykudan dönüş kanıtı olmadan kilit/uyku etkinleştirilmez.
