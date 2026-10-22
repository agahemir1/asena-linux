//! Calculator window: keypad modes, unit converter and history.
use crate::calc::{
    convert_units, format_number, unit_names, BinaryOp, CalcMode, CalculatorEngine, UnaryOp,
    UnitCategory,
};
use adw::prelude::*;
use asena_app::prelude::*;
use std::{cell::RefCell, rc::Rc};

const CATEGORIES: [UnitCategory; 5] = [
    UnitCategory::Length,
    UnitCategory::Mass,
    UnitCategory::Temperature,
    UnitCategory::Speed,
    UnitCategory::DataVolume,
];

struct State {
    window: AsenaWindow,
    engine: RefCell<CalculatorEngine>,
    expression: gtk::Label,
    result: gtk::Label,
    bases: gtk::Grid,
    base_values: [gtk::Label; 4],
    scientific: gtk::Revealer,
    programmer: gtk::Revealer,
    stack: gtk::Stack,
    history: gtk::ListBox,
    history_empty: gtk::Label,
}

#[derive(Clone)]
pub struct CalculatorWindow {
    state: Rc<State>,
}

fn key(label: &str, css: &[&str]) -> gtk::Button {
    let button = gtk::Button::with_label(label);
    button.set_hexpand(true);
    button.set_vexpand(true);
    button.add_css_class("asena-calc-key");
    for class in css {
        button.add_css_class(class);
    }
    button
}

