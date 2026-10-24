# Asena'ya katkıda bulun

Önce [proje talimatlarını](AGENTS.md), değişiklikle ilgili tasarım ve mimari
belgelerini okuyun. Kapsamı [uygulama yol haritasıyla](docs/apps/APP_ROADMAP.md)
karşılaştırın. Yeni ürün veya mimari kararını uygulamadan önce bir issue'da tartışın.

## Değişiklik hazırla

1. `main` üzerinden tek konuya odaklanan bir dal açın.
2. Debian 13 geliştirme VM'sinde `bash scripts/setup-dev.sh` çalıştırın.
3. Doğrudan Cargo komutlarından önce `source scripts/native-env.sh` çalıştırın.
4. Değişikliği ve gereken Türkçe çevirileri ekleyin.
5. `bash scripts/check-quality.sh` çalıştırın.
6. Davranış değiştiyse `bash scripts/dev-session.sh` içinde deneyin.
7. Sonucu ve denenemeyen koşulları pull request'e yazın.

Windows'ta `python scripts/check-source.py` ve
`python scripts/check-publication.py` kaynak denetimlerini çalıştırabilirsiniz.
Bu denetimler Linux derlemesinin veya masaüstü denemesinin yerine geçmez.
Web sitesi için [ayrı geliştirme adımlarını](website/README.md) izleyin.

## Proje kurallarını koru

- GTK 4.18, libadwaita 1.7 ve manifestteki Rust alt sürümünü koruyun.
- Ana iş parçacığında bloklayan işlem yapmayın. Test dışında `unwrap()` ve `expect()` kullanmayın.
- Kullanıcı metinlerini İngilizce kaynakla `gettext` içine alın, `po/tr.po` dosyasını güncelleyin.
- Türkçe aramada `asena_core::text::fold()` ve alfabetik sıralamada yerel karşılaştırma kullanın.
- Ortak `asena-app`, `asena-widgets` ve `asena-core` bileşenlerini kullanın.
- Sahte veri, çalışmayan kontrol ve belgesiz yeni bağımlılık eklemeyin.
- Kod, yorum ve commit mesajlarını İngilizce; belgeleri Türkçe yazın.

Commit mesajları Conventional Commits biçimini izler. Örnek:
`fix(store): preserve the selected package source`.

## Pull request gönder

Sorunu, değişen davranışı ve gerçek doğrulama sonuçlarını açıklayın.
Arayüz değişikliğinde ekran görüntüsü ekleyin. Günlüklerdeki kişisel bilgileri
paylaşmadan önce temizleyin. Donanımda denenmeyen bir özelliği doğrulanmış göstermeyin.
