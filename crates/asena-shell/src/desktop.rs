//! Desktop icons and the desktop context menu.
//!
//! Shows the user's Home folder followed by the contents of the XDG desktop
//! directory, which `gtk::DirectoryList` keeps up to date. Double click or
//! Enter opens an item; the right mouse button opens either the item menu or
//! the desktop menu. Every action changes real files or settings.
use adw::prelude::*;
use asena_core::i18n::gettext;
use gtk::{gdk, gio, glib};
use std::path::{Path, PathBuf};

const FILE_ATTRIBUTE: &str = "standard::file";
const ATTRIBUTES: &str = "standard::name,standard::display-name,standard::icon,standard::type,\
standard::is-hidden,standard::is-backup,standard::content-type,time::modified";

/// Directory shown on the desktop, or `None` when it would be the home folder.
fn desktop_directory() -> Option<PathBuf> {
    let home = glib::home_dir();
    let desktop = glib::user_special_dir(glib::UserDirectory::Desktop)?;
    (desktop != home).then_some(desktop)
}

/// "New folder", "New folder 2", ... avoiding existing names.
pub fn unique_child(directory: &Path, base: &str, exists: impl Fn(&Path) -> bool) -> PathBuf {
    let first = directory.join(base);
    if !exists(&first) {
        return first;
    }
    (2..10_000)
        .map(|index| directory.join(format!("{base} {index}")))
        .find(|candidate| !exists(candidate))
        .unwrap_or_else(|| directory.join(format!("{base} {}", glib::monotonic_time())))
}

/// File names must stay inside their directory and be non-empty.
pub fn valid_name(name: &str) -> bool {
    let name = name.trim();
    !name.is_empty() && name != "." && name != ".." && !name.contains('/') && !name.contains('\0')
}

fn info_file(info: &gio::FileInfo) -> Option<gio::File> {
    info.attribute_object(FILE_ATTRIBUTE)
        .and_then(|object| object.downcast::<gio::File>().ok())
}

fn home_info() -> gio::FileInfo {
    let info = gio::FileInfo::new();
    let home = gio::File::for_path(glib::home_dir());
    info.set_display_name(&gettext("Home"));
    info.set_name("home");
    // The Files application icon stands for the home folder on the desktop.
    info.set_icon(&gio::ThemedIcon::new("org.asenalinux.Files"));
    info.set_file_type(gio::FileType::Directory);
    info.set_attribute_object(FILE_ATTRIBUTE, &home);
    info.set_attribute_boolean("asena::fixed", true);
    info
}

fn show_error(widget: &impl IsA<gtk::Widget>, heading: &str, error: &glib::Error) {
    let dialog = adw::AlertDialog::new(Some(heading), Some(error.message()));
    dialog.add_response("close", &gettext("Close"));
    dialog.present(Some(widget));
}

fn open_file(widget: &gtk::Widget, file: &gio::File) {
    let launcher = gtk::FileLauncher::new(Some(file));
    let weak = widget.downgrade();
    // Never pass a layer-shell window as parent: the portal would try to export
    // it with xdg-foreign, which only accepts xdg_toplevel surfaces, and the
    // compositor would disconnect the shell with a protocol error.
    launcher.launch(
        None::<&gtk::Window>,
        gio::Cancellable::NONE,
        move |result| {
            if let (Err(error), Some(widget)) = (result, weak.upgrade()) {
                if !error.matches(gtk::DialogError::Dismissed) {
                    show_error(&widget, &gettext("Could not open the item"), &error);
                }
            }
        },
    );
}

fn launch_command(command: &str) {
    crate::launch::command(command);
}

