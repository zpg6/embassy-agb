#![no_std]
// This appears to be needed for testing to work
#![cfg_attr(any(test, feature = "testing"), no_main)]
#![cfg_attr(any(test, feature = "testing"), feature(custom_test_frameworks))]
#![cfg_attr(
    any(test, feature = "testing"),
    test_runner(agb::test_runner::test_runner)
)]
#![cfg_attr(
    any(test, feature = "testing"),
    reexport_test_harness_main = "test_main"
)]
#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

//! # Embassy async support for agb
//!
//! This crate provides async/await support for Game Boy Advance development using the embassy executor.
//! It integrates with the existing agb library to provide async APIs for display, input, sound, and timing.
//!
//! ## Features
//!
//! - Async display operations (VBlank waiting, DMA transfers)
//! - Async input handling (button press events) with automatic polling
//! - Async sound mixing
//! - Embassy time integration with GBA timers
//! - Task spawning and management
//! - Automatic power management via Halt mode
//!
//! ## Example
//!
//! ```rust,no_run
//! #![no_std]
//! #![no_main]
//!
//! use embassy_agb::Spawner;
//! use embassy_agb::agb::sound::mixer::Frequency;
//! use agb::include_wav;
//!
//! static JUMP: agb::sound::mixer::SoundData = include_wav!("jump.wav");
//!
//! #[embassy_agb::main]
//! async fn main(_spawner: Spawner) -> ! {
//!     let mut gba = embassy_agb::init(Default::default());
//!     
//!     // Get peripherals with convenient frame handling
//!     let mut peripherals = gba.peripherals(Frequency::Hz10512);
//!     
//!     loop {
//!         // wait_frame() returns events that occurred during the frame
//!         let events = peripherals.wait_frame().await;
//!         
//!         // Check button events from the frame context
//!         if events.is_pressed(agb::input::Button::A) {
//!             peripherals.play_sound(&JUMP);
//!         }
//!         
//!         // Or access peripherals directly for continuous state
//!         if peripherals.input.is_pressed(agb::input::Button::LEFT) {
//!             // Move left...
//!         }
//!         
//!         // Use frame counter for animations
//!         let animation_frame = (events.frame_count / 8) as usize;
//!     }
//! }
//! ```

// Include generated code
include!(concat!(env!("OUT_DIR"), "/_generated.rs"));

#[cfg(feature = "executor")]
pub use embassy_executor::Spawner;

// Re-export our macros
pub use embassy_agb_macros::{main, task};

#[cfg(feature = "time")]
pub use embassy_time as time;

#[cfg(feature = "time")]
pub use embassy_time::{Duration, Instant, Ticker, Timer};

pub use embassy_futures as futures;
pub use embassy_sync as sync;

// Re-export agb for convenience
pub use agb;

/// Configuration types for embassy-agb
pub mod config;
pub use config::*;

#[cfg(feature = "_time-driver")]
mod time_driver;

#[cfg(feature = "executor")]
mod executor;
#[cfg(feature = "executor")]
pub use executor::*;

/// Async display utilities
pub mod display;
pub mod input;
/// Async sound utilities
pub mod sound;
/// Utility functions and macros
pub mod utils;

/// Internal utilities (do not use directly)
#[doc(hidden)]
pub mod _internal;

/// Initialize the embassy-agb HAL with the given configuration.
///
/// This function must be called once before using any embassy-agb functionality.
/// It initializes the underlying agb library and sets up embassy integration.
///
/// # Example
///
/// ```rust,no_run
/// let gba = embassy_agb::init(Default::default());
/// ```
pub fn init(config: Config) -> InitializedGba {
    // Get the agb instance from internal storage (set by macro)
    let gba = unsafe { _internal::get_agb_instance() };

    // Configure the time driver with user settings
    #[cfg(feature = "_time-driver")]
    time_driver::configure_timer_frequency(config.timer.overflow_amount);

    // Take peripherals
    let peripherals = Peripherals::take();

    InitializedGba {
        gba,
        peripherals,
        _config: config,
    }
}

/// The initialized GBA with embassy integration
pub struct InitializedGba {
    gba: &'static mut agb::Gba,
    #[allow(dead_code)]
    peripherals: Peripherals,
    _config: Config,
}

