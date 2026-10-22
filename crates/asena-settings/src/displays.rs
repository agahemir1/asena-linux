//! Displays panel. Edits a copy of asena-daemon's layout; the daemon applies
//! it and reverts unless the user keeps it before the confirmation ends.
use crate::{pages, rows};
use adw::prelude::*;
use asena_core::{
    displays::{self, Config, Layout, Output, Side},
    i18n::{gettext, ngettext},
};
use futures_lite::StreamExt;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

const SCALES: [f64; 7] = [1.0, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0];
const SIDES: [Side; 4] = [Side::Right, Side::Left, Side::Above, Side::Below];

#[derive(Default)]
struct State {
    layout: Layout,
    edits: Vec<Config>,
    sides: Vec<(String, Side)>,
    primary: String,
    dirty: bool,
}

impl State {
    fn reset(&mut self, layout: Layout, primary: &str) {
        self.edits = displays::current(&layout.outputs);
        self.primary = layout
            .outputs
            .iter()
            .find(|output| output.state.name == primary && output.state.enabled)
            .or_else(|| layout.outputs.iter().find(|output| output.state.enabled))
            .map(|output| output.state.name.clone())
            .unwrap_or_default();
        self.layout = layout;
        self.read_sides();
        self.dirty = false;
    }

    /// Where each output sits relative to the primary one, read from positions.
    fn read_sides(&mut self) {
        let primary = self
            .edits
            .iter()
            .find(|config| config.name == self.primary)
            .and_then(Config::rect);
        self.sides = self
            .edits
            .iter()
            .filter(|config| config.name != self.primary)
            .map(|config| {
                let side = match (primary, config.rect()) {
                    (Some(primary), Some(rect)) => displays::side_of(&primary, &rect),
                    _ => Side::Right,
                };
                (config.name.clone(), side)
            })
            .collect();
    }

    fn side(&self, name: &str) -> Side {
        self.sides
            .iter()
            .find(|(known, _)| known == name)
            .map_or(Side::Right, |(_, side)| *side)
    }

    /// The layout to send: untouched positions until something changes.
    fn arranged(&self) -> Vec<Config> {
        let mut configs = self.edits.clone();
        if self.dirty {
            displays::arrange(&mut configs, &self.primary, &self.sides);
        }
        configs
    }

    fn changed(&self) -> bool {
        self.dirty && !displays::unchanged(&self.layout.outputs, &self.arranged())
    }
}

#[derive(Clone)]
struct Panel {
    page: adw::PreferencesPage,
    groups: Rc<RefCell<Vec<adw::PreferencesGroup>>>,
    state: Rc<RefCell<State>>,
    proxy: Rc<RefCell<Option<asena_core::dbus::DaemonProxy<'static>>>>,
    dialog: Rc<RefCell<Option<adw::AlertDialog>>>,
    actions: adw::PreferencesGroup,
    apply: gtk::Button,
    window: gtk::Window,
    error: gtk::Label,
    shell: Option<gio::Settings>,
}

fn reason(error: zbus::Error) -> String {
    match error {
        zbus::Error::MethodError(_, Some(message), _) => message,
        error => error.to_string(),
    }
}

fn size_label(width: i32, height: i32) -> String {
    format!("{width} × {height}")
}

fn combo(title: &str, labels: &[String], selected: usize) -> adw::ComboRow {
    let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
    let row = adw::ComboRow::builder()
        .title(title)
        .model(&gtk::StringList::new(&labels))
        .build();
    row.set_selected(selected as u32);
    row
}

impl Panel {
    /// The primary display as the shell picks it: the chosen connector when
    /// it is connected, else GTK's first monitor.
    fn primary_setting(&self) -> String {
        let chosen = self
            .shell
            .as_ref()
            .map(|settings| settings.string("panel-monitor").to_string())
            .unwrap_or_default();
        let Some(display) = gtk::gdk::Display::default() else {
            return chosen;
        };
        let monitors = display.monitors();
        let connectors: Vec<String> = (0..monitors.n_items())
            .filter_map(|index| monitors.item(index).and_downcast::<gtk::gdk::Monitor>())
            .filter_map(|monitor| monitor.connector().map(|name| name.to_string()))
            .collect();
        if connectors.contains(&chosen) {
            chosen
        } else {
            connectors.into_iter().next().unwrap_or(chosen)
        }
    }

