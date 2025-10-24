//! Simple Mario-like platformer game
//!
//! Jump across platforms to reach the goal!
//!
//! Controls:
//! - LEFT/RIGHT: Move the goof character horizontally
//! - A button: Jump (only when on ground/platform)
//! - Collect all coins (they respawn when you get them all!)
//! - Reach the goal platform to win!
//!
//! Features:
//! - Gravity and jump physics
//! - Multiple platforms to navigate
//! - Coin collection system with auto-respawn
//! - Collision detection
//! - Win condition: reach the goal platform
//! - Custom goof character sprite, grass platforms, and animated coins

#![no_std]
#![no_main]
#![cfg_attr(test, feature(custom_test_frameworks))]
#![cfg_attr(test, reexport_test_harness_main = "test_main")]
#![cfg_attr(test, test_runner(agb::test_runner::test_runner))]

extern crate alloc;

use agb::{display::object::Object, include_aseprite};
use embassy_agb::{
    agb::input::Button,
    input::{AsyncInput, InputConfig, PollingRate},
    sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex},
    Spawner,
};

include_aseprite!(mod goof_sprites, "gfx/goof.aseprite");
include_aseprite!(mod grass_sprites, "gfx/grass.aseprite");
include_aseprite!(mod coin_sprites, "gfx/coin.aseprite");

#[derive(Clone, Copy, Default)]
struct ButtonState {
    left: bool,
    right: bool,
    a: bool,
    a_just_pressed: bool,
}

impl ButtonState {
    fn is_moving(&self) -> bool {
        self.left || self.right
    }
}

static BUTTON_STATE: Mutex<CriticalSectionRawMutex, ButtonState> = Mutex::new(ButtonState {
    left: false,
    right: false,
    a: false,
    a_just_pressed: false,
});

#[embassy_executor::task]
async fn input_task(mut input: AsyncInput) {
    let mut prev_a_pressed = false;

    loop {
        let left_pressed = input.is_pressed(Button::LEFT);
        let right_pressed = input.is_pressed(Button::RIGHT);
        let a_pressed = input.is_pressed(Button::A);

        let a_just_pressed = a_pressed && !prev_a_pressed;
        prev_a_pressed = a_pressed;

        {
            let mut state = BUTTON_STATE.lock().await;
            state.left = left_pressed;
            state.right = right_pressed;
            state.a = a_pressed;
            state.a_just_pressed = a_just_pressed;
        }

        input.wait_for_any_button_press().await;
    }
}

#[derive(Clone, Copy)]
struct Platform {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

impl Platform {
    fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    fn collides_with(&self, px: i32, py: i32, pw: i32, ph: i32) -> bool {
        px + pw > self.x
            && px < self.x + self.width
            && py + ph > self.y
            && py < self.y + self.height
    }

    fn is_on_top(&self, px: i32, py: i32, pw: i32, ph: i32, vy: i32) -> bool {
        vy >= 0
            && px + pw > self.x
            && px < self.x + self.width
            && py + ph >= self.y
            && py + ph <= self.y + 8
    }
}

#[derive(Clone, Copy)]
struct Coin {
    x: i32,
    y: i32,
    collected: bool,
}

impl Coin {
    fn new(x: i32, y: i32) -> Self {
        Self {
            x,
            y,
            collected: false,
        }
    }

