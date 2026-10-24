//! System Monitor window: the process list and resource graphs.
use crate::{graph::Graph, process_object::ProcessObject, procfs};
use adw::prelude::*;
use asena_app::prelude::*;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    path::Path,
    rc::Rc,
    time::Duration,
};

const INTERVAL: Duration = Duration::from_secs(2);

/// One reading of `/proc`, taken off the main thread.
struct Sample {
    processes: Vec<procfs::Process>,
    cpu: Option<(u64, u64)>,
    processors: usize,
    memory: procfs::Memory,
    network: (u64, u64),
    disks: (u64, u64),
}

fn read_sample(disks: &[String]) -> Sample {
    let read = |path: &str| std::fs::read_to_string(path).unwrap_or_default();
    let stat = read("/proc/stat");
    Sample {
        processes: procfs::processes(Path::new("/proc")),
        cpu: procfs::parse_cpu(&stat),
        processors: procfs::processor_count(&stat),
        memory: procfs::parse_memory(&read("/proc/meminfo")),
        network: procfs::parse_network(&read("/proc/net/dev")),
        disks: procfs::parse_disks(&read("/proc/diskstats"), disks),
    }
}

#[derive(Default)]
struct Previous {
    cpu: Option<(u64, u64)>,
    ticks: HashMap<u32, u64>,
    network: Option<(u64, u64)>,
    disks: Option<(u64, u64)>,
}

/// One decimal with the locale's separator; GLib's size format already follows it.
fn decimal(value: f64) -> String {
    let separator = glib::format_size(1536)
        .chars()
        .nth(1)
        .filter(|character| !character.is_ascii_digit())
        .unwrap_or('.');
    format!("{value:.1}").replace('.', &separator.to_string())
}

fn percent(value: f64) -> String {
    gettext("{value}%").replace("{value}", &decimal(value))
}

fn rate(bytes: u64) -> String {
    gettext("{size}/s").replace("{size}", &glib::format_size(bytes))
}

struct Resource {
    value: gtk::Label,
    detail: gtk::Label,
    graph: Graph,
}

impl Resource {
    fn new(column: &gtk::Box, title: &str, rates: bool) -> Self {
        let heading = gtk::Label::new(Some(title));
        heading.add_css_class("heading");
        heading.set_xalign(0.0);
        heading.set_hexpand(true);
        let value = gtk::Label::new(None);
        value.add_css_class("numeric");
        let header = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        header.append(&heading);
        header.append(&value);
        let graph = Graph::new(rates);
        graph.update_property(&[gtk::accessible::Property::Label(title)]);
        let frame = gtk::Frame::new(None);
        frame.set_child(Some(&graph));
        let detail = gtk::Label::new(None);
        detail.add_css_class("dim-label");
        detail.add_css_class("caption");
        detail.add_css_class("numeric");
        detail.set_xalign(0.0);
        detail.set_wrap(true);
        let section = gtk::Box::new(gtk::Orientation::Vertical, 6);
        section.append(&header);
        section.append(&frame);
        section.append(&detail);
        column.append(&section);
        Self {
            value,
            detail,
            graph,
        }
    }
}

struct State {
    window: AsenaWindow,
    store: gio::ListStore,
    rows: RefCell<HashMap<u32, ProcessObject>>,
    filter: gtk::CustomFilter,
    sorter: Option<gtk::Sorter>,
    selection: gtk::SingleSelection,
    user_column: gtk::ColumnViewColumn,
    end: gtk::Button,
    all: Rc<Cell<bool>>,
    query: Rc<RefCell<String>>,
    previous: RefCell<Previous>,
    users: RefCell<HashMap<u32, String>>,
    own_uid: Cell<Option<u32>>,
    disks: RefCell<Vec<String>>,
    cpu: Resource,
    memory: Resource,
    network: Resource,
    disk: Resource,
    tick: RefCell<Option<glib::SourceId>>,
    busy: Cell<bool>,
}

#[derive(Clone)]
pub struct MonitorWindow {
    state: Rc<State>,
}