    fn clear(&self) {
        for group in self.groups.borrow_mut().drain(..) {
            self.page.remove(&group);
        }
    }

    fn add(&self, group: adw::PreferencesGroup) {
        // The apply buttons were added first and stay on top.
        self.page.add(&group);
        self.groups.borrow_mut().push(group);
    }

    /// The buttons appear only while there are edits to apply or discard.
    fn refresh_buttons(&self) {
        let state = self.state.borrow();
        let changed = state.changed();
        self.apply
            .set_sensitive(changed && self.dialog.borrow().is_none());
        self.actions
            .set_visible(state.layout.available && state.dirty);
    }

    fn edit(&self, index: usize, change: impl FnOnce(&mut Config)) {
        {
            let mut state = self.state.borrow_mut();
            if let Some(config) = state.edits.get_mut(index) {
                change(config);
            }
            state.dirty = true;
        }
        rows::error(&self.error, "");
        self.refresh_buttons();
    }

    fn render(&self) {
        self.clear();
        let state = self.state.borrow();
        if !state.layout.available {
            drop(state);
            self.read_only();
            self.refresh_buttons();
            return;
        }
        let outputs = state.layout.outputs.clone();
        let titles: Vec<String> = outputs
            .iter()
            .map(|output| {
                let title = output.title();
                if outputs
                    .iter()
                    .filter(|other| other.title() == title)
                    .count()
                    > 1
                {
                    format!("{title} ({})", output.state.name)
                } else {
                    title
                }
            })
            .collect();
        let enabled = state.edits.iter().filter(|config| config.enabled).count();
        let primary = state.primary.clone();
        drop(state);
        if outputs.len() > 1 {
            let group = adw::PreferencesGroup::new();
            let choices: Vec<(usize, &Output)> = outputs
                .iter()
                .enumerate()
                .filter(|(index, _)| self.state.borrow().edits[*index].enabled)
                .collect();
            let labels: Vec<String> = choices
                .iter()
                .map(|(index, _)| titles[*index].clone())
                .collect();
            let selected = choices
                .iter()
                .position(|(_, output)| output.state.name == primary)
                .unwrap_or(0);
            let row = combo(&gettext("Primary display"), &labels, selected);
            let names: Vec<String> = choices
                .iter()
                .map(|(_, output)| output.state.name.clone())
                .collect();
            let panel = self.clone();
            row.connect_selected_notify(move |row| {
                let Some(name) = names.get(row.selected() as usize).cloned() else {
                    return;
                };
                if let Some(settings) = &panel.shell {
                    if let Err(error) = settings.set_string("panel-monitor", &name) {
                        tracing::warn!(%error, "Could not save the primary display");
                    }
                }
                {
                    let mut state = panel.state.borrow_mut();
                    state.primary = name;
                    // The arrangement stays; only its reference point moves.
                    state.read_sides();
                }
                let panel = panel.clone();
                glib::idle_add_local_once(move || panel.render());
            });
            group.add(&row);
            self.add(group);
        }
        for (index, output) in outputs.iter().enumerate() {
            self.output_group(index, output, &titles[index], enabled, &primary);
        }
        self.refresh_buttons();
    }