impl InitializedGba {
    /// Get a convenient peripheral wrapper with automatic frame handling (recommended)
    ///
    /// This is the recommended high-level API for most games. It returns a `GbaPeripherals`
    /// struct that bundles display, mixer, and input together with automatic per-frame updates.
    ///
    /// **Features:**
    /// - Direct field access: `peripherals.display`, `peripherals.mixer`, `peripherals.input`
    /// - Automatic frame handling: `wait_frame()` handles input updates, audio mixing, and VBlank
    /// - Frame events: Returns button presses, releases, and frame counter
    /// - Convenience methods: `play_sound()` and `play_sound_high_priority()`
    ///
    /// For advanced use cases requiring finer control, see [`split()`](Self::split).
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use embassy_agb::agb::sound::mixer::Frequency;
    /// # use agb::include_wav;
    /// # static JUMP: agb::sound::mixer::SoundData = include_wav!("jump.wav");
    /// # async fn example() {
    /// let mut gba = embassy_agb::init(Default::default());
    /// let mut peripherals = gba.peripherals(Frequency::Hz10512);
    ///
    /// loop {
    ///     let events = peripherals.wait_frame().await;
    ///     
    ///     if events.is_pressed(agb::input::Button::A) {
    ///         peripherals.play_sound(&JUMP);
    ///     }
    /// }
    /// # }
    /// ```
    pub fn peripherals(
        &mut self,
        mixer_frequency: agb::sound::mixer::Frequency,
    ) -> GbaPeripherals<'_> {
        GbaPeripherals::new(
            &mut self.gba,
            mixer_frequency,
            input::InputConfig::default(),
        )
    }

    /// Get peripherals with custom input polling configuration
    ///
    /// Same as [`peripherals()`](Self::peripherals) but allows customizing the input
    /// polling rate (default is 60Hz).
    pub fn peripherals_with_input_config(
        &mut self,
        mixer_frequency: agb::sound::mixer::Frequency,
        input_config: input::InputConfig,
    ) -> GbaPeripherals<'_> {
        GbaPeripherals::new(&mut self.gba, mixer_frequency, input_config)
    }

    /// Split the GBA into display, mixer, and input peripherals
    ///
    /// This is the lower-level API that gives you separate components.
    /// Consider using [`peripherals()`](InitializedGba::peripherals) for a more convenient API.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use embassy_agb::agb::sound::mixer::Frequency;
    /// # async fn example() {
    /// let mut gba = embassy_agb::init(Default::default());
    /// let (mut mixer, display, mut input) = gba.split(Frequency::Hz10512);
    ///
    /// loop {
    ///     input.update();
    ///     mixer.frame();
    ///     display.wait_for_vblank().await;
    /// }
    /// # }
    /// ```
    pub fn split(
        &mut self,
        mixer_frequency: agb::sound::mixer::Frequency,
    ) -> (
        sound::AsyncMixer<'_>,
        display::AsyncDisplay<'_>,
        input::AsyncInput,
    ) {
        let mixer = sound::AsyncMixer::new(&mut self.gba.mixer, mixer_frequency);
        let display = display::AsyncDisplay::new(&mut self.gba.graphics);
        let input = input::AsyncInput::new();
        (mixer, display, input)
    }

    /// Split the GBA into display, mixer, and input with custom input configuration
    pub fn split_with_input_config(
        &mut self,
        mixer_frequency: agb::sound::mixer::Frequency,
        input_config: input::InputConfig,
    ) -> (
        sound::AsyncMixer<'_>,
        display::AsyncDisplay<'_>,
        input::AsyncInput,
    ) {
        let mixer = sound::AsyncMixer::new(&mut self.gba.mixer, mixer_frequency);
        let display = display::AsyncDisplay::new(&mut self.gba.graphics);
        let input = input::AsyncInput::with_config(input_config);
        (mixer, display, input)
    }

    /// Get the display peripheral for async operations
    pub fn display(&mut self) -> display::AsyncDisplay<'_> {
        display::AsyncDisplay::new(&mut self.gba.graphics)
    }

    /// Get the mixer peripheral for async operations
    ///
    /// Note: If you need to use display or input after creating the mixer,
    /// use [`split()`](InitializedGba::split) instead to avoid borrow checker issues.
    pub fn mixer(&mut self, frequency: agb::sound::mixer::Frequency) -> sound::AsyncMixer<'_> {
        sound::AsyncMixer::new(&mut self.gba.mixer, frequency)
    }

    /// Get the input peripheral for async operations
    pub fn input(&mut self) -> input::AsyncInput {
        input::AsyncInput::new()
    }

    /// Get the input peripheral for async operations with custom configuration
    pub fn input_with_config(&mut self, config: input::InputConfig) -> input::AsyncInput {
        input::AsyncInput::with_config(config)
    }

    /// Get access to the underlying agb::Gba for compatibility
    pub fn agb(&mut self) -> &mut agb::Gba {
        self.gba
    }
}

