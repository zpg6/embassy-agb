//! Embassy executor with automatic power management
//!
//! Uses `HALTCNT` (0x4000301) to enter Halt mode when idle, waking on interrupts.
//! - Halt (bit 7=0): CPU pauses until interrupt, hardware continues
//! - Stop (bit 7=1): Everything pauses (not used by executor)

use core::marker::PhantomData;

pub use embassy_executor::Spawner;
use embassy_executor::raw;

/// Embassy executor with automatic Halt mode when idle
pub struct Executor {
    inner: raw::Executor,
    not_send: PhantomData<*mut ()>,
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

impl Executor {
    /// Create a new executor for GBA
    pub fn new() -> Self {
        Self {
            inner: raw::Executor::new(core::ptr::null_mut()),
            not_send: PhantomData,
        }
    }

    /// Run the executor (never returns)
    ///
    /// Polls tasks continuously, entering Halt mode when idle to save power.
    pub fn run(&'static mut self, init: impl FnOnce(Spawner)) -> ! {
        // Initialize time driver if enabled
        #[cfg(feature = "_time-driver")]
        crate::time_driver::init();

        // Call the init function with our spawner
        init(self.inner.spawner());

        // Main executor loop - poll tasks continuously
        loop {
            unsafe {
                self.inner.poll();
            }

            // Halt until interrupt when idle (power saving)
            agb::halt();
        }
    }
}