    fn output_group(
        &self,
        index: usize,
        output: &Output,
        title: &str,
        enabled: usize,
        primary: &str,
    ) {
        let group = adw::PreferencesGroup::builder().title(title).build();
        let config = self.state.borrow().edits[index].clone();
        let mut dependent: Vec<gtk::Widget> = Vec::new();

        let mut sizes: Vec<(i32, i32)> = output
            .modes
            .iter()
            .map(|mode| (mode.width, mode.height))
            .collect();
        sizes.sort_by_key(|(width, height)| std::cmp::Reverse((width * height, *width)));
        sizes.dedup();
        if sizes.is_empty() {
            sizes.extend(config.mode.map(|mode| (mode.width, mode.height)));
        }
        let preferred = output
            .modes
            .iter()
            .find(|mode| mode.preferred)
            .map(|mode| (mode.width, mode.height));
        let labels: Vec<String> = sizes
            .iter()
            .map(|(width, height)| {
                let size = size_label(*width, *height);
                if Some((*width, *height)) == preferred {
                    gettext("{size} (recommended)").replace("{size}", &size)
                } else {
                    size
                }
            })
            .collect();
        let current = config.mode.map(|mode| (mode.width, mode.height));
        let resolution = combo(
            &gettext("Resolution"),
            &labels,
            sizes
                .iter()
                .position(|size| Some(*size) == current)
                .unwrap_or(0),
        );
        resolution.set_sensitive(sizes.len() > 1);
        group.add(&resolution);
        dependent.push(resolution.clone().upcast());

        let refresh = adw::ComboRow::builder()
            .title(gettext("Refresh rate"))
            .build();
        group.add(&refresh);
        dependent.push(refresh.clone().upcast());
        let rates: Rc<RefCell<Vec<displays::Mode>>> = Rc::default();
        let fill_rates = {
            let rates = rates.clone();
            let refresh = refresh.clone();
            let modes = output.modes.clone();
            move |size: (i32, i32), selected: Option<displays::Mode>| {
                let mut list: Vec<displays::Mode> = modes
                    .iter()
                    .copied()
                    .filter(|mode| (mode.width, mode.height) == size && mode.refresh > 0)
                    .collect();
                list.sort_by_key(|mode| std::cmp::Reverse(mode.refresh));
                list.dedup_by_key(|mode| mode.refresh);
                let labels: Vec<String> = list
                    .iter()
                    .map(|mode| format!("{:.2} Hz", f64::from(mode.refresh) / 1000.0))
                    .collect();
                let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
                refresh.set_model(Some(&gtk::StringList::new(&labels)));
                let position = selected
                    .and_then(|selected| list.iter().position(|mode| mode.same(&selected)))
                    .unwrap_or(0);
                refresh.set_selected(position as u32);
                refresh.set_visible(list.len() > 1);
                *rates.borrow_mut() = list;
            }
        };
        if let Some(size) = current {
            fill_rates(size, config.mode);
        } else {
            refresh.set_visible(false);
        }
        let filling = Rc::new(Cell::new(false));
        {
            let panel = self.clone();
            let rates = rates.clone();
            let filling = filling.clone();
            refresh.connect_selected_notify(move |row| {
                if filling.get() {
                    return;
                }
                if let Some(mode) = rates.borrow().get(row.selected() as usize).copied() {
                    panel.edit(index, |config| config.mode = Some(mode));
                }
            });
        }
        {
            let panel = self.clone();
            let modes = output.modes.clone();
            resolution.connect_selected_notify(move |row| {
                let Some(size) = sizes.get(row.selected() as usize).copied() else {
                    return;
                };
                // The preferred mode for that size, else its fastest refresh rate.
                let mut candidates: Vec<displays::Mode> = modes
                    .iter()
                    .copied()
                    .filter(|mode| (mode.width, mode.height) == size)
                    .collect();
                candidates.sort_by_key(|mode| (mode.preferred, mode.refresh));
                let Some(mode) = candidates.last().copied() else {
                    return;
                };
                panel.edit(index, |config| config.mode = Some(mode));
                filling.set(true);
                fill_rates(size, Some(mode));
                filling.set(false);
            });
        }

        let mut scales = SCALES.to_vec();
        if !scales
            .iter()
            .any(|scale| (scale - config.scale).abs() < 1e-6)
        {
            scales.push(config.scale);
            scales.sort_by(f64::total_cmp);
        }
        let scale_labels: Vec<String> = scales
            .iter()
            .map(|scale| format!("{:.0}%", scale * 100.0))
            .collect();
        let scale = combo(
            &gettext("Scale"),
            &scale_labels,
            scales
                .iter()
                .position(|scale| (scale - config.scale).abs() < 1e-6)
                .unwrap_or(0),
        );
        {
            let panel = self.clone();
            scale.connect_selected_notify(move |row| {
                if let Some(value) = scales.get(row.selected() as usize).copied() {
                    panel.edit(index, |config| config.scale = value);
                }
            });
        }
        group.add(&scale);
        dependent.push(scale.clone().upcast());

        let rotations: Vec<String> = ["0°", "90°", "180°", "270°"]
            .iter()
            .map(|label| label.to_string())
            .collect();
        let rotation = combo(
            &gettext("Rotation"),
            &rotations,
            (config.transform % 4) as usize,
        );
        {
            let panel = self.clone();
            rotation.connect_selected_notify(move |row| {
                let quarter = row.selected().min(3);
                // Keep a flip the compositor already applies.
                panel.edit(index, |config| {
                    config.transform = (config.transform & 4) | quarter
                });
            });
        }
        group.add(&rotation);
        dependent.push(rotation.clone().upcast());

        if output.state.name != primary && self.state.borrow().edits.len() > 1 {
            let labels = [
                gettext("Right of the primary display"),
                gettext("Left of the primary display"),
                gettext("Above the primary display"),
                gettext("Below the primary display"),
            ];
            let side = self.state.borrow().side(&output.state.name);
            let position = combo(
                &gettext("Position"),
                &labels,
                SIDES.iter().position(|known| *known == side).unwrap_or(0),
            );
            let panel = self.clone();
            let name = output.state.name.clone();
            position.connect_selected_notify(move |row| {
                let Some(side) = SIDES.get(row.selected() as usize).copied() else {
                    return;
                };
                {
                    let mut state = panel.state.borrow_mut();
                    state.sides.retain(|(known, _)| *known != name);
                    state.sides.push((name.clone(), side));
                }
                panel.edit(index, |_| {});
            });
            group.add(&position);
            dependent.push(position.upcast());
        }

        if self.state.borrow().edits.len() > 1 {
            let switch = adw::SwitchRow::builder()
                .title(gettext("Use this display"))
                .active(config.enabled)
                .build();
            // The last display that is on cannot be turned off here.
            switch.set_sensitive(!(config.enabled && enabled == 1));
            let panel = self.clone();
            let dependent = dependent.clone();
            // A display that was off may report no mode; start from its preferred one.
            let fallback = output
                .modes
                .iter()
                .find(|mode| mode.preferred)
                .or_else(|| output.modes.first())
                .copied();
            switch.connect_active_notify(move |switch| {
                let on = switch.is_active();
                panel.edit(index, |config| {
                    config.enabled = on;
                    if on && config.mode.is_none() {
                        config.mode = fallback;
                    }
                });
                for widget in &dependent {
                    widget.set_sensitive(on);
                }
                let panel = panel.clone();
                // Other switches and the primary choice depend on how many are on.
                glib::idle_add_local_once(move || panel.render());
            });
            group.add(&switch);
        }
        for widget in &dependent {
            if !config.enabled {
                widget.set_sensitive(false);
            }
        }
        self.add(group);
    }

