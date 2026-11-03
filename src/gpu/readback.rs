use std::{cell::Cell, future::Future, rc::Rc};

/// Runs `task` to completion without blocking. Web can't block its single JS
/// thread, so `task` is scheduled on the browser's microtask queue; native
/// has no such restriction and just runs it to completion on the calling
/// thread. Callers never need to branch on platform.
///
/// Native deliberately doesn't dispatch to a background thread: `task`
/// typically ends in a `device.poll(Wait)`, and running that concurrently
/// with the main thread's own submissions/polls on the same `wgpu::Device`
/// is a real hang risk.
pub fn spawn_task(task: impl Future<Output = ()> + 'static) {
    imp::spawn(task);
}

#[cfg(target_arch = "wasm32")]
mod imp {
    use std::future::Future;

    pub fn spawn(task: impl Future<Output = ()> + 'static) {
        wasm_bindgen_futures::spawn_local(task);
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use std::future::Future;

    pub fn spawn(task: impl Future<Output = ()> + 'static) {
        pollster::block_on(task);
    }
}

/// A GPU readback whose result is fetched via [`spawn_task`] and cached.
/// Works identically on native and web - callers never branch on platform.
pub struct Readback<T> {
    value: Rc<Cell<Option<T>>>,
    pending: Rc<Cell<bool>>,
}

impl<T: Copy + 'static> Default for Readback<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Copy + 'static> Readback<T> {
    pub fn new() -> Self {
        Self {
            value: Rc::new(Cell::new(None)),
            pending: Rc::new(Cell::new(false)),
        }
    }

    /// The most recently completed value, if any. Doesn't start a readback.
    pub fn get(&self) -> Option<T> {
        self.value.get()
    }

    /// Starts a new readback if none is already in flight.
    ///
    /// `task` may still be running when this returns (always true on web,
    /// never true on native). Only call this where it's safe for `task` to
    /// still be reading its source once it eventually runs - not on a path
    /// that's about to destroy that source based on [`Self::get`]'s current
    /// value.
    pub fn refresh(&self, task: impl Future<Output = Option<T>> + 'static) {
        if self.pending.get() {
            return;
        }
        self.pending.set(true);

        let value = self.value.clone();
        let pending = self.pending.clone();
        spawn_task(async move {
            if let Some(result) = task.await {
                value.set(Some(result));
            }
            pending.set(false);
        });
    }
}