impl CalculatorWindow {
    pub fn new(app: &adw::Application) -> Self {
        let expression = gtk::Label::new(None);
        expression.set_xalign(1.0);
        expression.add_css_class("dim-label");
        expression.add_css_class("numeric");
        let result = gtk::Label::new(Some("0"));
        result.set_xalign(1.0);
        result.set_selectable(true);
        result.set_ellipsize(gtk::pango::EllipsizeMode::Start);
        result.add_css_class("title-1");
        result.add_css_class("numeric");
        let display = gtk::Box::new(gtk::Orientation::Vertical, 4);
        display.add_css_class("card");
        display.set_margin_bottom(4);
        for label in [&expression, &result] {
            label.set_margin_start(12);
            label.set_margin_end(12);
        }
        expression.set_margin_top(10);
        result.set_margin_bottom(10);
        display.append(&expression);
        display.append(&result);

        let bases = gtk::Grid::new();
        bases.set_column_spacing(12);
        bases.set_row_spacing(2);
        let base_values = [
            gtk::Label::new(None),
            gtk::Label::new(None),
            gtk::Label::new(None),
            gtk::Label::new(None),
        ];
        for (row, (name, value)) in ["HEX", "DEC", "OCT", "BIN"]
            .iter()
            .zip(&base_values)
            .enumerate()
        {
            let caption = gtk::Label::new(Some(name));
            caption.add_css_class("caption-heading");
            caption.add_css_class("dim-label");
            caption.set_xalign(0.0);
            value.set_xalign(0.0);
            value.set_selectable(true);
            value.set_ellipsize(gtk::pango::EllipsizeMode::Start);
            value.add_css_class("monospace");
            bases.attach(&caption, 0, row as i32, 1, 1);
            bases.attach(value, 1, row as i32, 1, 1);
        }
        bases.set_visible(false);

        // Keypads ---------------------------------------------------------------------
        let keypad = gtk::Grid::new();
        keypad.set_row_spacing(6);
        keypad.set_column_spacing(6);
        keypad.set_row_homogeneous(true);
        keypad.set_column_homogeneous(true);
        keypad.set_vexpand(true);
        let scientific_grid = gtk::Grid::new();
        scientific_grid.set_row_spacing(6);
        scientific_grid.set_column_spacing(6);
        scientific_grid.set_column_homogeneous(true);
        scientific_grid.set_margin_bottom(6);
        let scientific = gtk::Revealer::new();
        scientific.set_child(Some(&scientific_grid));
        let programmer_grid = gtk::Grid::new();
        programmer_grid.set_row_spacing(6);
        programmer_grid.set_column_spacing(6);
        programmer_grid.set_column_homogeneous(true);
        programmer_grid.set_margin_bottom(6);
        let programmer = gtk::Revealer::new();
        programmer.set_child(Some(&programmer_grid));
        let pad = gtk::Box::new(gtk::Orientation::Vertical, 0);
        pad.append(&scientific);
        pad.append(&programmer);
        pad.append(&keypad);

        // Converter -------------------------------------------------------------------
        let converter = gtk::Box::new(gtk::Orientation::Vertical, 12);
        let category_names: Vec<String> = vec![
            gettext("Length"),
            gettext("Mass"),
            gettext("Temperature"),
            gettext("Speed"),
            gettext("Data size"),
        ];
        let category_refs: Vec<&str> = category_names.iter().map(String::as_str).collect();
        let category = gtk::DropDown::from_strings(&category_refs);
        let from_value = gtk::Entry::new();
        from_value.set_input_purpose(gtk::InputPurpose::Number);
        from_value.set_text("1");
        let from_unit = gtk::DropDown::from_strings(&[]);
        let to_value = gtk::Label::new(None);
        to_value.set_xalign(0.0);
        to_value.set_selectable(true);
        to_value.add_css_class("title-2");
        to_value.add_css_class("numeric");
        let to_unit = gtk::DropDown::from_strings(&[]);
        let swap = gtk::Button::from_icon_name("view-refresh-symbolic");
        swap.set_tooltip_text(Some(&gettext("Swap units")));
        swap.set_halign(gtk::Align::Center);
        swap.add_css_class("circular");
        converter.append(&category);
        let from_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        from_value.set_hexpand(true);
        from_row.append(&from_value);
        from_row.append(&from_unit);
        converter.append(&from_row);
        converter.append(&swap);
        let to_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        to_value.set_hexpand(true);
        to_row.append(&to_value);
        to_row.append(&to_unit);
        converter.append(&to_row);

        let stack = gtk::Stack::new();
        stack.set_vexpand(true);
        stack.add_named(&pad, Some("pad"));
        stack.add_named(&converter, Some("converter"));

        let content = gtk::Box::new(gtk::Orientation::Vertical, 8);
        content.set_margin_top(12);
        content.set_margin_bottom(12);
        content.set_margin_start(12);
        content.set_margin_end(12);
        content.append(&display);
        content.append(&bases);
        content.append(&stack);

        // Header ------------------------------------------------------------------------
        let modes = adw::ToggleGroup::new();
        for (name, label) in [
            ("basic", gettext("Basic")),
            ("scientific", gettext("Advanced")),
            ("programmer", gettext("Programmer")),
            ("converter", gettext("Convert")),
        ] {
            modes.add(adw::Toggle::builder().name(name).label(label).build());
        }
        modes.set_active_name(Some("basic"));
        let history = gtk::ListBox::new();
        history.set_selection_mode(gtk::SelectionMode::None);
        let history_empty = gtk::Label::new(Some(&gettext("No calculations yet")));
        history_empty.add_css_class("dim-label");
        history_empty.set_margin_top(12);
        history_empty.set_margin_bottom(12);
        let history_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
        history_box.append(&history_empty);
        history_box.append(
            &gtk::ScrolledWindow::builder()
                .child(&history)
                .hscrollbar_policy(gtk::PolicyType::Never)
                .propagate_natural_height(true)
                .max_content_height(360)
                .min_content_width(240)
                .build(),
        );
        let history_popover = gtk::Popover::new();
        history_popover.set_child(Some(&history_box));
        let history_button = gtk::MenuButton::new();
        history_button.set_icon_name("document-open-recent-symbolic");
        history_button.set_tooltip_text(Some(&gettext("History")));
        history_button.set_popover(Some(&history_popover));
        let menu = gio::Menu::new();
        menu.append(Some(&gettext("About Calculator")), Some("app.about"));
        let menu_button = gtk::MenuButton::new();
        menu_button.set_icon_name("open-menu-symbolic");
        menu_button.set_tooltip_text(Some(&gettext("Main menu")));
        menu_button.set_menu_model(Some(&menu));
        let header = adw::HeaderBar::new();
        header.pack_end(&menu_button);
        header.pack_start(&history_button);
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        let switcher_bar = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        switcher_bar.set_halign(gtk::Align::Center);
        switcher_bar.set_margin_bottom(4);
        switcher_bar.append(&modes);
        toolbar.add_top_bar(&switcher_bar);
        toolbar.set_content(Some(&content));

        let window = AsenaWindowBuilder::new(app)
            .title(gettext("Calculator"))
            .default_size(400, 600)
            .min_size(320, 480)
            .content(&toolbar)
            .build();

        let state = Rc::new(State {
            window,
            engine: RefCell::new(CalculatorEngine::new()),
            expression,
            result,
            bases,
            base_values,
            scientific,
            programmer,
            stack,
            history,
            history_empty,
        });
        let this = Self { state };
        this.build_keypad(&keypad);
        this.build_scientific(&scientific_grid);
        this.build_programmer(&programmer_grid);
        let cloned = this.clone();
        modes.connect_active_name_notify(move |group| {
            cloned.set_mode(group.active_name().as_deref().unwrap_or("basic"));
        });
        this.setup_converter(
            &category,
            &from_value,
            &from_unit,
            &to_value,
            &to_unit,
            &swap,
        );
        this.setup_keyboard();
        this.refresh();
        this
    }