    /// Without output management: what GTK knows, read only.
    fn read_only(&self) {
        let message = adw::PreferencesGroup::new();
        let row = adw::ActionRow::builder()
            .title(gettext("This session cannot change display settings"))
            .build();
        row.add_prefix(&gtk::Image::from_icon_name("dialog-information-symbolic"));
        message.add(&row);
        self.add(message);
        let Some(display) = gtk::gdk::Display::default() else {
            return;
        };
        let monitors = display.monitors();
        for index in 0..monitors.n_items() {
            let Some(monitor) = monitors.item(index).and_downcast::<gtk::gdk::Monitor>() else {
                continue;
            };
            let title = [monitor.manufacturer(), monitor.model()]
                .into_iter()
                .flatten()
                .map(|part| part.to_string())
                .collect::<Vec<_>>()
                .join(" ");
            let connector = monitor
                .connector()
                .map(|name| name.to_string())
                .unwrap_or_default();
            let section = pages::group(if title.trim().is_empty() {
                &connector
            } else {
                &title
            });
            let geometry = monitor.geometry();
            let scale = monitor.scale();
            rows::info(
                &section,
                &gettext("Resolution"),
                &size_label(
                    (f64::from(geometry.width()) * scale).round() as i32,
                    (f64::from(geometry.height()) * scale).round() as i32,
                ),
            );
            rows::info(
                &section,
                &gettext("Scale"),
                &format!("{:.0}%", scale * 100.0),
            );
            if monitor.refresh_rate() > 0 {
                rows::info(
                    &section,
                    &gettext("Refresh rate"),
                    &format!("{:.2} Hz", f64::from(monitor.refresh_rate()) / 1000.0),
                );
            }
            if !connector.is_empty() && !title.trim().is_empty() {
                rows::info(&section, &gettext("Connector"), &connector);
            }
            self.add(section);
        }
    }

