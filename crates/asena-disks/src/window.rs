//! Disks window: drives and their volumes from UDisks2, with mounting,
//! safe removal and formatting. UDisks asks polkit for each operation.
use crate::model::{self, Disk, Health, Kind, Volume};
use adw::prelude::*;
use asena_app::prelude::*;
use asena_services::{object_model::ObjectModel, property_model::Endpoint};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use zbus::zvariant::Value;

const SERVICE: &str = "org.freedesktop.UDisks2";
const MANAGER: &str = "/org/freedesktop/UDisks2/Manager";

thread_local! {
    static CONNECTION: RefCell<Option<zbus::Connection>> = const { RefCell::new(None) };
}

async fn connection() -> zbus::Result<zbus::Connection> {
    if let Some(connection) = CONNECTION.with(|slot| slot.borrow().clone()) {
        return Ok(connection);
    }
    let connection = zbus::Connection::system().await?;
    CONNECTION.with(|slot| slot.replace(Some(connection.clone())));
    Ok(connection)
}

async fn call<B, R>(path: &str, interface: &str, method: &str, body: &B) -> zbus::Result<R>
where
    B: serde::ser::Serialize + zbus::zvariant::DynamicType,
    R: for<'d> zbus::zvariant::DynamicDeserialize<'d>,
{
    let proxy = zbus::Proxy::new_owned(
        connection().await?,
        SERVICE.to_owned(),
        path.to_owned(),
        interface.to_owned(),
    )
    .await?;
    proxy.call(method, body).await
}

type Options<'a> = HashMap<&'a str, Value<'a>>;

/// A short, actionable message; UDisks errors are technical.
fn failure(error: &zbus::Error) -> String {
    let text = error.to_string();
    if text.contains("DeviceBusy") || text.contains("target is busy") {
        gettext("The volume is in use. Close the files and applications using it and try again.")
    } else if text.contains("NotAuthorized") {
        gettext("You are not allowed to do this.")
    } else if text.contains("Cancelled") || text.contains("Dismissed") {
        String::new()
    } else {
        gettext("The operation failed.")
    }
}

struct State {
    window: AsenaWindow,
    model: ObjectModel,
    list: gtk::ListBox,
    split: adw::NavigationSplitView,
    content: adw::NavigationPage,
    holder: adw::Bin,
    disks: RefCell<Vec<Disk>>,
    selected: RefCell<Option<String>>,
}

#[derive(Clone)]
pub struct DisksWindow {
    state: Rc<State>,
}

fn size_text(bytes: u64) -> String {
    glib::format_size(bytes).to_string()
}