fn open_terminal(directory: &Path) {
    let terminal = asena_core::settings::load("org.asenalinux.desktop")
        .map(|settings| settings.string("terminal").to_string())
        .unwrap_or_else(|_| "asena-terminal".into());
    let launcher = gio::SubprocessLauncher::new(gio::SubprocessFlags::NONE);
    launcher.set_cwd(directory);
    for (name, value) in crate::launch::language_environment() {
        launcher.setenv(name, value, true);
    }
    if let Err(error) = launcher.spawn(&[std::ffi::OsStr::new(&terminal)]) {
        tracing::warn!(%error, terminal, "Could not open a terminal");
    }
}

struct Desktop {
    grid: gtk::GridView,
    selection: gtk::SingleSelection,
    directory: Option<PathBuf>,
}

impl Desktop {
    fn selected(&self) -> Option<gio::FileInfo> {
        self.selection
            .selected_item()
            .and_downcast::<gio::FileInfo>()
    }

    fn item(&self, position: u32) -> Option<gio::FileInfo> {
        self.selection
            .item(position)
            .and_downcast::<gio::FileInfo>()
    }

    fn new_folder(&self) {
        let Some(directory) = self.directory.clone() else {
            return;
        };
        let name = gettext("New folder");
        let path = unique_child(&directory, &name, |path| path.exists());
        let file = gio::File::for_path(path);
        let grid = self.grid.downgrade();
        file.make_directory_async(
            glib::Priority::DEFAULT,
            gio::Cancellable::NONE,
            move |result| {
                if let (Err(error), Some(grid)) = (result, grid.upgrade()) {
                    show_error(&grid, &gettext("Could not create the folder"), &error);
                }
            },
        );
    }

    fn rename(&self, info: &gio::FileInfo) {
        let Some(file) = info_file(info) else {
            return;
        };
        let dialog = adw::AlertDialog::new(Some(&gettext("Rename")), None);
        let entry = gtk::Entry::new();
        entry.set_text(&info.display_name());
        entry.set_activates_default(true);
        dialog.set_extra_child(Some(&entry));
        dialog.add_response("cancel", &gettext("Cancel"));
        dialog.add_response("rename", &gettext("Rename"));
        dialog.set_response_appearance("rename", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("rename"));
        dialog.set_close_response("cancel");
        let weak = dialog.downgrade();
        entry.connect_changed(move |entry| {
            if let Some(dialog) = weak.upgrade() {
                dialog.set_response_enabled("rename", valid_name(&entry.text()));
            }
        });
        let grid = self.grid.downgrade();
        dialog.connect_response(Some("rename"), move |_, _| {
            let name = entry.text().trim().to_owned();
            if !valid_name(&name) {
                return;
            }
            let grid = grid.clone();
            file.set_display_name_async(
                &name,
                glib::Priority::DEFAULT,
                gio::Cancellable::NONE,
                move |result| {
                    if let (Err(error), Some(grid)) = (result, grid.upgrade()) {
                        show_error(&grid, &gettext("Could not rename the item"), &error);
                    }
                },
            );
        });
        dialog.present(Some(&self.grid));
    }

    fn trash(&self, info: &gio::FileInfo) {
        let Some(file) = info_file(info) else {
            return;
        };
        let grid = self.grid.downgrade();
        file.trash_async(
            glib::Priority::DEFAULT,
            gio::Cancellable::NONE,
            move |result| {
                if let (Err(error), Some(grid)) = (result, grid.upgrade()) {
                    show_error(
                        &grid,
                        &gettext("Could not move the item to the trash"),
                        &error,
                    );
                }
            },
        );
    }

    fn to_grid(&self, area: &gtk::Box, x: f64, y: f64) -> (f64, f64) {
        area.compute_point(&self.grid, &gtk::graphene::Point::new(x as f32, y as f32))
            .map(|point| (f64::from(point.x()), f64::from(point.y())))
            .unwrap_or((x, y))
    }