    async fn load(&self) {
        self.sync(false).await;
    }

    /// Reads the daemon's layout. `discard` drops unapplied edits, as after a
    /// change was kept or reverted, so the page shows what the screens show.
    async fn sync(&self, discard: bool) {
        let Some(proxy) = self.proxy.borrow().clone() else {
            self.render();
            return;
        };
        let layout = match proxy.display_layout().await {
            Ok(text) => Layout::from_json(&text).unwrap_or_default(),
            Err(error) => {
                tracing::warn!(%error, "Could not read the display layout");
                Layout::default()
            }
        };
        let mut discard = discard;
        if !layout.pending {
            // The daemon already reverted, for example when the time ran out.
            let dialog = self.dialog.borrow_mut().take();
            if let Some(dialog) = dialog {
                dialog.force_close();
                discard = true;
            }
        }
        let hardware_changed = {
            let state = self.state.borrow();
            let names = |outputs: &[Output]| {
                outputs
                    .iter()
                    .map(|output| output.identity())
                    .collect::<Vec<_>>()
            };
            names(&state.layout.outputs) != names(&layout.outputs)
                || state.layout.available != layout.available
        };
        // Otherwise keep unapplied edits unless the monitors themselves changed.
        if discard || hardware_changed || !self.state.borrow().dirty {
            let primary = self.primary_setting();
            self.state.borrow_mut().reset(layout, &primary);
            self.render();
        } else {
            self.state.borrow_mut().layout = layout;
            self.refresh_buttons();
        }
    }

    fn apply(&self) {
        let (outputs, configs) = {
            let state = self.state.borrow();
            (state.layout.outputs.clone(), state.arranged())
        };
        if let Err(invalid) = displays::validate(&outputs, &configs) {
            rows::error(&self.error, &invalid.message());
            return;
        }
        let Some(proxy) = self.proxy.borrow().clone() else {
            return;
        };
        let Ok(scope) = asena_core::session::current_scope() else {
            rows::error(
                &self.error,
                &gettext("This session cannot change display settings"),
            );
            return;
        };
        self.apply.set_sensitive(false);
        let panel = self.clone();
        glib::spawn_future_local(async move {
            match proxy
                .apply_displays(&scope, &displays::configs_to_json(&configs))
                .await
            {
                Ok(0) => panel.sync(true).await,
                Ok(seconds) => panel.confirm(proxy, scope, seconds),
                Err(error) => {
                    rows::error(&panel.error, &reason(error));
                    panel.refresh_buttons();
                }
            }
        });
    }

