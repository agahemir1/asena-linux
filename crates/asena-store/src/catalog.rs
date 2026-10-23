//! The catalog: AppStream applications from Debian, the Asena archive and
//! Flathub with their install state. Loading uses AppStream's async API on the
//! main context; everything after that is plain data.
use asena_app::prelude::gettext;
use asena_core::{launcher::Category, search::rank, text::fold};
use libappstream::prelude::*;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Source {
    /// A Debian package; `asena` is true for the Asena archive.
    Package { name: String, asena: bool },
    /// A Flatpak application from a remote, such as Flathub, configured in
    /// the user's or the system's installation.
    Flatpak {
        reference: String,
        remote: String,
        user: bool,
    },
}

impl Source {
    pub fn label(&self) -> String {
        match self {
            Self::Package { asena: true, .. } => "Asena".into(),
            Self::Package { asena: false, .. } => "Debian".into(),
            Self::Flatpak { remote, user, .. } => {
                let name = if remote == "flathub" {
                    "Flathub".to_owned()
                } else {
                    remote.clone()
                };
                if *user {
                    name
                } else {
                    gettext("{remote} (system)").replace("{remote}", &name)
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Icon {
    Named(String),
    File(PathBuf),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct App {
    pub id: String,
    pub name: String,
    pub summary: String,
    pub description: String,
    pub icon: Option<Icon>,
    pub categories: Vec<String>,
    pub keywords: Vec<String>,
    pub screenshots: Vec<String>,
    pub homepage: Option<String>,
    pub license: Option<String>,
    pub sources: Vec<Source>,
}

impl App {
    pub fn category(&self) -> Category {
        asena_core::launcher::category(&self.categories)
    }
}

/// What is installed, from dpkg and Flatpak.
#[derive(Clone, Debug, Default)]
pub struct Installed {
    pub packages: HashSet<String>,
    /// Flatpak references of the user and system installations.
    pub flatpaks: HashSet<String>,
}

impl Installed {
    pub fn has(&self, source: &Source) -> bool {
        match source {
            Source::Package { name, .. } => self.packages.contains(name),
            Source::Flatpak { reference, .. } => self.flatpaks.contains(reference),
        }
    }

    pub fn any(&self, app: &App) -> bool {
        app.sources.iter().any(|source| self.has(source))
    }
}

/// Joins components that describe the same application. The first source of
/// an application keeps its texts; later ones only add sources.
pub fn merge(apps: Vec<App>) -> Vec<App> {
    let mut merged: Vec<App> = Vec::with_capacity(apps.len());
    let mut index = std::collections::HashMap::new();
    for app in apps {
        // Flatpak IDs carry no ".desktop" suffix, Debian ones often do.
        let key = app.id.trim_end_matches(".desktop").to_owned();
        match index.get(&key) {
            Some(&position) => {
                let existing: &mut App = &mut merged[position];
                for source in app.sources {
                    if !existing.sources.contains(&source) {
                        existing.sources.push(source);
                    }
                }
                if existing.screenshots.is_empty() {
                    existing.screenshots = app.screenshots;
                }
                if existing.icon.is_none() {
                    existing.icon = app.icon;
                }
            }
            None => {
                index.insert(key, merged.len());
                merged.push(app);
            }
        }
    }
    merged
}

/// Applications matching `query`, best first. Names weigh most, then
/// keywords, then summaries; Turkish letters fold the same way as the menu.
pub fn search<'a>(apps: &'a [App], query: &str) -> Vec<&'a App> {
    let folded = fold(query);
    if folded.trim().is_empty() {
        return Vec::new();
    }
    let mut scored: Vec<(u32, &App)> = apps
        .iter()
        .filter_map(|app| {
            // Scattered letters (rank 100) match too much in a catalog of thousands.
            let strong = |score: u32| if score >= 200 { score } else { 0 };
            let name = strong(rank(query, &app.name));
            let keyword = app
                .keywords
                .iter()
                .map(|keyword| strong(rank(query, keyword)).saturating_sub(50))
                .max()
                .unwrap_or(0);
            let summary = if fold(&app.summary).contains(&folded) {
                90
            } else {
                0
            };
            let score = name.max(keyword).max(summary);
            (score > 0).then_some((score, app))
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| collate(&a.1.name, &b.1.name)));
    scored.into_iter().map(|(_, app)| app).collect()
}

/// Locale aware order for names shown in lists (CLAUDE.md rule 8).
pub fn collate(a: &str, b: &str) -> std::cmp::Ordering {
    glib::CollationKey::from(a).cmp(&glib::CollationKey::from(b))
}

pub fn sorted<'a>(apps: impl Iterator<Item = &'a App>) -> Vec<&'a App> {
    let mut apps: Vec<&App> = apps.collect();
    apps.sort_by(|a, b| collate(&a.name, &b.name));
    apps
}

fn text(value: Option<glib::GString>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

/// Where a component comes from while it is converted.
enum Origin<'a> {
    /// Debian and the Asena archive, through the system catalog.
    System,
    /// A Flatpak remote's catalog in `directory`.
    Flatpak {
        remote: &'a str,
        user: bool,
        directory: &'a Path,
    },
}

/// Icons of the system catalog live under /var/lib/swcatalog/icons/<origin>,
/// those of a Flatpak remote next to its catalog.
fn icon(component: &libappstream::Component, origin: &Origin<'_>) -> Option<Icon> {
    let icons = component.icons();
    let base = match origin {
        Origin::System => PathBuf::from("/var/lib/swcatalog/icons").join(text(component.origin())),
        Origin::Flatpak { directory, .. } => directory.join("icons"),
    };
    let cached = |size: u32| {
        icons.iter().find_map(|icon| {
            if icon.kind() != libappstream::IconKind::Cached || icon.width() != size {
                return None;
            }
            let file = icon
                .filename()
                .map(|path| PathBuf::from(path.as_str()))
                .filter(|path| path.is_absolute())
                .or_else(|| {
                    icon.name()
                        .map(|name| base.join(format!("{size}x{size}")).join(name.as_str()))
                })?;
            file.is_file().then_some(file)
        })
    };
    cached(128)
        .or_else(|| cached(64))
        .map(Icon::File)
        .or_else(|| {
            icons.iter().find_map(|icon| match icon.kind() {
                libappstream::IconKind::Local => icon
                    .filename()
                    .map(|path| PathBuf::from(path.as_str()))
                    .filter(|path| path.is_file())
                    .map(Icon::File),
                libappstream::IconKind::Stock => {
                    icon.name().map(|name| Icon::Named(name.to_string()))
                }
                _ => None,
            })
        })
}

fn screenshots(component: &libappstream::Component) -> Vec<String> {
    component
        .screenshots_all()
        .iter()
        .filter_map(|screenshot| {
            let images = screenshot.images();
            // The largest image that still fits a detail page.
            images
                .iter()
                .filter(|image| image.width() <= 1600)
                .max_by_key(|image| image.width())
                .or_else(|| images.first())
                .and_then(|image| image.url())
                .map(|url| url.to_string())
        })
        .filter(|url| url.starts_with("https://"))
        .take(6)
        .collect()
}

fn source(component: &libappstream::Component, origin: &Origin<'_>) -> Option<Source> {
    match origin {
        Origin::Flatpak { remote, user, .. } => {
            let bundle = component.bundle(libappstream::BundleKind::Flatpak)?;
            Some(Source::Flatpak {
                reference: text(bundle.id()),
                remote: (*remote).to_owned(),
                user: *user,
            })
        }
        Origin::System => {
            let name = text(component.pkgname());
            asena_core::packages::valid_package_name(&name).then(|| Source::Package {
                asena: text(component.origin()).starts_with("asena"),
                name,
            })
        }
    }
}

fn convert(components: &[libappstream::Component], origin: &Origin<'_>) -> Vec<App> {
    components
        .iter()
        .filter(|component| component.kind() == libappstream::ComponentKind::DesktopApp)
        .filter(|component| !component.is_ignored())
        .filter_map(|component| {
            let name = text(component.name());
            let source = source(component, origin)?;
            (!name.is_empty()).then(|| App {
                id: text(component.id()),
                summary: text(component.summary()),
                description: component
                    .description()
                    .and_then(|markup| {
                        libappstream::functions::markup_convert(
                            &markup,
                            libappstream::MarkupKind::Text,
                        )
                        .ok()
                    })
                    .map(|description| description.to_string())
                    .unwrap_or_default(),
                icon: icon(component, origin),
                categories: component
                    .categories()
                    .iter()
                    .map(|c| c.to_string())
                    .collect(),
                keywords: component.keywords().iter().map(|k| k.to_string()).collect(),
                screenshots: screenshots(component),
                homepage: component
                    .url(libappstream::UrlKind::Homepage)
                    .map(|url| url.to_string()),
                license: component
                    .project_license()
                    .map(|license| license.to_string()),
                sources: vec![source],
                name,
            })
        })
        .collect()
}

fn system_catalog() -> Vec<App> {
    let pool = libappstream::Pool::new();
    pool.set_flags(libappstream::PoolFlags::LOAD_OS_CATALOG);
    if let Err(error) = pool.load(gio::Cancellable::NONE) {
        tracing::warn!(%error, "The system catalog is incomplete");
    }
    pool.components()
        .map(|components| convert(&components.as_array(), &Origin::System))
        .unwrap_or_default()
}

/// AppStream's own Flatpak loader names every remote "flatpak"; reading each
/// remote's catalog keeps the remote and its installation.
fn flatpak_catalogs() -> Vec<App> {
    crate::flatpak::catalogs()
        .into_iter()
        .flat_map(|catalog| {
            let metadata = libappstream::Metadata::new();
            metadata.set_format_style(libappstream::FormatStyle::Catalog);
            let file = gio::File::for_path(catalog.directory.join("appstream.xml.gz"));
            if let Err(error) = metadata.parse_file(&file, libappstream::FormatKind::Xml) {
                tracing::warn!(%error, remote = %catalog.remote, "Could not read a Flatpak catalog");
                return Vec::new();
            }
            let origin = Origin::Flatpak {
                remote: &catalog.remote,
                user: catalog.user,
                directory: &catalog.directory,
            };
            metadata
                .components()
                .map(|components| convert(&components.as_array(), &origin))
                .unwrap_or_default()
        })
        .collect()
}

/// Loads every catalog on a worker thread; AppStream objects stay there.
pub async fn load() -> Vec<App> {
    gio::spawn_blocking(|| {
        let mut apps = system_catalog();
        apps.extend(flatpak_catalogs());
        merge(apps)
    })
    .await
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(id: &str, name: &str, source: Source) -> App {
        App {
            id: id.into(),
            name: name.into(),
            summary: String::new(),
            description: String::new(),
            icon: None,
            categories: Vec::new(),
            keywords: Vec::new(),
            screenshots: Vec::new(),
            homepage: None,
            license: None,
            sources: vec![source],
        }
    }

    fn package(name: &str) -> Source {
        Source::Package {
            name: name.into(),
            asena: false,
        }
    }

    fn flatpak(id: &str) -> Source {
        Source::Flatpak {
            reference: format!("app/{id}/x86_64/stable"),
            remote: "flathub".into(),
            user: true,
        }
    }

    #[test]
    fn the_same_application_from_two_sources_is_one_entry() {
        let apps = merge(vec![
            app("org.gnome.Loupe.desktop", "Loupe", package("loupe")),
            app(
                "org.gnome.Loupe",
                "Image Viewer",
                flatpak("org.gnome.Loupe"),
            ),
            app("org.gnome.Papers", "Papers", flatpak("org.gnome.Papers")),
        ]);
        assert_eq!(apps.len(), 2);
        assert_eq!(apps[0].name, "Loupe");
        assert_eq!(apps[0].sources.len(), 2);
        assert_eq!(apps[0].sources[1].label(), "Flathub");
    }

    #[test]
    fn search_folds_turkish_letters_and_prefers_names() {
        let mut calculator = app("calc", "Hesap Makinesi", package("calc"));
        calculator.summary = "Dört işlem".into();
        let mut images = app("images", "Görüntüler", package("images"));
        images.keywords = vec!["fotoğraf".into()];
        let mut editor = app("editor", "Metin", package("editor"));
        editor.summary = "Görüntüleri açmaz".into();
        let synthesizer = app("synth", "Analog Modelling Synthesizer", package("synth"));
        let apps = vec![calculator, images, editor, synthesizer];
        assert!(search(&apps, "mines").is_empty());
        let names: Vec<&str> = search(&apps, "GÖRÜNTÜ")
            .iter()
            .map(|a| a.name.as_str())
            .collect();
        assert_eq!(names, ["Görüntüler", "Metin"]);
        assert_eq!(search(&apps, "fotograf")[0].name, "Görüntüler");
        assert_eq!(search(&apps, "dort")[0].name, "Hesap Makinesi");
        assert!(search(&apps, "  ").is_empty());
    }

    #[test]
    fn installed_state_follows_each_source() {
        let mut installed = Installed::default();
        installed.packages.insert("loupe".into());
        let loupe = app("loupe", "Loupe", package("loupe"));
        let papers = app("papers", "Papers", flatpak("org.gnome.Papers"));
        assert!(installed.any(&loupe));
        assert!(!installed.any(&papers));
        installed
            .flatpaks
            .insert("app/org.gnome.Papers/x86_64/stable".into());
        assert!(installed.any(&papers));
    }
}