    /// Position of the desktop item under an area coordinate.
    fn item_at(&self, area: &gtk::Box, x: f64, y: f64) -> Option<u32> {
        if !self.grid.is_visible() {
            return None;
        }
        let (gx, gy) = self.to_grid(area, x, y);
        if !self.grid.contains(gx, gy) {
            return None;
        }
        let mut current = self.grid.pick(gx, gy, gtk::PickFlags::DEFAULT);
        while let Some(widget) = current {
            if widget.has_css_class("asena-desktop-item") {
                return widget.widget_name().parse::<u32>().ok();
            }
            current = widget.parent();
        }
        None
    }

    fn popup(&self, model: &gio::Menu, x: f64, y: f64) {
        let popover = gtk::PopoverMenu::from_model(Some(model));
        popover.set_has_arrow(false);
        popover.add_css_class("asena-desktop-menu");
        popover.set_parent(&self.grid);
        popover.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
        popover.set_position(gtk::PositionType::Bottom);
        popover.set_halign(gtk::Align::Start);
        popover.connect_closed(|popover| {
            let popover = popover.clone();
            glib::idle_add_local_once(move || {
                if popover.parent().is_some() {
                    popover.unparent();
                }
            });
        });
        popover.popup();
    }

    fn item_menu(&self, info: &gio::FileInfo) -> gio::Menu {
        let menu = gio::Menu::new();
        let open = gio::Menu::new();
        open.append(Some(&gettext("Open")), Some("desktop.open"));
        menu.append_section(None, &open);
        if !info.boolean("asena::fixed") {
            let edit = gio::Menu::new();
            edit.append(Some(&gettext("Rename…")), Some("desktop.rename"));
            edit.append(Some(&gettext("Move to trash")), Some("desktop.trash"));
            menu.append_section(None, &edit);
        }
        menu
    }

    fn desktop_menu(&self) -> gio::Menu {
        let menu = gio::Menu::new();
        if self.directory.is_some() {
            let files = gio::Menu::new();
            files.append(Some(&gettext("New folder")), Some("desktop.new-folder"));
            files.append(Some(&gettext("Open in terminal")), Some("desktop.terminal"));
            files.append(Some(&gettext("Open in Files")), Some("desktop.files"));
            menu.append_section(None, &files);
        }
        let view = gio::Menu::new();
        view.append(
            Some(&gettext("Show desktop icons")),
            Some("desktop.show-desktop-icons"),
        );
        menu.append_section(None, &view);
        let personal = gio::Menu::new();
        personal.append(
            Some(&gettext("Change background…")),
            Some("desktop.background"),
        );
        personal.append(Some(&gettext("Display settings")), Some("desktop.displays"));
        personal.append(
            Some(&gettext("Desktop and panel settings")),
            Some("desktop.panel"),
        );
        menu.append_section(None, &personal);
        menu
    }
}

fn item_widget() -> gtk::Box {
    let item = gtk::Box::new(gtk::Orientation::Vertical, 4);
    item.add_css_class("asena-desktop-item");
    item.set_size_request(96, -1);
    let image = gtk::Image::new();
    image.set_pixel_size(48);
    item.append(&image);
    let label = gtk::Label::new(None);
    label.set_wrap(true);
    label.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    label.set_lines(2);
    label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    label.set_justify(gtk::Justification::Center);
    label.set_max_width_chars(12);
    item.append(&label);
    item
}

fn sorter() -> gtk::CustomSorter {
    gtk::CustomSorter::new(|a, b| {
        let (Some(a), Some(b)) = (
            a.downcast_ref::<gio::FileInfo>(),
            b.downcast_ref::<gio::FileInfo>(),
        ) else {
            return gtk::Ordering::Equal;
        };
        let folder = |info: &gio::FileInfo| info.file_type() == gio::FileType::Directory;
        match (folder(a), folder(b)) {
            (true, false) => gtk::Ordering::Smaller,
            (false, true) => gtk::Ordering::Larger,
            _ => glib::CollationKey::from(a.display_name().as_str())
                .cmp(&glib::CollationKey::from(b.display_name().as_str()))
                .into(),
        }
    })
}

