//! Sound mixing support for Game Boy Advance
//!
//! This module provides async-friendly wrappers around the agb sound mixer,
//! allowing you to play up to 8 simultaneous sound channels with various
//! frequencies and effects.
//!
//! # Usage
//!
//! 1. Create a mixer with [`InitializedGba::split()`](crate::InitializedGba::split)
//! 2. Load sound data using [`include_wav!`](agb::include_wav)
//! 3. Play sounds with [`AsyncMixer::play_sound()`]
//! 4. Call [`AsyncMixer::frame()`] once per frame before VBlank
//!
//! # Example (Convenient API)
//!
//! ```rust,no_run
//! use agb::sound::mixer::{Frequency, SoundChannel};
//! use agb::include_wav;
//! use embassy_agb::Spawner;
//!
//! static JUMP_SOUND: agb::sound::mixer::SoundData = include_wav!("sfx/jump.wav");
//!
//! #[embassy_agb::main]
//! async fn main(_spawner: Spawner) -> ! {
//!     let mut gba = embassy_agb::init(Default::default());
//!     let mut peripherals = gba.peripherals(Frequency::Hz10512);
//!
//!     loop {
//!         if peripherals.input.is_just_pressed_polling(agb::input::Button::A) {
//!             let channel = SoundChannel::new(JUMP_SOUND);
//!             peripherals.mixer.play_sound(channel);
//!         }
//!
//!         // Automatically handles input.update(), mixer.frame(), and wait_for_vblank()
//!         peripherals.wait_frame().await;
//!     }
//! }
//! ```
//!
//! # Example (Manual Control)
//!
//! For more control over the frame timing, you can use the split API:
//!
//! ```rust,no_run
//! # use agb::sound::mixer::{Frequency, SoundChannel};
//! # use agb::include_wav;
//! # use embassy_agb::Spawner;
//! # static JUMP_SOUND: agb::sound::mixer::SoundData = include_wav!("sfx/jump.wav");
//! #[embassy_agb::main]
//! async fn main(_spawner: Spawner) -> ! {
//!     let mut gba = embassy_agb::init(Default::default());
//!     let (mut mixer, display, mut input) = gba.split(Frequency::Hz10512);
//!
//!     loop {
//!         input.update();
//!         
//!         if input.is_just_pressed_polling(agb::input::Button::A) {
//!             let channel = SoundChannel::new(JUMP_SOUND);
//!             mixer.play_sound(channel);
//!         }
//!
//!         mixer.frame(); // Must call once per frame!
//!         display.wait_for_vblank().await;
//!     }
//! }
//! ```

use agb::sound::mixer::{Frequency, MixerController, SoundChannel};

/// Error type for sound operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoundError;

impl core::fmt::Display for SoundError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Sound operation failed")
    }
}

/// Async-friendly wrapper for the agb sound mixer
///
/// The mixer supports up to 8 simultaneous sound channels and can play
/// both mono and stereo sounds at various sample rates.
///
/// ## Important: Frame Processing
///
/// You **must** call [`frame()`](AsyncMixer::frame) exactly once per frame
/// (60Hz) for proper sound playback. Call it just before waiting for VBlank.
///
/// ## Sound Priorities
///
/// - **High priority**: Use [`SoundChannel::new_high_priority()`](agb::sound::mixer::SoundChannel::new_high_priority)
///   for background music or critical sounds that must always play
/// - **Low priority**: Use [`SoundChannel::new()`](agb::sound::mixer::SoundChannel::new)
///   for sound effects that can be interrupted
///
/// ## Frequencies
///
/// Choose a frequency based on quality vs performance:
/// - [`Frequency::Hz10512`](agb::sound::mixer::Frequency::Hz10512) - Good quality, low CPU usage (recommended)
/// - [`Frequency::Hz18157`](agb::sound::mixer::Frequency::Hz18157) - Better quality, medium CPU usage
/// - [`Frequency::Hz32768`](agb::sound::mixer::Frequency::Hz32768) - Best quality, high CPU usage
///
/// WAV files must be converted to match the chosen frequency.
pub struct AsyncMixer<'a> {
    mixer: agb::sound::mixer::Mixer<'a>,
}

impl<'a> AsyncMixer<'a> {
    pub(crate) fn new(mixer_controller: &'a mut MixerController, frequency: Frequency) -> Self {
        let mixer = mixer_controller.mixer(frequency);
        Self { mixer }
    }

    /// Process one frame of audio
    ///
    /// **IMPORTANT**: This must be called exactly once per frame (60Hz) for proper sound playback.
    /// Call this just before waiting for VBlank.
    ///
    /// Skipping frames will cause audio glitches and crackling. Calling it more than once
    /// per frame is harmless but wastes CPU cycles.
    pub fn frame(&mut self) {
        self.mixer.frame();
    }

    /// Play a sound and return its channel ID
    ///
    /// Returns `Ok(channel_id)` if the sound starts playing, or `Err(SoundError)`
    /// if all channels are busy and the sound has low priority.
    pub fn play_sound(
        &mut self,
        channel: SoundChannel,
    ) -> Result<agb::sound::mixer::ChannelId, SoundError> {
        self.mixer.play_sound(channel).ok_or(SoundError)
    }

    /// Get a reference to a playing channel
    ///
    /// Returns `Some(&mut channel)` if the channel is still playing, or `None`
    /// if it has finished or been replaced.
    pub fn channel(
        &mut self,
        id: &agb::sound::mixer::ChannelId,
    ) -> Option<&mut agb::sound::mixer::SoundChannel> {
        self.mixer.channel(id)
    }

    /// Get access to the underlying mixer for synchronous operations
    pub fn mixer(&mut self) -> &mut agb::sound::mixer::Mixer<'a> {
        &mut self.mixer
    }
}