/// A column whose cells show one property of the row, through `text`.
fn column(
    title: &str,
    property: &'static str,
    text: fn(&glib::Value) -> Option<String>,
    numeric: bool,
) -> gtk::ColumnViewColumn {
    let factory = gtk::SignalListItemFactory::new();
    factory.connect_setup(move |_, item| {
        let Some(item) = item.downcast_ref::<gtk::ListItem>() else {
            return;
        };
        let label = gtk::Label::new(None);
        label.set_xalign(if numeric { 1.0 } else { 0.0 });
        label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        if numeric {
            label.add_css_class("numeric");
        }
        let value = gtk::PropertyExpression::new(
            ProcessObject::static_type(),
            Some(&item.property_expression("item")),
            property,
        );
        gtk::ClosureExpression::new::<String>(
            [value],
            glib::RustClosure::new_local(move |values| Some(text(values.get(1)?)?.to_value())),
        )
        .bind(&label, "label", gtk::Widget::NONE);
        if property == "name" {
            gtk::PropertyExpression::new(
                ProcessObject::static_type(),
                Some(&item.property_expression("item")),
                "command",
            )
            .bind(&label, "tooltip-text", gtk::Widget::NONE);
        }
        item.set_child(Some(&label));
    });
    gtk::ColumnViewColumn::new(Some(title), Some(factory))
}