/// Builds the desktop view that fills the background surface.
pub fn create(
    window: &gtk::ApplicationWindow,
    settings: &gio::Settings,
    icons: bool,
) -> gtk::Widget {
    let fixed = gio::ListStore::new::<gio::FileInfo>();
    fixed.append(&home_info());
    let models = gio::ListStore::new::<gio::ListModel>();
    models.append(&fixed);
    let directory = desktop_directory();
    if let Some(path) = &directory {
        if !path.exists() {
            if let Err(error) = std::fs::create_dir_all(path) {
                tracing::warn!(%error, "Desktop folder is unavailable");
            }
        }
        let list = gtk::DirectoryList::new(Some(ATTRIBUTES), Some(&gio::File::for_path(path)));
        list.set_monitored(true);
        let visible = gtk::CustomFilter::new(|item| {
            item.downcast_ref::<gio::FileInfo>()
                .is_some_and(|info| !info.is_hidden() && !info.is_backup())
        });
        let filtered = gtk::FilterListModel::new(Some(list), Some(visible));
        let sorted = gtk::SortListModel::new(Some(filtered), Some(sorter()));
        models.append(&sorted);
    }
    let flat = gtk::FlattenListModel::new(Some(models));
    let selection = gtk::SingleSelection::new(Some(flat));
    selection.set_autoselect(false);
    selection.set_can_unselect(true);
    selection.set_selected(gtk::INVALID_LIST_POSITION);

    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(|_, item| {
        if let Some(item) = item.downcast_ref::<gtk::ListItem>() {
            item.set_child(Some(&item_widget()));
        }
    });
    factory.connect_bind(|_, item| {
        let Some(item) = item.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let (Some(info), Some(widget)) = (
            item.item().and_downcast::<gio::FileInfo>(),
            item.child().and_downcast::<gtk::Box>(),
        ) else {
            return;
        };
        widget.set_widget_name(&item.position().to_string());
        if let Some(image) = widget.first_child().and_downcast::<gtk::Image>() {
            match info.icon() {
                Some(icon) => image.set_from_gicon(&icon),
                None => image.set_icon_name(Some("text-x-generic")),
            }
        }
        if let Some(label) = widget.last_child().and_downcast::<gtk::Label>() {
            label.set_text(&info.display_name());
        }
        widget.set_tooltip_text(Some(&info.display_name()));
    });
    let grid = gtk::GridView::new(Some(selection.clone()), Some(factory));
    grid.add_css_class("asena-desktop");
    grid.set_orientation(gtk::Orientation::Horizontal);
    grid.set_max_columns(32);
    grid.set_single_click_activate(false);
    grid.set_enable_rubberband(false);
    grid.set_valign(gtk::Align::Start);
    grid.set_halign(gtk::Align::Start);

    let desktop = std::rc::Rc::new(Desktop {
        grid: grid.clone(),
        selection: selection.clone(),
        directory: directory.clone(),
    });
    let state = desktop.clone();
    grid.connect_activate(move |grid, position| {
        if let Some(file) = state.item(position).as_ref().and_then(info_file) {
            open_file(grid.upcast_ref(), &file);
        }
    });

    let actions = gio::SimpleActionGroup::new();
    let add = |name: &str, callback: Box<dyn Fn()>| {
        let action = gio::SimpleAction::new(name, None);
        action.connect_activate(move |_, _| callback());
        actions.add_action(&action);
    };
    let state = desktop.clone();
    add(
        "open",
        Box::new(move || {
            if let Some(file) = state.selected().as_ref().and_then(info_file) {
                open_file(state.grid.upcast_ref(), &file);
            }
        }),
    );
    let state = desktop.clone();
    add(
        "rename",
        Box::new(move || {
            if let Some(info) = state.selected() {
                state.rename(&info);
            }
        }),
    );
    let state = desktop.clone();
    add(
        "trash",
        Box::new(move || {
            if let Some(info) = state.selected() {
                if !info.boolean("asena::fixed") {
                    state.trash(&info);
                }
            }
        }),
    );
    let state = desktop.clone();
    add("new-folder", Box::new(move || state.new_folder()));
    let path = directory.clone();
    add(
        "terminal",
        Box::new(move || {
            if let Some(path) = &path {
                open_terminal(path);
            }
        }),
    );
    let state = desktop.clone();
    let path = directory.clone();
    add(
        "files",
        Box::new(move || {
            if let Some(path) = &path {
                open_file(state.grid.upcast_ref(), &gio::File::for_path(path));
            }
        }),
    );
    add(
        "background",
        Box::new(|| launch_command("asena-settings appearance")),
    );
    add(
        "displays",
        Box::new(|| launch_command("asena-settings displays")),
    );
    add(
        "panel",
        Box::new(|| launch_command("asena-settings desktop")),
    );
    actions.add_action(&settings.create_action("show-desktop-icons"));
    window.insert_action_group("desktop", Some(&actions));

    if icons {
        settings
            .bind("show-desktop-icons", &grid, "visible")
            .get_only()
            .build();
    } else {
        // Secondary monitors keep the desktop menu without duplicate icons.
        grid.set_visible(false);
    }

    let keys = gtk::EventControllerKey::new();
    let state = desktop.clone();
    keys.connect_key_pressed(move |_, key, _, _| {
        let Some(info) = state.selected() else {
            return glib::Propagation::Proceed;
        };
        match key {
            gdk::Key::F2 if !info.boolean("asena::fixed") => state.rename(&info),
            gdk::Key::Delete if !info.boolean("asena::fixed") => state.trash(&info),
            _ => return glib::Propagation::Proceed,
        }
        glib::Propagation::Stop
    });
    grid.add_controller(keys);

    // The view fills the screen so the context menu works on empty space too.
    let area = gtk::Box::new(gtk::Orientation::Vertical, 0);
    area.set_hexpand(true);
    area.set_vexpand(true);
    area.append(&grid);
    let gesture = gtk::GestureClick::new();
    gesture.set_button(0);
    let weak_area = area.downgrade();
    gesture.connect_pressed(move |gesture, _, x, y| {
        let Some(area) = weak_area.upgrade() else {
            return;
        };
        let position = desktop.item_at(&area, x, y);
        match gesture.current_button() {
            gdk::BUTTON_SECONDARY => {
                gesture.set_state(gtk::EventSequenceState::Claimed);
                let menu = match position
                    .and_then(|index| desktop.item(index).map(|info| (index, info)))
                {
                    Some((index, info)) => {
                        desktop.selection.set_selected(index);
                        desktop.item_menu(&info)
                    }
                    None => {
                        desktop.selection.set_selected(gtk::INVALID_LIST_POSITION);
                        desktop.desktop_menu()
                    }
                };
                let (gx, gy) = desktop.to_grid(&area, x, y);
                desktop.popup(&menu, gx, gy);
            }
            gdk::BUTTON_PRIMARY if position.is_none() => {
                desktop.selection.set_selected(gtk::INVALID_LIST_POSITION);
            }
            _ => {}
        }
    });
    area.add_controller(gesture);
    area.upcast()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_names_skip_existing_entries() {
        let taken = ["/d/New folder", "/d/New folder 2"];
        let exists = |path: &Path| taken.iter().any(|item| Path::new(item) == path);
        assert_eq!(
            unique_child(Path::new("/d"), "New folder", exists),
            PathBuf::from("/d/New folder 3")
        );
        assert_eq!(
            unique_child(Path::new("/d"), "Notes", exists),
            PathBuf::from("/d/Notes")
        );
    }

    #[test]
    fn names_cannot_escape_the_directory() {
        assert!(valid_name("Belgeler"));
        assert!(valid_name("rapor.pdf"));
        assert!(!valid_name(""));
        assert!(!valid_name("   "));
        assert!(!valid_name(".."));
        assert!(!valid_name("a/b"));
    }
}