/// Frame events returned by [`GbaPeripherals::wait_frame()`]
///
/// Provides information about what happened during the frame.
///
/// ## What's Included
///
/// - **Button presses**: Buttons that transitioned from released to pressed
/// - **Button releases**: Buttons that transitioned from pressed to released
/// - **Frame counter**: Auto-incrementing counter for animations and timing
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameEvents {
    /// Bit flags for buttons that were just pressed this frame
    pressed: u16,
    /// Bit flags for buttons that were just released this frame  
    released: u16,
    /// Frame counter (wraps at u32::MAX)
    pub frame_count: u32,
}

impl FrameEvents {
    /// Check if a specific button was just pressed this frame
    pub fn is_pressed(&self, button: agb::input::Button) -> bool {
        (self.pressed & button.bits() as u16) != 0
    }

    /// Check if a specific button was just released this frame
    pub fn is_released(&self, button: agb::input::Button) -> bool {
        (self.released & button.bits() as u16) != 0
    }

    /// Check if any button was pressed this frame
    pub fn any_pressed(&self) -> bool {
        self.pressed != 0
    }

    /// Check if any button was released this frame
    pub fn any_released(&self) -> bool {
        self.released != 0
    }

    /// Get all buttons that were pressed this frame as a bitmask
    pub fn pressed_buttons(&self) -> u16 {
        self.pressed
    }

    /// Get all buttons that were released this frame as a bitmask
    pub fn released_buttons(&self) -> u16 {
        self.released
    }
}

/// High-level peripheral wrapper with automatic frame handling
///
/// This struct bundles the GBA's display, sound mixer, and input together with
/// automatic per-frame updates. It follows Embassy's design pattern of providing
/// direct field access plus convenient helper methods.
///
/// ## Design
///
/// - **Public fields**: Access `display`, `mixer`, and `input` directly
/// - **Frame synchronization**: `wait_frame()` handles all per-frame updates automatically
/// - **Event-driven**: Returns frame events (button presses/releases, frame counter)
/// - **Zero overhead**: Just a thin wrapper with smart defaults
///
/// ## Usage Pattern
///
/// 1. Call `wait_frame()` once per game loop iteration
/// 2. Handle button events from the returned `FrameEvents`
/// 3. Access peripherals directly when needed (e.g., `peripherals.mixer`)
/// 4. Use convenience methods like `play_sound()` for common operations
///
/// # Example
///
/// ```rust,no_run
/// # use embassy_agb::agb::sound::mixer::{Frequency, SoundChannel};
/// # use agb::include_wav;
/// # static JUMP: agb::sound::mixer::SoundData = include_wav!("jump.wav");
/// # async fn example() {
/// let mut gba = embassy_agb::init(Default::default());
/// let mut peripherals = gba.peripherals(Frequency::Hz10512);
///
/// loop {
///     // wait_frame() returns events that occurred during the frame
///     let events = peripherals.wait_frame().await;
///     
///     // Check button events from the returned context
///     if events.is_pressed(agb::input::Button::A) {
///         peripherals.play_sound(&JUMP);
///     }
///     
///     // Access peripherals directly for continuous state
///     if peripherals.input.is_pressed(agb::input::Button::LEFT) {
///         // Move left...
///     }
///     
///     // Use frame counter for animations
///     let anim_frame = (events.frame_count / 8) as usize;
/// }
/// # }
/// ```
pub struct GbaPeripherals<'a> {
    /// Display peripheral for VBlank and rendering
    pub display: display::AsyncDisplay<'a>,
    /// Sound mixer for audio playback
    pub mixer: sound::AsyncMixer<'a>,
    /// Input peripheral for button handling
    pub input: input::AsyncInput,
    frame_count: u32,
    prev_button_state: u16,
}

impl<'a> GbaPeripherals<'a> {
    fn new(
        gba: &'a mut agb::Gba,
        mixer_frequency: agb::sound::mixer::Frequency,
        input_config: input::InputConfig,
    ) -> Self {
        Self {
            mixer: sound::AsyncMixer::new(&mut gba.mixer, mixer_frequency),
            display: display::AsyncDisplay::new(&mut gba.graphics),
            input: input::AsyncInput::with_config(input_config),
            frame_count: 0,
            prev_button_state: 0,
        }
    }

