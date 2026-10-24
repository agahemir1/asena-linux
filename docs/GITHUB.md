# Asena GitHub deposunu yayımla

Bu rehber mevcut yerel Git deposunu `asena-linux` adıyla GitHub'a taşır.
Projenin sitesi [asenalinux.netlify.app](https://asenalinux.netlify.app) adresindedir.
Kaynak deposu [agahemir1/asena-linux](https://github.com/agahemir1/asena-linux),
Git adresi `https://github.com/agahemir1/asena-linux.git` olarak belirlendi.
İlk kaynak yüklemesi 25 Ekim 2026'da tamamlandı. Aşağıdaki adımlar başka bir
çalışma kopyası veya sonraki güncellemeler için başvuru olarak korunur.

## Yerel hazırlığı denetle

Depo kökünde PowerShell ile çalıştırın:

```powershell
python scripts/check-source.py
python scripts/check-publication.py
python -m unittest discover -s tests -p test_publication.py -v
git diff --check
git status --short
```

Yayın kontrolü mevcut izlenen dosyaları, ignore edilmeyen yeni dosyaları ve
`HEAD`, yerel dallar ve etiketlerden erişilen Git geçmişini tarar. Bilinen anahtar ve token kalıplarını,
yerel hesap dosyalarını, imaj dosyalarını ve 100 MiB sınırını denetler.
Rapor yalnız dosya adlarını ve nedenleri gösterir. Bu kontrol bütün sırları
bulma garantisi vermez. Paylaşılacak değişiklikleri ve eski commit'leri ayrıca inceleyin.

`.netlify/`, `.claude/`, ortam dosyaları, özel anahtarlar, derleme çıktıları ve
VM diskleri yeni commit'lere alınmaz. İzlenen bir dosya veya geçmişteki bir sır
`.gitignore` ile temizlenmez. Kontrol böyle bir kayıt bulursa yayından önce düzeltin.
Depodaki XanMod arşiv anahtarı herkese açık doğrulama anahtarıdır ve kaynakta kalır.

## GitHub'da boş depo aç

1. GitHub'da kendi hesabınızda **New repository** açın.
2. Adı `asena-linux` olarak girin.
3. Açıklamayı `Debian 13 tabanlı Asena Linux masaüstü ve uygulamaları` olarak girin.
4. Herkese açık kaynak için **Public** seçin.
5. README, `.gitignore` ve lisans oluşturma seçeneklerini boş bırakın. Bu dosyalar mevcut depodadır.
6. Depoyu oluşturun ve HTTPS adresini kopyalayın.

## Kaynağı yükle

Mevcut değişiklikleri inceleyin. Önceki uygulama çalışmalarının da ilk yüklemeye
girmesi gerekir. `git archive HEAD` henüz commit edilmemiş kaynakları taşımaz.

```powershell
git diff --stat
git diff
git add --all
git diff --cached --stat
python scripts/check-publication.py
```

Hazırlanan dosyalar doğruysa commit oluşturun:

```powershell
git commit -m "chore: prepare Asena sources for GitHub"
```

Bu çalışma kopyasında `origin` Asena deposuna bağlandı. Önce adresi kontrol edin.
Başka bir kopyada `origin` yoksa belirtilen adresi ekleyin. Token'ı adrese
eklemeyin; Git'in normal tarayıcı veya kimlik bilgisi akışını kullanın.

```powershell
git remote -v
git push -u origin main
```

`origin` bulunmayan yeni bir çalışma kopyasında:

```powershell
git remote add origin https://github.com/agahemir1/asena-linux.git
git push -u origin main
```

Mevcut adresi incelemeden değiştirmeyin. İlk yüklemede `--force` veya `--mirror` kullanmayın.

## GitHub ayarlarını tamamla

- **About** bölümünde siteyi `https://asenalinux.netlify.app` yapın.
- Konulara `linux`, `debian`, `wayland`, `wayfire`, `rust`, `gtk4` ve `libadwaita` ekleyin.
- **Settings > General** altında varsayılan dalın `main` olduğunu kontrol edin.
- Güvenlik açığı bildirimleri için **Settings > Security** altında private vulnerability reporting'i etkinleştirin.
- **Actions > Debian 13 > Run workflow** ile ilk Linux kontrolünü elle başlatın.
- İlk dış koşu geçtikten sonra `main` dalında pull request ve başarılı kontrol gerektiren kuralı ayarlayın.

Debian iş akışı yalnız `workflow_dispatch` ile çalışır. İlk yükleme veya pull
request otomatik iş başlatmaz. İş akışı paket, ISO, site veya APT deposu yayımlamaz.
Dış CI ilk kez geçtiğinde sonucu rapora yazın. Yerel VM sonuçları dış CI sonucu değildir.

Netlify sitesi mevcut elle yayın yolunu korur. GitHub deposu açılması siteyi
kendiliğinden Netlify'a bağlamaz. Bağlanırsa depo kökündeki `netlify.toml`
mevcut site ayarlarını taşır.

## Paket dağıtımına geç

Kaynak deposunun yüklenmesi APT deposu oluşturmaz.
[Paket dağıtım planındaki](../packaging/README.md) sıra `6A → 6B → 6C → 6D` olur.
İlk GitHub yüklemesinde indirme varmış gibi release veya ISO etiketi oluşturmayın.

Kaynaklar: [GitHub'a mevcut depo yükleme](https://docs.github.com/en/migrations/importing-source-code/using-the-command-line-to-import-source-code/adding-locally-hosted-code-to-github),
[issue form biçimi](https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/syntax-for-issue-forms).
