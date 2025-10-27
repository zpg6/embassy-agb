//! # Sound Test Example
//!
//! Demonstrates how to use the sound mixer in embassy-agb.
//!
//! ## Controls
//! - **A button**: Play a jump sound effect
//!
//! ## Key Points
//! 1. Load WAV files using `include_wav!()` macro
//! 2. Use `gba.peripherals()` to get a convenient wrapper with auto frame handling
//! 3. `wait_frame()` returns frame events with button changes and frame count
//! 4. Use `peripherals.play_sound()` for easy sound playback

#![no_std]
#![no_main]

use agb::include_wav;
use embassy_agb::{agb::sound::mixer::Frequency, Spawner};

/// Load jump sound effect from agb examples
/// The WAV file must be at 10512Hz to match the mixer frequency
static JUMP_SOUND: agb::sound::mixer::SoundData =
    include_wav!("../../agb/agb/examples/sfx/jump.wav");

#[embassy_agb::main]
async fn main(_spawner: Spawner) -> ! {
    let mut gba = embassy_agb::init(Default::default());

    // Get peripherals with convenient frame handling
    // Using Hz10512 provides good quality with low CPU usage
    let mut peripherals = gba.peripherals(Frequency::Hz10512);

    loop {
        // wait_frame() returns events that occurred during the frame
        // This automatically handles input.update(), mixer.frame(), and wait_for_vblank()
        // halting the CPU between frames to conserve power.
        let events = peripherals.wait_frame().await;

        // GAME LOGIC evaluated once per frame:

        // Check button events from the frame context and play sound
        if events.is_pressed(embassy_agb::agb::input::Button::A) {
            let _ = peripherals.play_sound(&JUMP_SOUND);
        }
    }
}