    fn confirm(&self, proxy: asena_core::dbus::DaemonProxy<'static>, scope: String, seconds: u32) {
        let body = |left: u32| {
            ngettext(
                "Reverting in {seconds} second.",
                "Reverting in {seconds} seconds.",
                left,
            )
            .replace("{seconds}", &left.to_string())
        };
        let dialog = adw::AlertDialog::new(
            Some(&gettext("Keep these display settings?")),
            Some(&body(seconds)),
        );
        dialog.add_response("revert", &gettext("Revert"));
        dialog.add_response("keep", &gettext("Keep changes"));
        dialog.set_response_appearance("keep", adw::ResponseAppearance::Suggested);
        dialog.set_default_response(Some("keep"));
        dialog.set_close_response("revert");
        let left = Rc::new(Cell::new(seconds));
        let weak = dialog.downgrade();
        let timer = glib::timeout_add_seconds_local(1, move || {
            let Some(dialog) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            left.set(left.get().saturating_sub(1));
            dialog.set_body(&body(left.get()));
            // The daemon reverts on its own; its signal closes the dialog.
            glib::ControlFlow::Continue
        });
        let timer = Rc::new(RefCell::new(Some(timer)));
        let panel = self.clone();
        dialog.connect_response(None, move |_, response| {
            if let Some(timer) = timer.borrow_mut().take() {
                timer.remove();
            }
            let keep = response == "keep";
            let panel = panel.clone();
            let proxy = proxy.clone();
            let scope = scope.clone();
            panel.dialog.borrow_mut().take();
            glib::spawn_future_local(async move {
                let result = if keep {
                    proxy.confirm_displays(&scope).await
                } else {
                    proxy.revert_displays(&scope).await
                };
                if let Err(error) = result {
                    pages::toast(&panel.apply, &reason(error));
                }
                panel.sync(true).await;
            });
        });
        self.dialog.replace(Some(dialog.clone()));
        self.refresh_buttons();
        dialog.present(Some(&self.window));
    }
}

pub fn page(context: &pages::Context) -> adw::PreferencesPage {
    let page = adw::PreferencesPage::builder()
        .title(gettext("Displays"))
        .icon_name("video-display-symbolic")
        .build();
    let apply = gtk::Button::with_label(&gettext("Apply"));
    apply.add_css_class("suggested-action");
    // Discards the edits that were not applied.
    let reset = gtk::Button::with_label(&gettext("Cancel"));
    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    buttons.set_halign(gtk::Align::End);
    buttons.append(&reset);
    buttons.append(&apply);
    let actions = adw::PreferencesGroup::new();
    actions.add(&buttons);
    actions.set_visible(false);
    page.add(&actions);
    let panel = Panel {
        page: page.clone(),
        groups: Rc::default(),
        state: Rc::default(),
        proxy: Rc::default(),
        dialog: Rc::default(),
        actions,
        apply: apply.clone(),
        window: context.window.clone(),
        error: context.error.clone(),
        shell: asena_core::settings::load(asena_core::settings::SHELL_SCHEMA).ok(),
    };
    {
        let panel = panel.clone();
        apply.connect_clicked(move |_| panel.apply());
    }
    {
        let panel = panel.clone();
        reset.connect_clicked(move |_| {
            let (layout, primary) = (panel.state.borrow().layout.clone(), panel.primary_setting());
            panel.state.borrow_mut().reset(layout, &primary);
            rows::error(&panel.error, "");
            panel.render();
        });
    }
    glib::spawn_future_local(async move {
        // One connection for the page's lifetime: the daemon reverts a pending
        // change as soon as the connection that applied it goes away.
        let proxy = async {
            let connection = zbus::Connection::session().await?;
            asena_core::dbus::DaemonProxy::new(&connection).await
        }
        .await;
        let proxy = match proxy {
            Ok(proxy) => proxy,
            Err(error) => {
                tracing::warn!(%error, "Display settings are unavailable");
                panel.render();
                return;
            }
        };
        let changes = proxy.receive_displays_changed().await;
        panel.proxy.replace(Some(proxy));
        panel.load().await;
        let Ok(mut changes) = changes else {
            return;
        };
        while changes.next().await.is_some() {
            panel.load().await;
        }
    });
    page
}
