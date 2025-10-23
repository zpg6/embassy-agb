//! Animated ship example with rocket firing
//!
//! Demonstrates async display + async input working together:
//! - VBlank-synchronized rendering for smooth 60Hz display
//! - Timer-based input polling for responsive controls (configurable rate)
//! - Embassy task coordination between input and display
//! - Animated sprite movement with position clamping
//! - Supports holding multiple buttons for diagonal movement
//! - Loading sprites from Aseprite files using `include_aseprite!`
//! - Uses IDLE animation when stationary, FLAME animation when moving
//! - Fire rockets with A button that travel upward until they reach the top
//!
//! Controls:
//! - D-pad moves the animated ship, clamped to screen edges
//! - A button fires rockets from the ship (hold A for rapid fire)
//! - Hold buttons for continuous movement
//! - Hold multiple buttons for diagonal movement
//! - Ship shows FLAME animation when moving, IDLE when stationary
//! Input polling: 60Hz (configurable from 30-120Hz)

#![no_std]
#![no_main]
#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, reexport_test_harness_main = "test_main")]
#![cfg_attr(test, test_runner(agb::test_runner::test_runner))]

extern crate alloc;

use agb::{display::object::Object, include_aseprite};
use alloc::vec::Vec;
use embassy_agb::{
    agb::input::Button,
    input::{AsyncInput, InputConfig, PollingRate},
    sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex},
    Spawner,
};

// Import the ship sprites from the Aseprite file
include_aseprite!(mod ship_sprites, "gfx/ship.aseprite");

// Import the rocket sprites from the Aseprite file
include_aseprite!(mod rocket_sprites, "gfx/rocket.aseprite");

// Shared button state between input task and main loop
#[derive(Clone, Copy, Default)]
struct ButtonState {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    a: bool,
    a_just_pressed: bool,
}

impl ButtonState {
    /// Calculate net movement from current button state
    fn net_movement(&self) -> (i32, i32) {
        let mut x = 0;
        let mut y = 0;

        if self.left {
            x -= 1;
        }
        if self.right {
            x += 1;
        }
        if self.up {
            y -= 1;
        }
        if self.down {
            y += 1;
        }

        (x, y)
    }

    /// Check if any movement button is pressed
    fn is_moving(&self) -> bool {
        self.up || self.down || self.left || self.right
    }
}

// Rocket structure to track individual rockets
#[derive(Clone, Copy)]
struct Rocket {
    x: i32,
    y: i32,
    active: bool,
}

impl Rocket {
    fn new(x: i32, y: i32) -> Self {
        Self { x, y, active: true }
    }

    fn update(&mut self) {
        if self.active {
            self.y -= 8; // Move rocket upward faster
            if self.y < -16 {
                // Remove rocket when it goes off screen
                self.active = false;
            }
        }
    }
}

static BUTTON_STATE: Mutex<CriticalSectionRawMutex, ButtonState> = Mutex::new(ButtonState {
    up: false,
    down: false,
    left: false,
    right: false,
    a: false,
    a_just_pressed: false,
});

// Input task: continuously poll button state and update shared state
#[embassy_executor::task]
async fn input_task(mut input: AsyncInput) {
    let mut prev_a_pressed = false;

    loop {
        // Poll current button state (non-blocking)
        let up_pressed = input.is_pressed(Button::UP);
        let down_pressed = input.is_pressed(Button::DOWN);
        let left_pressed = input.is_pressed(Button::LEFT);
        let right_pressed = input.is_pressed(Button::RIGHT);
        let a_pressed = input.is_pressed(Button::A);

        // Detect A button just pressed (edge detection)
        let a_just_pressed = a_pressed && !prev_a_pressed;
        prev_a_pressed = a_pressed;

        // Update shared state
        {
            let mut state = BUTTON_STATE.lock().await;
            state.up = up_pressed;
            state.down = down_pressed;
            state.left = left_pressed;
            state.right = right_pressed;
            state.a = a_pressed;
            state.a_just_pressed = a_just_pressed;
        }

        // Wait for any button press or release (non-blocking)
        input.wait_for_any_button_press().await;
    }
}

