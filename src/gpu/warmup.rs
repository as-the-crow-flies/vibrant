use std::{
    cell::{LazyCell, RefCell},
    collections::VecDeque,
    fmt::Display,
    ops::Deref,
    rc::{Rc, Weak},
    time::Duration,
};

use web_time::Instant;

type Cell<T> = LazyCell<T, Box<dyn FnOnce() -> T>>;

/// A lazily built value that is also queued for background warm-up: the
/// first deref builds it if [`step`] hasn't got to it yet.
pub struct Lazy<T>(Rc<Cell<T>>);

impl<T: 'static> Lazy<T> {
    /// Logs how long `build` took at debug level.
    pub fn new(label: impl Display + 'static, build: impl FnOnce() -> T + 'static) -> Self {
        let label = label.to_string();

        let cell = Rc::new(LazyCell::new(Box::new({
            let label = label.clone();
            move || {
                let start = Instant::now();
                let value = build();
                let elapsed = start.elapsed();
                log::debug!("{label}: {:.1} ms", elapsed.as_secs_f64() * 1000.0);
                BUILDS.with_borrow_mut(|builds| builds.push((label, elapsed)));
                value
            }
        }) as Box<dyn FnOnce() -> T>));

        let weak: Weak<dyn Force> = Rc::downgrade(&cell) as Weak<Cell<T>>;
        QUEUE.with_borrow_mut(|queue| {
            queue.pending.push_back(weak);
            queue.total += 1;
        });

        Self(cell)
    }
}

impl<T> Deref for Lazy<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

trait Force {
    fn force(&self);
}

impl<T> Force for Cell<T> {
    fn force(&self) {
        LazyCell::force(self);
    }
}

#[derive(Default)]
struct Queue {
    pending: VecDeque<Weak<dyn Force>>,
    // Queued since the queue was last empty, for progress.
    total: usize,
}

thread_local! {
    static QUEUE: RefCell<Queue> = RefCell::default();
    static BUILDS: RefCell<Vec<(String, Duration)>> = RefCell::default();
}

/// Builds the next queued value, skipping ones that were dropped.
pub fn step() {
    let next = QUEUE.with_borrow_mut(|queue| {
        let next = std::iter::from_fn(|| queue.pending.pop_front()).find_map(|weak| weak.upgrade());
        if queue.pending.is_empty() {
            queue.total = 0;
        }
        next
    });

    // Outside the borrow: building may register new lazies.
    if let Some(value) = next {
        value.force();
    }
}

pub fn pending() -> bool {
    QUEUE.with_borrow(|queue| !queue.pending.is_empty())
}

/// `(built, total)` while values are queued.
pub fn progress() -> Option<(usize, usize)> {
    QUEUE.with_borrow(|queue| {
        (!queue.pending.is_empty()).then(|| (queue.total - queue.pending.len(), queue.total))
    })
}

/// Every build since the last call, with how long it took.
pub fn take_builds() -> Vec<(String, Duration)> {
    BUILDS.take()
}
