import type { ImageMetadata } from 'astro';
import type { Lang } from '../i18n/routes';

// The visible applications of docs/apps/APP_CATALOG.md. Terminal, Camera,
// Documents and Weather are hidden in the menu (NoDisplay=true) and not listed.
const icons = import.meta.glob<{ default: ImageMetadata }>(
  '../../../data/icons/hicolor/scalable/apps/org.asenalinux.*.svg',
  { eager: true },
);

export function appIcon(id: string): ImageMetadata {
  const hit = icons[`../../../data/icons/hicolor/scalable/apps/org.asenalinux.${id}.svg`];
  if (!hit) throw new Error(`Missing app icon ${id}`);
  return hit.default;
}

type Text = Record<Lang, string>;

export interface App {
  id: string;
  slug: string;
  shot: string;
  name: Text;
  summary: Text;
  features: Record<Lang, string[]>;
}

export const apps: App[] = [
  {
    id: 'Files',
    slug: 'files',
    shot: 'app-files',
    name: { tr: 'Dosyalar', en: 'Files' },
    summary: {
      tr: 'Ev klasörünüz, takılı diskler ve Çöp aynı pencerede.',
      en: 'Your home folder, mounted drives and the Trash in one window.',
    },
    features: {
      tr: [
        'Klasörler önce gelir, adlar Türk alfabesine göre sıralanır. Ada, tarihe ya da boyuta göre de sıralanabilir.',
        'Klasörde arama yaparken "istanbul" yazmanız "İSTANBUL" adlı dosyayı da bulur.',
        'Yeniden adlandırma, yeni klasör, konumu kopyalama, çöpe taşıma ve Çöp’ten geri yükleme.',
        'Klasör içeriği arka planda okunur ve değişiklikler kendiliğinden yansır.',
        'Pencere daralınca kenar çubuğu ayrı bir sayfaya geçer.',
      ],
      en: [
        'Folders come first and names follow the Turkish alphabet. You can also sort by name, date or size.',
        'Searching a folder for "istanbul" also finds a file called "İSTANBUL".',
        'Rename, create folders, copy a location, move to the Trash and restore from it.',
        'Folders are read in the background and changes show up on their own.',
        'In a narrow window the sidebar moves to its own page.',
      ],
    },
  },
  {
    id: 'Settings',
    slug: 'settings',
    shot: 'app-settings',
    name: { tr: 'Ayarlar', en: 'Settings' },
    summary: {
      tr: 'Görünümden ekran düzenine kadar 17 bölüm, hepsi gerçek sisteme bağlı.',
      en: 'Seventeen sections, from appearance to display layout, each wired to the real system.',
    },
    features: {
      tr: [
        'Görünüm, Masaüstü ve panel, Bildirimler, Ağ, Bluetooth, Ekranlar, Ses, Klavye, Fare ve dokunmatik yüzey, Güç, Kullanıcılar, Bölge ve dil, Tarih ve saat, Varsayılan uygulamalar, Gizlilik, Erişilebilirlik ve Hakkında.',
        'Ekran çözünürlüğünü ya da ölçeğini değiştirdiğinizde 15 saniye içinde onaylamazsanız eski düzen geri gelir.',
        'Ayarlar içinde arama, Türkçe harflerle sorunsuz.',
        'Klavye düzeni ve fare hızı oturumu kapatmadan uygulanır.',
      ],
      en: [
        'Appearance, Desktop and panel, Notifications, Network, Bluetooth, Displays, Sound, Keyboard, Mouse and touchpad, Power, Users, Region and language, Date and time, Default apps, Privacy, Accessibility and About.',
        'Change the resolution or scale and the old layout comes back unless you confirm within 15 seconds.',
        'Search inside Settings, with Turkish letters handled properly.',
        'Keyboard layout and pointer speed apply without logging out.',
      ],
    },
  },
  {
    id: 'Store',
    slug: 'software',
    shot: 'app-store',
    name: { tr: 'Yazılım', en: 'Software' },
    summary: {
      tr: 'Debian arşivi ve isterseniz Flathub, tek katalogda.',
      en: 'The Debian archive and, if you want it, Flathub in one catalog.',
    },
    features: {
      tr: [
        'Kategoriler, arama, kurulu uygulamalar ve güncellemeler.',
        'Debian paketi kurmadan önce neyin değişeceğini gösterir, sonra parolanızı sorar.',
        'Flatpak uygulamasının istediği izinleri kurulumdan önce listeler.',
        'Sistem güncellemeleri arka planda iner, bir sonraki yeniden başlatmada kurulur.',
        'Yarım kalmış bir paket işlemi varsa onarmayı önerir.',
      ],
      en: [
        'Categories, search, installed apps and updates.',
        'Before installing a Debian package it shows what will change, then asks for your password.',
        'Lists the permissions a Flatpak app asks for before you install it.',
        'System updates download in the background and install on the next restart.',
        'Offers a repair when a package operation was left half done.',
      ],
    },
  },
  {
    id: 'Text',
    slug: 'text-editor',
    shot: 'app-text',
    name: { tr: 'Metin Düzenleyici', en: 'Text Editor' },
    summary: { tr: 'Sekmeli, sade bir düzenleyici.', en: 'A plain editor with tabs.' },
    features: {
      tr: [
        'Birden çok dosya sekmelerde açılır.',
        'Kaydedilmemiş değişiklik işaretlenir, kapatırken sorulur.',
        'Belgede bulma, satır ve sütun göstergesi, satır kaydırma.',
        'Dosyalar UTF-8 olarak denetlenir; 32 MB üstü açılmaz.',
      ],
      en: [
        'Several files open side by side in tabs.',
        'Unsaved changes are marked and you are asked before closing.',
        'Find in document, line and column indicator, line wrapping.',
        'Files are checked as UTF-8; anything over 32 MB is refused.',
      ],
    },
  },
  {
    id: 'Notes',
    slug: 'notes',
    shot: 'app-notes',
    name: { tr: 'Notlar', en: 'Notes' },
    summary: { tr: 'Yazdıkça kaydeden not defteri.', en: 'A notebook that saves as you type.' },
    features: {
      tr: [
        'Her not sıradan bir Markdown dosyasıdır, başka bir programla da açılır.',
        'Başlık notun ilk satırından gelir.',
        'Notlarda arama, yeni not ve çöpe taşıma.',
      ],
      en: [
        'Each note is an ordinary Markdown file that other programs can open.',
        'The title comes from the first line.',
        'Search, new note and move to Trash.',
      ],
    },
  },
  {
    id: 'Calendar',
    slug: 'calendar',
    shot: 'app-calendar',
    name: { tr: 'Takvim', en: 'Calendar' },
    summary: { tr: 'Ay görünümü ve yerel etkinlikler.', en: 'A month view with local events.' },
    features: {
      tr: [
        'Etkinlik olan günler işaretlenir, seçilen günün listesi yanda.',
        'Tüm gün ya da saatli etkinlik, konumuyla birlikte.',
        'ICS dosyası içe ve dışa aktarılır. Yinelenen etkinlik kuralları henüz açılmıyor.',
      ],
      en: [
        'Days with events are marked and the selected day is listed beside the month.',
        'All-day or timed events, with a location.',
        'Imports and exports ICS files. Repeating event rules are not expanded yet.',
      ],
    },
  },
  {
    id: 'Clock',
    slug: 'clocks',
    shot: 'app-clock',
    name: { tr: 'Saat', en: 'Clocks' },
    summary: { tr: 'Dünya saatleri, kronometre ve zamanlayıcı.', en: 'World clocks, stopwatch and timer.' },
    features: {
      tr: [
        'Dünya saatleri yaz saati dahil gerçek saat farkını gösterir.',
        'Turlarıyla kronometre.',
        'Zamanlayıcı bitince bildirim gelir.',
      ],
      en: [
        'World clocks show the real offset, daylight saving included.',
        'Stopwatch with laps.',
        'The timer sends a notification when it ends.',
      ],
    },
  },
  {
    id: 'Calculator',
    slug: 'calculator',
    shot: 'app-calculator',
    name: { tr: 'Hesap Makinesi', en: 'Calculator' },
    summary: { tr: 'Dört kip ve birim dönüştürücü.', en: 'Four modes and a unit converter.' },
    features: {
      tr: [
        'Temel, gelişmiş (trigonometri, logaritma, faktöriyel) ve programcı (HEX, DEC, OCT, BIN) tuş takımları.',
        'Uzunluk, kütle, sıcaklık, hız ve veri boyutu dönüştürme.',
        'Bellek, geçmiş ve klavyeyle kullanım.',
      ],
      en: [
        'Basic, advanced (trigonometry, logarithms, factorial) and programmer (HEX, DEC, OCT, BIN) keypads.',
        'Converts length, mass, temperature, speed and data size.',
        'Memory, history and full keyboard input.',
      ],
    },
  },
  {
    id: 'Photos',
    slug: 'photos',
    shot: 'app-photos',
    name: { tr: 'Fotoğraflar', en: 'Photos' },
    summary: { tr: 'Resimler klasörünüz, küçük resimlerle.', en: 'Your Pictures folder as a grid of thumbnails.' },
    features: {
      tr: [
        'Küçük resimler arka planda hazırlanır, fotoğrafın dönüklüğü EXIF bilgisinden düzeltilir.',
        'Önceki ve sonraki, arka plan yapma, Dosyalar’da gösterme, çöpe taşıma.',
        'Başka bir klasörü de açabilirsiniz.',
      ],
      en: [
        'Thumbnails are prepared in the background and rotated from the EXIF orientation.',
        'Previous and next, set as background, show in Files, move to Trash.',
        'You can open another folder too.',
      ],
    },
  },
  {
    id: 'Player',
    slug: 'media-player',
    shot: 'app-player',
    name: { tr: 'Oynatıcı', en: 'Media Player' },
    summary: { tr: 'Müzik ve video, GStreamer ile.', en: 'Music and video, played with GStreamer.' },
    features: {
      tr: [
        'Müzik ve Videolar klasörlerinden kitaplık ve arama.',
        'Sıradaki, önceki ve dosya açma.',
      ],
      en: [
        'A library and search from your Music and Videos folders.',
        'Next, previous and open file.',
      ],
    },
  },
  {
    id: 'Archive',
    slug: 'archive',
    shot: 'app-archive',
    name: { tr: 'Arşiv', en: 'Archive' },
    summary: { tr: 'ZIP, TAR, 7z ve fazlası.', en: 'ZIP, TAR, 7z and more.' },
    features: {
      tr: [
        'Arşivin içini açmadan listeler.',
        'Yeni bir alt klasöre çıkarır, dağınıklık bırakmaz.',
        'Çıkarmadan önce her yolu denetler; arşivdeki bir dosya hedef klasörün dışına yazılamaz.',
      ],
      en: [
        'Lists what is inside without extracting.',
        'Extracts into a new subfolder, so nothing spills out.',
        'Checks every path first; no file in an archive can be written outside the target folder.',
      ],
    },
  },
  {
    id: 'Capture',
    slug: 'screenshot',
    shot: 'app-capture',
    name: { tr: 'Ekran Görüntüsü', en: 'Screenshot' },
    summary: { tr: 'Tam ekran, gecikmeli ya da seçerek.', en: 'Whole screen, delayed or a selection.' },
    features: {
      tr: [
        'Print tuşu ekranın tamamını hemen kaydeder.',
        'Görüntü Resimler › Ekran Görüntüleri klasörüne gider, isterseniz panoya kopyalanır.',
        'Gecikme ve alan seçimi uygulamadan ayarlanır.',
      ],
      en: [
        'The Print key saves the whole screen right away.',
        'Images go to Pictures › Screenshots and can be copied to the clipboard.',
        'Set a delay or pick an area from the app.',
      ],
    },
  },
];