#[embassy_agb::main]
async fn main(spawner: Spawner) -> ! {
    let mut gba = embassy_agb::init(Default::default());

    // Configure input polling at 60Hz
    let input_config = InputConfig {
        poll_rate: PollingRate::Hz60,
    };
    spawner.spawn(embassy_agb::input::input_polling_task(input_config).unwrap());

    let input = gba.input_with_config(input_config);
    let mut display = gba.display();

    // Sprite position and movement
    let mut ship_x = 120; // Center X
    let mut ship_y = 80; // Center Y
    const MOVE_SPEED: i32 = 4;
    const SPRITE_SIZE: i32 = 32; // Ship sprite is 32x32 pixels

    // Screen bounds
    const MIN_X: i32 = 0;
    const MAX_X: i32 = agb::display::WIDTH - SPRITE_SIZE;
    const MIN_Y: i32 = 0;
    const MAX_Y: i32 = agb::display::HEIGHT - SPRITE_SIZE;

    // Animation timing
    let mut frame_count = 0u32;
    const IDLE_ANIMATION_RATE: u32 = 15; // slower animation for idle
    const FLAME_ANIMATION_RATE: u32 = 8; // faster animation for flame

    // Rocket management
    let mut rockets: Vec<Rocket> = Vec::new();
    const MAX_ROCKETS: usize = 12; // Increased limit for faster firing
    let mut fire_cooldown = 0u32;
    const FIRE_RATE: u32 = 4; // Fire every 4 frames when holding A (about 15 rockets per second at 60fps)

    // Spawn input task
    spawner.spawn(input_task(input).unwrap());

    loop {
        // Wait for VBlank: ensures smooth rendering without tearing
        display.wait_for_vblank().await;

        // Get current button state and calculate net movement
        let (move_x, move_y, is_moving, a_pressed, fire_rocket) = {
            let mut state = BUTTON_STATE.lock().await;
            let movement = state.net_movement();
            let fire = state.a_just_pressed;
            let a_held = state.a;
            // Reset the just_pressed flag after reading it
            state.a_just_pressed = false;
            (movement.0, movement.1, state.is_moving(), a_held, fire)
        };

        // Apply movement if any buttons are pressed
        if move_x != 0 || move_y != 0 {
            // Calculate new position with net movement
            ship_x += move_x * MOVE_SPEED;
            ship_y += move_y * MOVE_SPEED;

            // Clamp to screen bounds
            ship_x = ship_x.clamp(MIN_X, MAX_X);
            ship_y = ship_y.clamp(MIN_Y, MAX_Y);
        }

        // Update fire cooldown
        if fire_cooldown > 0 {
            fire_cooldown -= 1;
        }

        // Fire rocket if A button was just pressed or if A is held and cooldown is ready
        if (fire_rocket || (a_pressed && fire_cooldown == 0)) && rockets.len() < MAX_ROCKETS {
            // Fire rocket from the center-top of the ship
            let rocket_x = ship_x + SPRITE_SIZE / 2 - 4; // Center rocket on ship (rocket is 8x8)
            let rocket_y = ship_y; // Start rocket at the top of the ship (no gap)
            rockets.push(Rocket::new(rocket_x, rocket_y));
            fire_cooldown = FIRE_RATE; // Set cooldown for next rocket
        }

        // Update all rockets
        rockets.retain_mut(|rocket| {
            rocket.update();
            rocket.active
        });

        // Choose animation based on movement state
        let (animation_tag, animation_rate) = if is_moving {
            // Use FLAME animation when moving (faster animation)
            (&ship_sprites::FLAME, FLAME_ANIMATION_RATE)
        } else {
            // Use IDLE animation when stationary (slower animation)
            (&ship_sprites::IDLE, IDLE_ANIMATION_RATE)
        };

        // Calculate current animation frame based on the chosen rate
        let animation_frame = (frame_count / animation_rate) as usize;

        // Create sprite object with current animation frame and position
        let mut ship = Object::new(animation_tag.animation_sprite(animation_frame));
        ship.set_pos((ship_x, ship_y));

        // Create rocket objects
        let mut rocket_objects: Vec<Object> = rockets
            .iter()
            .map(|rocket| {
                let mut rocket_obj = Object::new(rocket_sprites::MOVING.animation_sprite(0));
                rocket_obj.set_pos((rocket.x, rocket.y));
                rocket_obj
            })
            .collect();

        // Render the frame
        let mut frame = display.frame().await;
        ship.show(&mut frame);

        // Show all rockets
        for rocket_obj in &mut rocket_objects {
            rocket_obj.show(&mut frame);
        }

        frame.commit();

        frame_count = frame_count.wrapping_add(1);
    }
}
