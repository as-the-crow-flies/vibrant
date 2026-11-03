use egui::{ProgressBar, Spinner, Ui};

/// Background work shown on the right of the top bar.
#[derive(Default)]
pub struct Status {
    tasks: Vec<Task>,
}

struct Task {
    label: String,
    // `None` for work without a known extent: a spinner instead of a bar.
    progress: Option<f32>,
}

impl Status {
    pub fn progress(&mut self, label: impl Into<String>, fraction: f32) {
        self.tasks.push(Task {
            label: label.into(),
            progress: Some(fraction),
        });
    }

    pub fn busy(&mut self, label: impl Into<String>) {
        self.tasks.push(Task {
            label: label.into(),
            progress: None,
        });
    }

    /// Lays out right to left, so call inside a right-to-left layout.
    pub fn show(&self, ui: &mut Ui) {
        for task in self.tasks.iter().rev() {
            match task.progress {
                Some(fraction) => {
                    ui.add(
                        ProgressBar::new(fraction)
                            .desired_width(200.0)
                            .text(&task.label),
                    );
                }
                None => {
                    ui.label(&task.label);
                    ui.add(Spinner::new());
                }
            }
        }
    }
}
