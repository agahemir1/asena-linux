//! A line graph of the last samples of one or two series, drawn in the
//! widget's CSS color; the second series is drawn fainter.
use gtk::{gdk, glib, gsk, prelude::*, subclass::prelude::*};
use std::cell::{Cell, RefCell};

/// Two minutes at one sample every two seconds.
pub const CAPACITY: usize = 60;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct Graph {
        /// Samples, oldest first.
        pub series: RefCell<Vec<Vec<f64>>>,
        /// Fractions are drawn against 1; rates against the largest sample shown.
        pub rates: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Graph {
        const NAME: &'static str = "AsenaMonitorGraph";
        type Type = super::Graph;
        type ParentType = gtk::Widget;

        fn class_init(class: &mut Self::Class) {
            class.set_css_name("asena-graph");
            class.set_accessible_role(gtk::AccessibleRole::Img);
        }
    }

    impl ObjectImpl for Graph {}

    impl WidgetImpl for Graph {
        fn measure(&self, orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            match orientation {
                gtk::Orientation::Horizontal => (120, 320, -1, -1),
                _ => (96, 96, -1, -1),
            }
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let widget = self.obj();
            let width = widget.width() as f32;
            let height = widget.height() as f32;
            if width < 2.0 || height < 2.0 {
                return;
            }
            let color = widget.color();
            let step = width / (CAPACITY - 1) as f32;
            let series = self.series.borrow();
            let scale = if self.rates.get() {
                series.iter().flatten().copied().fold(1.0, f64::max)
            } else {
                1.0
            };
            for (index, values) in series.iter().enumerate() {
                if values.len() < 2 {
                    continue;
                }
                let start = (CAPACITY.saturating_sub(values.len())) as f32 * step;
                let point = |position: usize, value: f64| {
                    (
                        start + position as f32 * step,
                        height - 1.0 - ((value / scale).clamp(0.0, 1.0) as f32) * (height - 2.0),
                    )
                };
                let line = gsk::PathBuilder::new();
                let area = gsk::PathBuilder::new();
                area.move_to(start, height);
                for (position, value) in values.iter().enumerate() {
                    let (x, y) = point(position, *value);
                    if position == 0 {
                        line.move_to(x, y);
                    } else {
                        line.line_to(x, y);
                    }
                    area.line_to(x, y);
                }
                area.line_to(start + (values.len() - 1) as f32 * step, height);
                area.close();
                let alpha = if index == 0 { 1.0 } else { 0.55 };
                let stroke_color = gdk::RGBA::new(
                    color.red(),
                    color.green(),
                    color.blue(),
                    color.alpha() * alpha,
                );
                if index == 0 {
                    let fill = gdk::RGBA::new(
                        color.red(),
                        color.green(),
                        color.blue(),
                        color.alpha() * 0.15,
                    );
                    snapshot.append_fill(&area.to_path(), gsk::FillRule::Winding, &fill);
                }
                snapshot.append_stroke(&line.to_path(), &gsk::Stroke::new(2.0), &stroke_color);
            }
        }
    }
}

glib::wrapper! {
    pub struct Graph(ObjectSubclass<imp::Graph>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Graph {
    /// `rates` scales to the largest sample instead of to 1.
    pub fn new(rates: bool) -> Self {
        let graph: Self = glib::Object::new();
        graph.imp().rates.set(rates);
        graph.add_css_class("accent");
        graph
    }

    /// Appends one sample per series and drops the oldest beyond capacity.
    pub fn push(&self, samples: &[f64]) {
        let mut series = self.imp().series.borrow_mut();
        series.resize_with(samples.len(), Vec::new);
        for (values, sample) in series.iter_mut().zip(samples) {
            values.push(*sample);
            if values.len() > CAPACITY {
                values.remove(0);
            }
        }
        drop(series);
        self.queue_draw();
    }
}
