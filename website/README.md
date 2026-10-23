# Asena Linux web sitesi

`asenalinux.netlify.app` için statik site (K29). Astro 7, Türkçe kökte, İngilizce `/en/` altında.
Kararlar ve bağımlılıklar: [PLANNING K29](../docs/PLANNING.md), [DEPENDENCIES](../docs/DEPENDENCIES.md),
rapor: [reports/website.md](../docs/reports/website.md).

## Çalıştırma

Node 24 gerekir. Site Windows'ta da derlenir; Rust çalışma alanıyla ilgisi yoktur.

```sh
cd website
npm ci
ASTRO_TELEMETRY_DISABLED=1 npm run dev       # http://localhost:4321
ASTRO_TELEMETRY_DISABLED=1 npm run build     # dist/ ve Pagefind arama dizini
npm run preview                              # derlenmiş siteyi sun
npm run check                                # tür denetimi
npm test                                     # fold testleri (Rust testleriyle aynı örnekler)
```

Arama dizini yalnız `npm run build` sonrasında vardır; `npm run dev` içinde arama penceresi bunu söyler.

## Yapı

| Yol | İş |
|---|---|
| `src/pages/` | Yönlendirme. Türkçe sayfalar kökte, İngilizceler `en/` altında; ikisi de aynı `src/views/` bileşenini `lang` ile çağırır |
| `src/views/` | Sayfa gövdeleri; her sayfanın iki dildeki metni aynı dosyada `c = { tr, en }[lang]` |
| `src/content/docs/<dil>/`, `src/content/news/<dil>/` | Belgeler ve haberler, Markdown. `key` iki dildeki karşılığı eşler, `permalink` adresi verir |
| `src/components/` | Masaüstü turu, görüntü değiştirici, cam/opak karşılaştırma, vurgu seçici, Türkçe arama denemesi, açılış şeridi |
| `src/i18n/` | Adresler (`routes.ts`) ve arayüz metinleri (`ui.ts`) |
| `src/styles/tokens.css` | Odak token'ları; renkler `asena_core::theme` ve DESIGN_SYSTEM'den. Bileşenler yalnız token kullanır |
| `src/lib/fold.ts` | `asena_core::text::fold` karşılığı |
| `src/assets/screens/` | Ekran görüntüleri, `<dil>-<ad>.webp` |
| `scripts/` | Ekran görüntüsü çekme ve dönüştürme |

Logo, duvar kâğıtları, uygulama, klasör ve çizgi simgeleri kopyalanmaz; `../data/` altından okunur.
Açılış görüntüleri `../docs/reports/evidence/boot-test/` içindeki gerçek test çıktılarıdır.

## Ekran görüntülerini yenilemek

Geliştirme VM'sinde, depo kökünde, derlenmiş çalışma alanıyla:

```sh
bash website/scripts/capture-screens.sh start tr
bash website/scripts/take-shots.sh tr
bash website/scripts/capture-screens.sh stop
```

Aynısı `en` ile tekrarlanır. Betik ayrı bir gösterim ev dizini kurar (duvar kâğıtları Resimler'de, birkaç
not ve takvim girdisi); kullanıcının kendi ev dizinine ve oturumuna dokunmaz. Çekimler
`target/site-capture/shots` altına iner. Windows'ta:

```sh
node website/scripts/convert-screens.mjs <png klasörü>
```

Masaüstü yüzeyleri tam kare kalır; uygulama ve vurgu çekimleri masaüstüyle farkından pencereye göre
kırpılır. Bilinen tuzaklar betiklerin yorumlarındadır (Türkçe yerel ayarda `[a-z]`, yapılandırma yeniden
yüklenince ölçeğin 1'e dönmesi, VM diski dolarken `/tmp` kullanımı).

## Metin kuralları

DESIGN'in arayüz metni kuralları burada da geçerlidir: em dash yok, slogan ve dolgu cümle yok, dekoratif
rozet yok. Yalnız çalışan şey "çalışıyor" diye yazılır; karar ve hedefler öyle adlandırılır. İngilizce metin
uygulamanın kaynak dili gibi ABD yazımıyla yazılır.

## Yayın

`../netlify.toml` Netlify'a `website/` klasörünü derlemesini söyler. Yayına alma proje sahibinin Netlify
hesabıyla yapılır.