impl DisksWindow {
    pub fn new(app: &adw::Application) -> Self {
        let list = gtk::ListBox::new();
        list.add_css_class("navigation-sidebar");
        let sidebar_toolbar = adw::ToolbarView::new();
        sidebar_toolbar.add_top_bar(
            &AsenaHeaderBarBuilder::new()
                .title(gettext("Disks"))
                .primary_menu(
                    {
                        let menu = gio::Menu::new();
                        menu.append(Some(&gettext("About Disks")), Some("app.about"));
                        menu
                    }
                    .upcast_ref(),
                )
                .build(),
        );
        sidebar_toolbar.set_content(Some(
            &gtk::ScrolledWindow::builder()
                .child(&list)
                .hscrollbar_policy(gtk::PolicyType::Never)
                .build(),
        ));
        let sidebar = adw::NavigationPage::new(&sidebar_toolbar, &gettext("Disks"));
        let holder = adw::Bin::new();
        let content_toolbar = adw::ToolbarView::new();
        content_toolbar.add_top_bar(&adw::HeaderBar::new());
        content_toolbar.set_content(Some(&holder));
        let content = adw::NavigationPage::new(&content_toolbar, &gettext("Disk"));
        let split = adw::NavigationSplitView::new();
        split.set_sidebar(Some(&sidebar));
        split.set_content(Some(&content));
        let window = AsenaWindowBuilder::new(app)
            .title(gettext("Disks"))
            .default_size(900, 620)
            .min_size(360, 400)
            .content(&split)
            .build();
        let breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            640.0,
            adw::LengthUnit::Sp,
        ));
        breakpoint.add_setter(&split, "collapsed", Some(&true.to_value()));
        window.add_breakpoint(breakpoint);

        let model = ObjectModel::new(Endpoint::system(
            SERVICE,
            "/org/freedesktop/UDisks2",
            "org.freedesktop.DBus.ObjectManager",
        ));
        let state = Rc::new(State {
            window,
            model,
            list,
            split,
            content,
            holder,
            disks: RefCell::default(),
            selected: RefCell::default(),
        });
        let this = Self { state };
        this.install_actions();
        let cloned = this.clone();
        this.state.model.connect_changed(move |_| cloned.refresh());
        let cloned = this.clone();
        this.state.list.connect_row_activated(move |_, row| {
            cloned
                .state
                .selected
                .replace(Some(row.widget_name().to_string()));
            cloned.render_disk();
            cloned.state.split.set_show_content(true);
        });
        this.state.model.start();
        this.refresh();
        this
    }

    pub fn present(&self) {
        self.state.window.present();
    }

    fn install_actions(&self) {
        let window = &self.state.window;
        let add = |name: &str, run: fn(&Self, String)| {
            let action = gio::SimpleAction::new(name, Some(glib::VariantTy::STRING));
            let this = self.clone();
            action.connect_activate(move |_, parameter| {
                if let Some(path) = parameter.and_then(|value| value.str().map(str::to_owned)) {
                    run(&this, path);
                }
            });
            window.add_action(&action);
        };
        add("mount", |this, path| this.mount(path));
        add("unmount", |this, path| this.unmount(path));
        add("open", |this, point| {
            gtk::FileLauncher::new(Some(&gio::File::for_path(point))).launch(
                Some(this.state.window.window()),
                gio::Cancellable::NONE,
                |_| {},
            );
        });
        add("format-volume", |this, path| {
            this.confirm_format(path, false)
        });
        add("format-disk", |this, path| this.confirm_format(path, true));
        add("remove", |this, path| this.safely_remove(path));
        add("eject", |this, path| this.eject(path));
    }

    fn refresh(&self) {
        let state = &self.state;
        let disks = state.model.with_objects(model::disks);
        state.list.remove_all();
        for disk in &disks {
            let row = adw::ActionRow::builder()
                .title(glib::markup_escape_text(if disk.name.is_empty() {
                    &disk.path
                } else {
                    &disk.name
                }))
                .subtitle(size_text(disk.size))
                .activatable(true)
                .build();
            row.add_prefix(&gtk::Image::from_icon_name(disk.kind.icon()));
            if disk.health == Health::Failing {
                let warning = gtk::Image::from_icon_name("dialog-warning-symbolic");
                warning.add_css_class("error");
                warning.set_tooltip_text(Some(&gettext("The disk reports that it is failing")));
                row.add_suffix(&warning);
            }
            row.set_widget_name(&disk.path);
            state.list.append(&row);
        }
        let selected = state
            .selected
            .borrow()
            .clone()
            .filter(|path| disks.iter().any(|disk| &disk.path == path))
            .or_else(|| disks.first().map(|disk| disk.path.clone()));
        state.disks.replace(disks);
        state.selected.replace(selected.clone());
        let mut child = state.list.first_child();
        while let Some(widget) = child {
            if Some(widget.widget_name().as_str()) == selected.as_deref() {
                if let Some(row) = widget.downcast_ref::<gtk::ListBoxRow>() {
                    state.list.select_row(Some(row));
                }
            }
            child = widget.next_sibling();
        }
        self.render_disk();
    }

    fn render_disk(&self) {
        let state = &self.state;
        let disks = state.disks.borrow();
        let Some(disk) = state
            .selected
            .borrow()
            .as_ref()
            .and_then(|path| disks.iter().find(|disk| &disk.path == path))
        else {
            let empty = adw::StatusPage::builder()
                .icon_name("drive-harddisk-symbolic")
                .title(if state.model.available() {
                    gettext("No disks")
                } else {
                    gettext("Disk service is unavailable")
                })
                .build();
            state.holder.set_child(Some(&empty));
            state.content.set_title(&gettext("Disk"));
            return;
        };
        state.content.set_title(&disk.name);
        let page = adw::PreferencesPage::new();
        let details = adw::PreferencesGroup::builder()
            .title(glib::markup_escape_text(&disk.name))
            .build();
        let info = |title: &str, value: &str| {
            let row = adw::ActionRow::builder().title(title).build();
            let label = gtk::Label::new(Some(value));
            label.add_css_class("dim-label");
            label.set_selectable(true);
            row.add_suffix(&label);
            details.add(&row);
        };
        info(&gettext("Size"), &size_text(disk.size));
        let kind = match disk.kind {
            Kind::SolidState => gettext("Solid state disk"),
            Kind::Rotating => gettext("Hard disk"),
            Kind::Removable => gettext("Removable drive"),
            Kind::Optical => gettext("Optical drive"),
        };
        info(&gettext("Type"), &kind);
        match disk.health {
            Health::Good => info(&gettext("Health"), &gettext("Good")),
            Health::Failing => info(
                &gettext("Health"),
                &gettext("Failing. Back up its files now."),
            ),
            Health::Unknown => {}
        }
        if let Some(celsius) = disk.temperature {
            info(
                &gettext("Temperature"),
                &gettext("{value} °C").replace("{value}", &(celsius.round() as i64).to_string()),
            );
        }
        if let Some(hours) = disk.power_on_hours {
            info(
                &gettext("Powered on"),
                &ngettext(
                    "{count} hour",
                    "{count} hours",
                    hours.min(u32::MAX as u64) as u32,
                )
                .replace("{count}", &hours.to_string()),
            );
        }
        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        actions.set_margin_top(12);
        if disk.can_power_off && !disk.system() {
            let button = gtk::Button::with_label(&gettext("Safely remove"));
            button.add_css_class("pill");
            button.set_action_name(Some("win.remove"));
            button.set_action_target_value(Some(&disk.path.to_variant()));
            actions.append(&button);
        } else if disk.ejectable && !disk.system() {
            let button = gtk::Button::with_label(&gettext("Eject"));
            button.add_css_class("pill");
            button.set_action_name(Some("win.eject"));
            button.set_action_target_value(Some(&disk.path.to_variant()));
            actions.append(&button);
        }
        if let (Some(block), false) = (&disk.block, disk.system()) {
            let button = gtk::Button::with_label(&gettext("Format disk"));
            button.add_css_class("pill");
            button.add_css_class("destructive-action");
            button.set_action_name(Some("win.format-disk"));
            button.set_action_target_value(Some(&block.to_variant()));
            actions.append(&button);
        }
        page.add(&details);
        if actions.first_child().is_some() {
            let group = adw::PreferencesGroup::new();
            group.add(&actions);
            page.add(&group);
        }
        if !disk.volumes.is_empty() {
            let volumes = adw::PreferencesGroup::builder()
                .title(gettext("Volumes"))
                .build();
            for volume in &disk.volumes {
                volumes.add(&volume_row(volume));
            }
            page.add(&volumes);
        }
        state.holder.set_child(Some(&page));
    }

    fn report(&self, result: zbus::Result<()>) {
        if let Err(error) = result {
            tracing::warn!(%error, "Disk operation failed");
            let message = failure(&error);
            if !message.is_empty() {
                self.state.window.show_toast(&message);
            }
        }
    }

    fn mount(&self, path: String) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            let result: zbus::Result<String> =
                call(&path, model::FILESYSTEM, "Mount", &(Options::new(),)).await;
            this.report(result.map(|_| ()));
        });
    }

    fn unmount(&self, path: String) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            let result = call(&path, model::FILESYSTEM, "Unmount", &(Options::new(),)).await;
            this.report(result);
        });
    }

    fn eject(&self, drive: String) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            let result = call(&drive, model::DRIVE, "Eject", &(Options::new(),)).await;
            this.report(result);
        });
    }

    /// Unmounts every volume of the drive, then cuts its power.
    fn safely_remove(&self, drive: String) {
        let mounted: Vec<String> = self
            .state
            .disks
            .borrow()
            .iter()
            .find(|disk| disk.path == drive)
            .map(|disk| {
                disk.volumes
                    .iter()
                    .filter(|volume| !volume.mount_points.is_empty())
                    .map(|volume| volume.path.clone())
                    .collect()
            })
            .unwrap_or_default();
        let this = self.clone();
        glib::spawn_future_local(async move {
            for volume in mounted {
                let result: zbus::Result<()> =
                    call(&volume, model::FILESYSTEM, "Unmount", &(Options::new(),)).await;
                if result.is_err() {
                    this.report(result);
                    return;
                }
            }
            let result = call(&drive, model::DRIVE, "PowerOff", &(Options::new(),)).await;
            if result.is_ok() {
                this.state
                    .window
                    .show_toast(&gettext("The drive can be unplugged"));
            }
            this.report(result);
        });
    }

    /// Asks UDisks which filesystems it can create, then confirms with the user.
    fn confirm_format(&self, block: String, whole: bool) {
        let name = {
            let disks = self.state.disks.borrow();
            disks
                .iter()
                .find_map(|disk| {
                    if disk.block.as_deref() == Some(block.as_str()) {
                        Some(disk.name.clone())
                    } else {
                        disk.volumes
                            .iter()
                            .find(|volume| volume.path == block)
                            .map(|volume| {
                                if volume.label.is_empty() {
                                    volume.device.clone()
                                } else {
                                    volume.label.clone()
                                }
                            })
                    }
                })
                .unwrap_or_default()
        };
        let this = self.clone();
        glib::spawn_future_local(async move {
            let mut available = Vec::new();
            for (id, label) in model::FORMATS {
                let result: zbus::Result<(bool, String)> = call(
                    MANAGER,
                    "org.freedesktop.UDisks2.Manager",
                    "CanFormat",
                    &(*id,),
                )
                .await;
                if matches!(result, Ok((true, _))) {
                    available.push((*id, *label));
                }
            }
            if available.is_empty() {
                this.state
                    .window
                    .show_toast(&gettext("No formatting tools are installed"));
                return;
            }
            this.format_dialog(block, whole, name, available);
        });
    }

    fn format_dialog(
        &self,
        block: String,
        whole: bool,
        name: String,
        available: Vec<(&'static str, &'static str)>,
    ) {
        let dialog = adw::AlertDialog::new(
            Some(&gettext("Format {name}?").replace("{name}", &name)),
            Some(&gettext("Everything on it will be erased.")),
        );
        let fields = adw::PreferencesGroup::new();
        let label = adw::EntryRow::builder().title(gettext("Name")).build();
        fields.add(&label);
        let labels: Vec<&str> = available.iter().map(|(_, label)| *label).collect();
        let kind = adw::ComboRow::builder()
            .title(gettext("Format"))
            .model(&gtk::StringList::new(&labels))
            .build();
        fields.add(&kind);
        dialog.set_extra_child(Some(&fields));
        dialog.add_response("cancel", &gettext("Cancel"));
        dialog.add_response("format", &gettext("Format"));
        dialog.set_response_appearance("format", adw::ResponseAppearance::Destructive);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        let this = self.clone();
        dialog.connect_response(Some("format"), move |_, _| {
            let Some((filesystem, _)) = available.get(kind.selected() as usize).copied() else {
                return;
            };
            let text: String = label
                .text()
                .chars()
                .take(model::label_limit(filesystem))
                .collect();
            let block = block.clone();
            let this = this.clone();
            glib::spawn_future_local(async move {
                let mut options = Options::new();
                options.insert("label", Value::from(text.as_str()));
                options.insert("update-partition-type", Value::from(true));
                let result: zbus::Result<()> = if whole {
                    // A new MBR table with one partition reads on every system.
                    match call::<_, ()>(&block, model::BLOCK, "Format", &("dos", Options::new()))
                        .await
                    {
                        Ok(()) => {
                            let created: zbus::Result<zbus::zvariant::OwnedObjectPath> = call(
                                &block,
                                model::TABLE,
                                "CreatePartitionAndFormat",
                                &(0u64, 0u64, "", "", Options::new(), filesystem, options),
                            )
                            .await;
                            created.map(|_| ())
                        }
                        Err(error) => Err(error),
                    }
                } else {
                    call(&block, model::BLOCK, "Format", &(filesystem, options)).await
                };
                if result.is_ok() {
                    this.state.window.show_toast(&gettext("Formatted"));
                }
                this.report(result);
            });
        });
        dialog.present(Some(self.state.window.window()));
    }
}