impl MonitorWindow {
    pub fn new(app: &adw::Application) -> Self {
        let stack = adw::ViewStack::new();

        // Processes ----------------------------------------------------------------
        let store = gio::ListStore::new::<ProcessObject>();
        let all = Rc::new(Cell::new(false));
        let query = Rc::new(RefCell::new(String::new()));
        let filter = {
            let all = all.clone();
            let query = query.clone();
            gtk::CustomFilter::new(move |object| {
                let Some(process) = object.downcast_ref::<ProcessObject>() else {
                    return false;
                };
                if !all.get() && !process.own() {
                    return false;
                }
                let query = query.borrow();
                query.is_empty()
                    || asena_core::text::fold(&format!("{} {}", process.name(), process.command()))
                        .contains(query.as_str())
            })
        };
        let filtered = gtk::FilterListModel::new(Some(store.clone()), Some(filter.clone()));
        let sorted = gtk::SortListModel::new(Some(filtered), None::<gtk::Sorter>);
        let selection = gtk::SingleSelection::new(Some(sorted.clone()));
        selection.set_autoselect(false);
        selection.set_can_unselect(true);
        let view = gtk::ColumnView::new(Some(selection.clone()));
        view.add_css_class("data-table");
        view.set_reorderable(false);

        let name = column(
            &gettext("Name"),
            "name",
            |value| value.get::<String>().ok(),
            false,
        );
        name.set_expand(true);
        name.set_resizable(true);
        let name_sorter = gtk::StringSorter::new(Some(gtk::PropertyExpression::new(
            ProcessObject::static_type(),
            None::<gtk::Expression>,
            "name",
        )));
        name_sorter.set_collation(gtk::Collation::Unicode);
        name.set_sorter(Some(&name_sorter));
        let user = column(
            &gettext("User"),
            "user",
            |value| value.get::<String>().ok(),
            false,
        );
        user.set_visible(false);
        user.set_fixed_width(110);
        let cpu = column(
            &gettext("Processor"),
            "cpu",
            |value| value.get::<f64>().ok().map(percent),
            true,
        );
        cpu.set_fixed_width(110);
        let numeric = |property: &str| {
            gtk::NumericSorter::new(Some(gtk::PropertyExpression::new(
                ProcessObject::static_type(),
                None::<gtk::Expression>,
                property,
            )))
        };
        cpu.set_sorter(Some(&numeric("cpu")));
        let memory = column(
            &gettext("Memory"),
            "memory",
            |value| {
                value
                    .get::<u64>()
                    .ok()
                    .map(|bytes| glib::format_size(bytes).to_string())
            },
            true,
        );
        memory.set_fixed_width(110);
        memory.set_sorter(Some(&numeric("memory")));
        let pid = column(
            &gettext("ID"),
            "pid",
            |value| value.get::<u32>().ok().map(|pid| pid.to_string()),
            true,
        );
        pid.set_fixed_width(80);
        pid.set_sorter(Some(&numeric("pid")));
        for column in [&name, &user, &cpu, &memory, &pid] {
            view.append_column(column);
        }
        sorted.set_sorter(view.sorter().as_ref());
        view.sort_by_column(Some(&cpu), gtk::SortType::Descending);

        let search = gtk::SearchEntry::new();
        search.set_placeholder_text(Some(&gettext("Search processes")));
        search.set_hexpand(true);
        let search_bar = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        search_bar.set_margin_top(6);
        search_bar.set_margin_bottom(6);
        search_bar.set_margin_start(12);
        search_bar.set_margin_end(12);
        search_bar.append(&search);
        let end = gtk::Button::with_label(&gettext("End process"));
        end.add_css_class("destructive-action");
        end.set_sensitive(false);
        end.set_action_name(Some("win.end-process"));
        let actions = gtk::ActionBar::new();
        actions.pack_end(&end);
        let processes = gtk::Box::new(gtk::Orientation::Vertical, 0);
        processes.append(&search_bar);
        processes.append(
            &gtk::ScrolledWindow::builder()
                .child(&view)
                .vexpand(true)
                .build(),
        );
        processes.append(&actions);
        stack.add_titled_with_icon(
            &processes,
            Some("processes"),
            &gettext("Processes"),
            "view-list-symbolic",
        );

        // Resources ----------------------------------------------------------------
        let column_box = gtk::Box::new(gtk::Orientation::Vertical, 24);
        column_box.set_margin_top(24);
        column_box.set_margin_bottom(24);
        column_box.set_margin_start(12);
        column_box.set_margin_end(12);
        let cpu_resource = Resource::new(&column_box, &gettext("Processor"), false);
        let memory_resource = Resource::new(&column_box, &gettext("Memory"), false);
        let network_resource = Resource::new(&column_box, &gettext("Network"), true);
        let disk_resource = Resource::new(&column_box, &gettext("Disk"), true);
        stack.add_titled_with_icon(
            &gtk::ScrolledWindow::builder()
                .child(
                    &adw::Clamp::builder()
                        .maximum_size(720)
                        .child(&column_box)
                        .build(),
                )
                .hscrollbar_policy(gtk::PolicyType::Never)
                .build(),
            Some("resources"),
            &gettext("Resources"),
            "utilities-system-monitor-symbolic",
        );

        // Window ---------------------------------------------------------------------
        let switcher = adw::ViewSwitcher::new();
        switcher.set_stack(Some(&stack));
        switcher.set_policy(adw::ViewSwitcherPolicy::Wide);
        let menu = gio::Menu::new();
        menu.append(
            Some(&gettext("Show all processes")),
            Some("win.all-processes"),
        );
        menu.append(Some(&gettext("About System Monitor")), Some("app.about"));
        let header = AsenaHeaderBarBuilder::new()
            .title_widget(switcher.clone())
            .primary_menu(menu.upcast_ref())
            .build();
        let bottom = adw::ViewSwitcherBar::new();
        bottom.set_stack(Some(&stack));
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.set_content(Some(&stack));
        toolbar.add_bottom_bar(&bottom);
        let window = AsenaWindowBuilder::new(app)
            .title(gettext("System Monitor"))
            .default_size(860, 620)
            .min_size(360, 420)
            .content(&toolbar)
            .build();
        let breakpoint = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            560.0,
            adw::LengthUnit::Sp,
        ));
        breakpoint.add_setter(
            &header,
            "title-widget",
            Some(&None::<gtk::Widget>.to_value()),
        );
        breakpoint.add_setter(&bottom, "reveal", Some(&true.to_value()));
        breakpoint.add_setter(&pid, "visible", Some(&false.to_value()));
        window.add_breakpoint(breakpoint);

        let state = Rc::new(State {
            window,
            store,
            rows: RefCell::default(),
            filter,
            sorter: view.sorter(),
            selection,
            user_column: user,
            end,
            all,
            query,
            previous: RefCell::default(),
            users: RefCell::default(),
            own_uid: Cell::new(None),
            disks: RefCell::default(),
            cpu: cpu_resource,
            memory: memory_resource,
            network: network_resource,
            disk: disk_resource,
            tick: RefCell::default(),
            busy: Cell::new(false),
        });
        let this = Self { state };

        let cloned = this.clone();
        search.connect_search_changed(move |entry| {
            cloned
                .state
                .query
                .replace(asena_core::text::fold(entry.text().trim()));
            cloned.state.filter.changed(gtk::FilterChange::Different);
        });
        search.set_key_capture_widget(Some(this.state.window.window()));
        let cloned = this.clone();
        this.state
            .selection
            .connect_selected_item_notify(move |_| cloned.sync_end());
        this.install_actions();
        let cloned = this.clone();
        this.state.window.connect_map(move |_| cloned.start());
        let cloned = this.clone();
        this.state.window.connect_unmap(move |_| {
            if let Some(source) = cloned.state.tick.borrow_mut().take() {
                source.remove();
            }
        });
        this.load_static();
        this
    }

    pub fn present(&self) {
        self.state.window.present();
    }

    fn install_actions(&self) {
        let window = &self.state.window;
        let all = gio::SimpleAction::new_stateful("all-processes", None, &false.to_variant());
        let this = self.clone();
        all.connect_activate(move |action, _| {
            let value = !this.state.all.get();
            action.set_state(&value.to_variant());
            this.state.all.set(value);
            this.state.user_column.set_visible(value);
            this.state.filter.changed(gtk::FilterChange::Different);
        });
        window.add_action(&all);
        let end = gio::SimpleAction::new("end-process", None);
        let this = self.clone();
        end.connect_activate(move |_, _| this.confirm_end());
        window.add_action(&end);
        if let Some(app) = window.application() {
            app.set_accels_for_action("win.end-process", &["Delete"]);
        }
    }

    fn selected(&self) -> Option<ProcessObject> {
        self.state
            .selection
            .selected_item()
            .and_downcast::<ProcessObject>()
    }

    fn sync_end(&self) {
        self.state
            .end
            .set_sensitive(self.selected().is_some_and(|process| process.own()));
    }

    /// User names, the own user id and the disk list change rarely; read them once.
    fn load_static(&self) {
        let this = self.clone();
        glib::spawn_future_local(async move {
            let (users, uid, disks) = gio::spawn_blocking(|| {
                let users = procfs::parse_users(
                    &std::fs::read_to_string("/etc/passwd").unwrap_or_default(),
                );
                let (uid, _) = procfs::parse_status(
                    &std::fs::read_to_string("/proc/self/status").unwrap_or_default(),
                );
                (users, uid, procfs::physical_disks(Path::new("/sys/block")))
            })
            .await
            .unwrap_or_default();
            this.state.users.replace(users);
            this.state.own_uid.set(uid);
            this.state.disks.replace(disks);
            this.refresh();
        });
    }

    fn start(&self) {
        if let Some(source) = self.state.tick.borrow_mut().take() {
            source.remove();
        }
        let weak = Rc::downgrade(&self.state);
        let source = glib::timeout_add_local(INTERVAL, move || {
            let Some(state) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            Self { state }.refresh();
            glib::ControlFlow::Continue
        });
        self.state.tick.replace(Some(source));
    }

    fn refresh(&self) {
        if self.state.busy.replace(true) {
            return;
        }
        let disks = self.state.disks.borrow().clone();
        let this = self.clone();
        glib::spawn_future_local(async move {
            let sample = gio::spawn_blocking(move || read_sample(&disks)).await;
            this.state.busy.set(false);
            if let Ok(sample) = sample {
                this.apply(sample);
            }
        });
    }

    fn apply(&self, sample: Sample) {
        let state = &self.state;
        let mut previous = state.previous.borrow_mut();
        let elapsed = INTERVAL.as_secs_f64();
        let total_delta = match (previous.cpu, sample.cpu) {
            (Some((before, _)), Some((now, _))) => now.saturating_sub(before),
            _ => 0,
        };

        // Processes: update rows in place, add new ones, drop the ones that exited.
        let users = state.users.borrow();
        let own = state.own_uid.get();
        let mut rows = state.rows.borrow_mut();
        let mut seen = HashMap::with_capacity(sample.processes.len());
        for process in &sample.processes {
            let used = previous
                .ticks
                .get(&process.pid)
                .map(|before| process.ticks.saturating_sub(*before))
                .unwrap_or(0);
            seen.insert(process.pid, process.ticks);
            let row = rows.entry(process.pid).or_insert_with(|| {
                let row = ProcessObject::new(process.pid);
                state.store.append(&row);
                row
            });
            let name = procfs::display_name(&process.name, &process.command);
            if row.name() != name {
                row.set_name(name);
            }
            if row.command() != process.command {
                row.set_command(process.command.clone());
            }
            let user = users
                .get(&process.uid)
                .cloned()
                .unwrap_or_else(|| process.uid.to_string());
            if row.user() != user {
                row.set_user(user);
            }
            row.set_own(own == Some(process.uid));
            row.set_cpu(procfs::share(used, total_delta));
            row.set_memory(process.memory);
        }
        rows.retain(|pid, _| seen.contains_key(pid));
        drop(rows);
        // Exited processes, removed from the end so positions stay valid.
        for position in (0..state.store.n_items()).rev() {
            let exited = state
                .store
                .item(position)
                .and_downcast::<ProcessObject>()
                .is_some_and(|row| !seen.contains_key(&row.pid()));
            if exited {
                state.store.remove(position);
            }
        }
        previous.ticks = seen;
        if let Some(sorter) = &state.sorter {
            sorter.changed(gtk::SorterChange::Different);
        }
        state.filter.changed(gtk::FilterChange::Different);
        self.sync_end();

        // Processor ------------------------------------------------------------------
        if let (Some((before_total, before_idle)), Some((total, idle))) = (previous.cpu, sample.cpu)
        {
            let busy =
                total.saturating_sub(before_total) as f64 - idle.saturating_sub(before_idle) as f64;
            let share = procfs::share(busy.max(0.0) as u64, total.saturating_sub(before_total));
            state.cpu.value.set_text(&percent(share));
            state.cpu.graph.push(&[share / 100.0]);
        }
        state.cpu.detail.set_text(
            &ngettext(
                "{count} processor",
                "{count} processors",
                sample.processors as u32,
            )
            .replace("{count}", &sample.processors.to_string()),
        );
        previous.cpu = sample.cpu;

        // Memory ---------------------------------------------------------------------
        let memory = sample.memory;
        if memory.total > 0 {
            let used = memory.total.saturating_sub(memory.available);
            state.memory.value.set_text(
                &gettext("{used} of {total}")
                    .replace("{used}", &glib::format_size(used))
                    .replace("{total}", &glib::format_size(memory.total)),
            );
            let swap_used = memory.swap_total.saturating_sub(memory.swap_free);
            state.memory.detail.set_text(&if memory.swap_total > 0 {
                gettext("Swap {used} of {total}")
                    .replace("{used}", &glib::format_size(swap_used))
                    .replace("{total}", &glib::format_size(memory.swap_total))
            } else {
                String::new()
            });
            let mut samples = vec![used as f64 / memory.total as f64];
            if memory.swap_total > 0 {
                samples.push(swap_used as f64 / memory.swap_total as f64);
            }
            state.memory.graph.push(&samples);
        }

        // Network and disk -----------------------------------------------------------
        let per_second = |now: (u64, u64), before: Option<(u64, u64)>| {
            before.map(|before| {
                (
                    (now.0.saturating_sub(before.0) as f64 / elapsed) as u64,
                    (now.1.saturating_sub(before.1) as f64 / elapsed) as u64,
                )
            })
        };
        if let Some((received, sent)) = per_second(sample.network, previous.network) {
            state.network.value.set_text(&rate(received));
            state.network.detail.set_text(
                &gettext("Received {received} · Sent {sent}")
                    .replace("{received}", &rate(received))
                    .replace("{sent}", &rate(sent)),
            );
            state.network.graph.push(&[received as f64, sent as f64]);
        }
        previous.network = Some(sample.network);
        if let Some((read, written)) = per_second(sample.disks, previous.disks) {
            state.disk.value.set_text(&rate(read + written));
            state.disk.detail.set_text(
                &gettext("Read {read} · Written {written}")
                    .replace("{read}", &rate(read))
                    .replace("{written}", &rate(written)),
            );
            state.disk.graph.push(&[read as f64, written as f64]);
        }
        previous.disks = Some(sample.disks);
    }

    fn confirm_end(&self) {
        let Some(process) = self.selected().filter(ProcessObject::own) else {
            return;
        };
        let dialog = adw::AlertDialog::new(
            Some(&gettext("End {name}?").replace("{name}", &process.name())),
            Some(&gettext("Unsaved work in this process may be lost.")),
        );
        dialog.add_response("cancel", &gettext("Cancel"));
        dialog.add_response("kill", &gettext("Force stop"));
        dialog.add_response("end", &gettext("End process"));
        dialog.set_response_appearance("kill", adw::ResponseAppearance::Destructive);
        dialog.set_response_appearance("end", adw::ResponseAppearance::Destructive);
        dialog.set_default_response(Some("cancel"));
        dialog.set_close_response("cancel");
        let this = self.clone();
        dialog.connect_response(None, move |_, response| {
            let signal = match response {
                "end" => "-TERM",
                "kill" => "-KILL",
                _ => return,
            };
            let pid = process.pid().to_string();
            let this = this.clone();
            glib::spawn_future_local(async move {
                let result = gio::spawn_blocking(move || {
                    std::process::Command::new("kill")
                        .args([signal, "--", &pid])
                        .stderr(std::process::Stdio::null())
                        .status()
                        .is_ok_and(|status| status.success())
                })
                .await
                .unwrap_or(false);
                if result {
                    this.refresh();
                } else {
                    this.state
                        .window
                        .show_toast(&gettext("Could not end the process"));
                }
            });
        });
        dialog.present(Some(self.state.window.window()));
    }
}