    fn collides_with(&self, px: i32, py: i32, pw: i32, ph: i32) -> bool {
        if self.collected {
            return false;
        }
        const COIN_SIZE: i32 = 8;
        px + pw > self.x && px < self.x + COIN_SIZE && py + ph > self.y && py < self.y + COIN_SIZE
    }
}

#[embassy_agb::main]
async fn main(spawner: Spawner) -> ! {
    let mut gba = embassy_agb::init(Default::default());

    let input_config = InputConfig {
        poll_rate: PollingRate::Hz60,
    };
    spawner.spawn(embassy_agb::input::input_polling_task(input_config).unwrap());

    let input = gba.input_with_config(input_config);
    let mut display = gba.display();

    const SPRITE_SIZE: i32 = 8;
    const MOVE_SPEED: i32 = 2;
    const GRAVITY: i32 = 1;
    const JUMP_STRENGTH: i32 = -12;
    const MAX_FALL_SPEED: i32 = 8;

    let mut goof_x = 16;
    let mut goof_y = 0;
    let mut velocity_y = 0;
    let mut on_ground = false;
    let mut game_won = false;
    let mut facing_right = true;

    let platforms = alloc::vec![
        Platform::new(0, 140, 80, 20),
        Platform::new(100, 130, 50, 20),
        Platform::new(160, 110, 50, 20),
        Platform::new(100, 80, 40, 20),
        Platform::new(160, 50, 50, 20),
        Platform::new(200, 140, 40, 20),
    ];

    let goal_platform = Platform::new(200, 140, 40, 20);

    let mut coins = alloc::vec![
        Coin::new(40, 125),
        Coin::new(60, 125),
        Coin::new(120, 115),
        Coin::new(180, 95),
        Coin::new(120, 65),
        Coin::new(180, 35),
        Coin::new(200, 35),
        Coin::new(220, 125),
    ];

    let total_coins = coins.len();
    let mut collected_coins = 0;

    let mut frame_count = 0u32;
    const IDLE_ANIMATION_RATE: u32 = 15;

    spawner.spawn(input_task(input).unwrap());

    loop {
        display.wait_for_vblank().await;

        if !game_won {
            let (move_left, move_right, _is_moving, jump) = {
                let mut state = BUTTON_STATE.lock().await;
                let jump = state.a_just_pressed;
                state.a_just_pressed = false;
                (state.left, state.right, state.is_moving(), jump)
            };

            if move_left {
                goof_x -= MOVE_SPEED;
                facing_right = false;
            }
            if move_right {
                goof_x += MOVE_SPEED;
                facing_right = true;
            }

            goof_x = goof_x.clamp(0, agb::display::WIDTH - SPRITE_SIZE);

            if jump && on_ground {
                velocity_y = JUMP_STRENGTH;
            }

            velocity_y += GRAVITY;
            velocity_y = velocity_y.min(MAX_FALL_SPEED);

            let next_y = goof_y + velocity_y;

            on_ground = false;
            for platform in &platforms {
                if platform.is_on_top(goof_x, next_y, SPRITE_SIZE, SPRITE_SIZE, velocity_y) {
                    goof_y = platform.y - SPRITE_SIZE;
                    velocity_y = 0;
                    on_ground = true;
                    break;
                }
            }

            if !on_ground {
                goof_y = next_y;
            }

            if goof_y > agb::display::HEIGHT {
                goof_x = 16;
                goof_y = 0;
                velocity_y = 0;
            }

            for coin in &mut coins {
                if coin.collides_with(goof_x, goof_y, SPRITE_SIZE, SPRITE_SIZE) {
                    coin.collected = true;
                    collected_coins += 1;
                }
            }

            if collected_coins == total_coins {
                for coin in &mut coins {
                    coin.collected = false;
                }
                collected_coins = 0;
            }

            if goal_platform.collides_with(goof_x, goof_y, SPRITE_SIZE, SPRITE_SIZE) {
                game_won = true;
            }

            let animation_frame = (frame_count / IDLE_ANIMATION_RATE) as usize;

            let animation_tag = if facing_right {
                &goof_sprites::RIGHT
            } else {
                &goof_sprites::LEFT
            };

            let mut goof = Object::new(animation_tag.animation_sprite(animation_frame));
            goof.set_pos((goof_x, goof_y));

            let mut frame = display.frame().await;
            goof.show(&mut frame);

            for platform in &platforms {
                for i in 0..(platform.width / 8) {
                    let mut platform_obj = Object::new(grass_sprites::IDLE.animation_sprite(0));
                    platform_obj.set_pos((platform.x + i * 8, platform.y));
                    platform_obj.show(&mut frame);
                }
            }

            for i in 0..(goal_platform.width / 8) {
                let mut goal_obj = Object::new(goof_sprites::RIGHT.animation_sprite(0));
                goal_obj.set_pos((goal_platform.x + i * 8, goal_platform.y));
                goal_obj.show(&mut frame);
            }

            let coin_animation_frame = (frame_count / 8) as usize;
            for coin in &coins {
                if !coin.collected {
                    let mut coin_obj =
                        Object::new(coin_sprites::IDLE.animation_sprite(coin_animation_frame));
                    coin_obj.set_pos((coin.x, coin.y));
                    coin_obj.show(&mut frame);
                }
            }

            frame.commit();
        } else {
            let animation_frame = (frame_count / 5) as usize;

            let animation_tag = if facing_right {
                &goof_sprites::RIGHT
            } else {
                &goof_sprites::LEFT
            };

            let mut goof = Object::new(animation_tag.animation_sprite(animation_frame));
            goof.set_pos((goof_x, goof_y));

            let mut frame = display.frame().await;
            goof.show(&mut frame);

            for platform in &platforms {
                for i in 0..(platform.width / 8) {
                    let mut platform_obj = Object::new(grass_sprites::IDLE.animation_sprite(0));
                    platform_obj.set_pos((platform.x + i * 8, platform.y));
                    platform_obj.show(&mut frame);
                }
            }

            for i in 0..(goal_platform.width / 8) {
                let mut goal_obj =
                    Object::new(goof_sprites::RIGHT.animation_sprite(animation_frame));
                goal_obj.set_pos((goal_platform.x + i * 8, goal_platform.y));
                goal_obj.show(&mut frame);
            }

            let coin_animation_frame = (frame_count / 8) as usize;
            for coin in &coins {
                if !coin.collected {
                    let mut coin_obj =
                        Object::new(coin_sprites::IDLE.animation_sprite(coin_animation_frame));
                    coin_obj.set_pos((coin.x, coin.y));
                    coin_obj.show(&mut frame);
                }
            }

            frame.commit();
        }

        frame_count = frame_count.wrapping_add(1);
    }
}