fn volume_row(volume: &Volume) -> adw::ActionRow {
    let title = if volume.label.is_empty() {
        volume.device.clone()
    } else {
        volume.label.clone()
    };
    let mut details = Vec::new();
    if !volume.filesystem.is_empty() {
        details.push(volume.filesystem.clone());
    }
    details.push(size_text(volume.size));
    if let Some(point) = volume.mount_points.first() {
        details.push(point.clone());
    }
    let row = adw::ActionRow::builder()
        .title(glib::markup_escape_text(&title))
        .subtitle(glib::markup_escape_text(&details.join(" · ")))
        .build();
    let menu = gio::Menu::new();
    let target = volume.path.to_variant();
    if volume.mountable {
        if let Some(point) = volume.mount_points.first() {
            let open = gio::MenuItem::new(Some(&gettext("Open")), None);
            open.set_action_and_target_value(Some("win.open"), Some(&point.to_variant()));
            menu.append_item(&open);
            if !volume.system {
                let unmount = gio::MenuItem::new(Some(&gettext("Unmount")), None);
                unmount.set_action_and_target_value(Some("win.unmount"), Some(&target));
                menu.append_item(&unmount);
            }
        } else {
            let mount = gio::MenuItem::new(Some(&gettext("Mount")), None);
            mount.set_action_and_target_value(Some("win.mount"), Some(&target));
            menu.append_item(&mount);
        }
    }
    if !volume.system {
        let format = gio::MenuItem::new(Some(&gettext("Format")), None);
        format.set_action_and_target_value(Some("win.format-volume"), Some(&target));
        menu.append_item(&format);
    }
    if menu.n_items() > 0 {
        let button = gtk::MenuButton::new();
        button.set_icon_name("view-more-symbolic");
        button.add_css_class("flat");
        button.set_valign(gtk::Align::Center);
        button.set_tooltip_text(Some(&gettext("Volume actions")));
        button.set_menu_model(Some(&menu));
        row.add_suffix(&button);
    }
    row
}