    /// Wait for the next frame, automatically handling all per-frame updates
    ///
    /// This method:
    /// 1. Updates input state (detects button changes)
    /// 2. Processes one frame of audio mixing
    /// 3. Waits for VBlank (~16.7ms at 60Hz)
    /// 4. Returns frame events (button changes, frame count, etc.)
    ///
    /// Call this once per frame in your game loop.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # async fn example(mut peripherals: embassy_agb::GbaPeripherals<'_>) {
    /// loop {
    ///     let events = peripherals.wait_frame().await;
    ///     
    ///     if events.is_pressed(agb::input::Button::A) {
    ///         // Button A was just pressed this frame
    ///     }
    ///     
    ///     // Use frame_count for animations
    ///     let animation_frame = (events.frame_count / 8) as usize;
    /// }
    /// # }
    /// ```
    pub async fn wait_frame(&mut self) -> FrameEvents {
        self.input.update();

        // Get current button state as raw bits
        let current_state = self.input.button_state_bits();

        // Calculate button changes
        let pressed = current_state & !self.prev_button_state;
        let released = !current_state & self.prev_button_state;

        self.prev_button_state = current_state;

        self.mixer.frame();
        self.display.wait_for_vblank().await;

        let events = FrameEvents {
            pressed,
            released,
            frame_count: self.frame_count,
        };

        self.frame_count = self.frame_count.wrapping_add(1);

        events
    }

    /// Play a sound effect with default priority
    ///
    /// Convenience method that creates a `SoundChannel` and plays it through the mixer.
    /// Returns `Ok(channel_id)` if the sound starts playing, or `Err(SoundError)`
    /// if all channels are busy.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use agb::include_wav;
    /// # static JUMP: agb::sound::mixer::SoundData = include_wav!("jump.wav");
    /// # async fn example(mut peripherals: embassy_agb::GbaPeripherals<'_>) {
    /// let events = peripherals.wait_frame().await;
    ///
    /// if events.is_pressed(agb::input::Button::A) {
    ///     peripherals.play_sound(&JUMP);
    /// }
    /// # }
    /// ```
    pub fn play_sound(
        &mut self,
        sound: &'static agb::sound::mixer::SoundData,
    ) -> Result<agb::sound::mixer::ChannelId, sound::SoundError> {
        let channel = agb::sound::mixer::SoundChannel::new(*sound);
        self.mixer.play_sound(channel)
    }

    /// Play a sound effect with high priority
    ///
    /// High priority sounds will replace low priority sounds if all channels are busy.
    /// Use this for important sounds like background music or critical sound effects.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use agb::include_wav;
    /// # static BGM: agb::sound::mixer::SoundData = include_wav!("music.wav");
    /// # async fn example(mut peripherals: embassy_agb::GbaPeripherals<'_>) {
    /// // Play background music with high priority so it doesn't get interrupted
    /// peripherals.play_sound_high_priority(&BGM);
    /// # }
    /// ```
    pub fn play_sound_high_priority(
        &mut self,
        sound: &'static agb::sound::mixer::SoundData,
    ) -> Result<agb::sound::mixer::ChannelId, sound::SoundError> {
        let channel = agb::sound::mixer::SoundChannel::new_high_priority(*sound);
        self.mixer.play_sound(channel)
    }
}

/// Enable automatic input polling with the given polling rate.
///
/// This function should be called once at startup to automatically spawn
/// the input polling task. If not called, input methods will still work
/// but will use polling-based approach instead of interrupt-driven.
///
/// # Example
///
/// ```rust,no_run
/// use embassy_agb::input::PollingRate;
///
/// #[embassy_agb::main]
/// async fn main(spawner: Spawner) -> ! {
///     let mut gba = embassy_agb::init(Default::default());
///     
///     // Enable automatic input polling at 60Hz
///     embassy_agb::enable_input_polling(&spawner, PollingRate::Hz60);
///     
///     let mut input = gba.input();
///     // ... rest of your code
/// }
/// ```
#[cfg(all(feature = "time", feature = "executor"))]
pub fn enable_input_polling(spawner: &Spawner, rate: input::PollingRate) {
    let config = input::InputConfig::from(rate);
    spawner.must_spawn(input::input_polling_task(config));
}