    pub fn present(&self) {
        self.state.window.present();
    }

    fn act(&self, action: impl Fn(&mut CalculatorEngine)) {
        action(&mut self.state.engine.borrow_mut());
        self.refresh();
    }

    fn connect(&self, button: &gtk::Button, action: impl Fn(&mut CalculatorEngine) + 'static) {
        let this = self.clone();
        button.connect_clicked(move |_| this.act(&action));
    }

    fn build_keypad(&self, grid: &gtk::Grid) {
        let layout: [(&str, i32, i32, &[&str]); 24] = [
            ("MC", 0, 0, &["flat"]),
            ("MR", 1, 0, &["flat"]),
            ("M+", 2, 0, &["flat"]),
            ("M−", 3, 0, &["flat"]),
            ("C", 0, 1, &[]),
            ("CE", 1, 1, &[]),
            ("⌫", 2, 1, &[]),
            ("÷", 3, 1, &["asena-calc-operator"]),
            ("7", 0, 2, &["asena-calc-digit"]),
            ("8", 1, 2, &["asena-calc-digit"]),
            ("9", 2, 2, &["asena-calc-digit"]),
            ("×", 3, 2, &["asena-calc-operator"]),
            ("4", 0, 3, &["asena-calc-digit"]),
            ("5", 1, 3, &["asena-calc-digit"]),
            ("6", 2, 3, &["asena-calc-digit"]),
            ("−", 3, 3, &["asena-calc-operator"]),
            ("1", 0, 4, &["asena-calc-digit"]),
            ("2", 1, 4, &["asena-calc-digit"]),
            ("3", 2, 4, &["asena-calc-digit"]),
            ("+", 3, 4, &["asena-calc-operator"]),
            ("±", 0, 5, &[]),
            ("0", 1, 5, &["asena-calc-digit"]),
            (".", 2, 5, &["asena-calc-digit"]),
            ("=", 3, 5, &["suggested-action"]),
        ];
        for (label, column, row, css) in layout {
            let button = key(label, css);
            grid.attach(&button, column, row, 1, 1);
            match label {
                "MC" => self.connect(&button, CalculatorEngine::memory_clear),
                "MR" => self.connect(&button, CalculatorEngine::memory_recall),
                "M+" => self.connect(&button, CalculatorEngine::memory_add),
                "M−" => self.connect(&button, CalculatorEngine::memory_subtract),
                "C" => self.connect(&button, CalculatorEngine::clear),
                "CE" => self.connect(&button, CalculatorEngine::clear_entry),
                "⌫" => self.connect(&button, CalculatorEngine::backspace),
                "÷" => self.connect(&button, |engine| engine.set_binary_op(BinaryOp::Divide)),
                "×" => self.connect(&button, |engine| engine.set_binary_op(BinaryOp::Multiply)),
                "−" => self.connect(&button, |engine| engine.set_binary_op(BinaryOp::Subtract)),
                "+" => self.connect(&button, |engine| engine.set_binary_op(BinaryOp::Add)),
                "±" => self.connect(&button, |engine| engine.apply_unary_op(UnaryOp::Negate)),
                "." => self.connect(&button, CalculatorEngine::input_dot),
                "=" => self.connect(&button, CalculatorEngine::equals),
                digit => {
                    let digit = digit.chars().next().unwrap_or('0');
                    self.connect(&button, move |engine| engine.input_digit(digit));
                }
            }
        }
        for (label, tooltip) in [
            ("MC", gettext("Clear memory")),
            ("MR", gettext("Recall memory")),
            ("M+", gettext("Add to memory")),
            ("M−", gettext("Subtract from memory")),
        ] {
            let mut child = grid.first_child();
            while let Some(widget) = child {
                if let Some(button) = widget.downcast_ref::<gtk::Button>() {
                    if button.label().as_deref() == Some(label) {
                        button.set_tooltip_text(Some(&tooltip));
                    }
                }
                child = widget.next_sibling();
            }
        }
    }

