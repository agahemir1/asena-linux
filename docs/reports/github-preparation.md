# GitHub kaynak deposu hazırlığı, 25 Ekim 2026

Kullanıcı `asenalinux.netlify.app` sitesini kullanarak GitHub deposu için
dosyaların hazırlanmasını istedi. Depo adı `asena-linux` olarak önerildi ve
uygun bulundu. Ardından kullanıcı [agahemir1/asena-linux](https://github.com/agahemir1/asena-linux)
adresini paylaştı. Uzak depo boş olarak doğrulandı ve `origin` bu adrese bağlandı.

## Hazırlanan dosyalar

- README gerçek geliştirme durumunu, Debian başlangıç adımlarını ve mevcut siteyi gösterir.
- `CONTRIBUTING.md`, issue formları ve pull request şablonu katkı akışını tanımlar.
- `.gitignore` yerel hesap/ajan ayarlarını, ortam dosyalarını, özel anahtarları ve disk imajlarını dışarıda tutar.
- `scripts/check-publication.py` kaynak adaylarını ve `HEAD`, dallar ve etiketlerden erişilen Git geçmişini denetler.
- `docs/GITHUB.md` boş GitHub deposunu oluşturma, mevcut kaynakları commit etme ve ilk yükleme adımlarını verir.
- `packaging/README.md` ayrı Debian uygulama paketleri, AppStream kataloğu, yerel imzalı arşiv ve sonraki Flatpak kapsamını planlar.
- Debian iş akışı yalnız `workflow_dispatch` ile çalışır; ilk yükleme otomatik dış koşu başlatmaz.

Yayın denetimi bilinen özel anahtar/token kalıplarını ve 100 MiB sınırını
kontrol eder; gizli değerleri yazdırmaz. Testler, silinmiş bir sırrın eski
commit'te bulunmasını ve ignore edilmiş ama izlenen yerel dosyanın
reddedilmesini de kapsar. Araçların özel Git referansları yayımlanacak dal
geçmişinden ayrıdır ve bu denetimin dışındadır. Rehber yalnız `main` yükler;
`--mirror` kullanılmaz. Denetim bütün olası sırların yokluğunu kanıtlamaz.

## Doğrulama

Windows'ta yalnız taşınabilir kaynak/dosya kontrolleri çalıştırıldı:

| Kontrol | Sonuç |
|---|---|
| Yayın adayları ve Git geçmişi | Geçti; 1182 dosya adayı ve geçmişteki 1083 blob denetlendi |
| Yayın denetiminin geçici Git depolarıyla testleri | 9 test geçti |
| `python scripts/check-source.py` | Geçti; 34 crate, 200 Rust dosyası, XML/kaynak/çeviri listesi/LF |
| GitHub YAML dosyalarının ayrıştırılması | Geçti; iş akışı yalnız elle başlatılır, depo izni salt okunurdur |
| Yeni README, rehber ve şablonların yerel bağlantıları | Geçti |
| `git diff --check` | Geçti |

Tekrar çalıştırma:

```powershell
python scripts/check-publication.py
python -m unittest discover -s tests -p test_publication.py -v
python scripts/check-source.py
git diff --check
```

Rust kaynakları ve ürün arayüzü bu çalışmada değiştirilmedi. Linux build,
Clippy, fmt, workspace testleri ve canlı masaüstü denemesi yeniden çalıştırılmadı.
Önceki sonuçlar ilgili uygulama/arayüz raporlarında kalır.

## Açık işler

Uzak GitHub deposu kullanıcı tarafından açıldı. `git push --dry-run origin main`
başarılı oldu; ilk gerçek push hazırlanıyor. Mevcut çalışma kopyasındaki önceki
uygulama değişiklikleri ilk kaynak yüklemesine dahil edilir. Dış CI koşusu yapılmadı.
Üretilmiş `.deb`, Asena APT arşivi veya ISO yoktur.

## Sonraki adaylar

- Faz 6A için gerçek Debian dosya sahipliği, bağımlılık ve bakım betikleri.
- Faz 6B için yerel arşiv, ürün anahtarı sahipliği ve barındırma seçimi.