    fn build_scientific(&self, grid: &gtk::Grid) {
        let unary = [
            ("sin", UnaryOp::Sin),
            ("cos", UnaryOp::Cos),
            ("tan", UnaryOp::Tan),
            ("√", UnaryOp::SquareRoot),
            ("ln", UnaryOp::Ln),
            ("log", UnaryOp::Log10),
            ("n!", UnaryOp::Factorial),
        ];
        for (index, (label, op)) in unary.into_iter().enumerate() {
            let button = key(label, &[]);
            grid.attach(&button, (index % 4) as i32, (index / 4) as i32, 1, 1);
            self.connect(&button, move |engine| engine.apply_unary_op(op));
        }
        let power = key("xʸ", &[]);
        grid.attach(&power, 3, 1, 1, 1);
        self.connect(&power, |engine| engine.set_binary_op(BinaryOp::Power));
        let pi = key("π", &[]);
        grid.attach(&pi, 0, 2, 1, 1);
        self.connect(&pi, |engine| engine.input_constant("pi"));
        let e = key("e", &[]);
        grid.attach(&e, 1, 2, 1, 1);
        self.connect(&e, |engine| engine.input_constant("e"));
        let modulo = key("mod", &[]);
        grid.attach(&modulo, 2, 2, 1, 1);
        self.connect(&modulo, |engine| engine.set_binary_op(BinaryOp::Modulo));
    }

    fn build_programmer(&self, grid: &gtk::Grid) {
        let binary = [
            ("AND", BinaryOp::BitAnd),
            ("OR", BinaryOp::BitOr),
            ("XOR", BinaryOp::BitXor),
            ("<<", BinaryOp::ShiftLeft),
            (">>", BinaryOp::ShiftRight),
        ];
        for (index, (label, op)) in binary.into_iter().enumerate() {
            let button = key(label, &[]);
            grid.attach(&button, (index % 3) as i32, (index / 3) as i32, 1, 1);
            self.connect(&button, move |engine| engine.set_binary_op(op));
        }
        let not = key("NOT", &[]);
        grid.attach(&not, 2, 1, 1, 1);
        self.connect(&not, |engine| engine.apply_unary_op(UnaryOp::BitNot));
    }

    fn set_mode(&self, name: &str) {
        let mode = match name {
            "scientific" => CalcMode::Scientific,
            "programmer" => CalcMode::Programmer,
            "converter" => CalcMode::Converter,
            _ => CalcMode::Basic,
        };
        self.state.engine.borrow_mut().set_mode(mode);
        self.state
            .scientific
            .set_reveal_child(mode == CalcMode::Scientific);
        self.state
            .programmer
            .set_reveal_child(mode == CalcMode::Programmer);
        self.state.bases.set_visible(mode == CalcMode::Programmer);
        self.state
            .stack
            .set_visible_child_name(if mode == CalcMode::Converter {
                "converter"
            } else {
                "pad"
            });
        self.refresh();
    }

    fn refresh(&self) {
        let engine = self.state.engine.borrow();
        let text = if engine.current_input == "Error" {
            gettext("Error")
        } else {
            engine.current_input.clone()
        };
        self.state.result.set_text(&text);
        self.state.expression.set_text(&engine.pending_expression());
        let bases = engine.programmer_representations();
        for (label, value) in self
            .state
            .base_values
            .iter()
            .zip([bases.hex, bases.dec, bases.oct, bases.bin])
        {
            label.set_text(&value);
        }
        let history = &self.state.history;
        history.remove_all();
        self.state
            .history_empty
            .set_visible(engine.history.is_empty());
        for entry in engine.history.iter().rev().take(50) {
            let row = adw::ActionRow::new();
            row.set_title(&glib::markup_escape_text(&entry.result));
            row.set_subtitle(&glib::markup_escape_text(&entry.expression));
            row.add_css_class("numeric");
            history.append(&row);
        }
    }

    fn setup_converter(
        &self,
        category: &gtk::DropDown,
        from_value: &gtk::Entry,
        from_unit: &gtk::DropDown,
        to_value: &gtk::Label,
        to_unit: &gtk::DropDown,
        swap: &gtk::Button,
    ) {
        let convert = {
            let (category, from_value, from_unit, to_value, to_unit) = (
                category.clone(),
                from_value.clone(),
                from_unit.clone(),
                to_value.clone(),
                to_unit.clone(),
            );
            move || {
                let kind = CATEGORIES[(category.selected() as usize).min(CATEGORIES.len() - 1)];
                let text = from_value.text().replace(',', ".");
                match text.trim().parse::<f64>() {
                    Ok(value) => {
                        let result = convert_units(
                            kind,
                            value,
                            from_unit.selected() as usize,
                            to_unit.selected() as usize,
                        );
                        to_value.set_text(&format_number(result));
                        from_value.remove_css_class("error");
                    }
                    Err(_) => {
                        to_value.set_text("");
                        from_value.add_css_class("error");
                    }
                }
            }
        };
        let convert = Rc::new(convert);
        let fill = {
            let (category, from_unit, to_unit, convert) = (
                category.clone(),
                from_unit.clone(),
                to_unit.clone(),
                convert.clone(),
            );
            move || {
                let kind = CATEGORIES[(category.selected() as usize).min(CATEGORIES.len() - 1)];
                let names = unit_names(kind);
                let refs: Vec<&str> = names.iter().map(String::as_str).collect();
                from_unit.set_model(Some(&gtk::StringList::new(&refs)));
                to_unit.set_model(Some(&gtk::StringList::new(&refs)));
                from_unit.set_selected(0);
                to_unit.set_selected(1);
                convert();
            }
        };
        fill();
        category.connect_selected_notify(move |_| fill());
        let update = convert.clone();
        from_value.connect_changed(move |_| update());
        let update = convert.clone();
        from_unit.connect_selected_notify(move |_| update());
        let update = convert.clone();
        to_unit.connect_selected_notify(move |_| update());
        let (from_unit, to_unit) = (from_unit.clone(), to_unit.clone());
        swap.connect_clicked(move |_| {
            let from = from_unit.selected();
            from_unit.set_selected(to_unit.selected());
            to_unit.set_selected(from);
        });
    }

    fn setup_keyboard(&self) {
        let keys = gtk::EventControllerKey::new();
        let this = self.clone();
        keys.connect_key_pressed(move |_, key, _, _| {
            if this.state.stack.visible_child_name().as_deref() == Some("converter") {
                return glib::Propagation::Proceed;
            }
            use gtk::gdk::Key;
            let handled = match key.to_unicode() {
                Some(c @ '0'..='9') => {
                    this.act(|engine| engine.input_digit(c));
                    true
                }
                Some('.') | Some(',') => {
                    this.act(CalculatorEngine::input_dot);
                    true
                }
                Some('+') => {
                    this.act(|engine| engine.set_binary_op(BinaryOp::Add));
                    true
                }
                Some('-') => {
                    this.act(|engine| engine.set_binary_op(BinaryOp::Subtract));
                    true
                }
                Some('*') => {
                    this.act(|engine| engine.set_binary_op(BinaryOp::Multiply));
                    true
                }
                Some('/') => {
                    this.act(|engine| engine.set_binary_op(BinaryOp::Divide));
                    true
                }
                Some('=') => {
                    this.act(CalculatorEngine::equals);
                    true
                }
                _ => match key {
                    Key::Return | Key::KP_Enter => {
                        this.act(CalculatorEngine::equals);
                        true
                    }
                    Key::BackSpace => {
                        this.act(CalculatorEngine::backspace);
                        true
                    }
                    Key::Escape | Key::Delete => {
                        this.act(CalculatorEngine::clear);
                        true
                    }
                    _ => false,
                },
            };
            if handled {
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        self.state.window.add_controller(keys);
    }
}
