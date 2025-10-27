/// Configuration for embassy-agb initialization
#[derive(Debug, Clone, Default)]
pub struct Config {
    /// Timer configuration for the embassy time driver
    pub timer: TimerConfig,
}

/// Timer configuration for embassy time driver
///
/// GBA has four 16-bit timers (0-3). Embassy uses one with Divider256 (65.536kHz).
/// Timers 0-1 often used by sound system, so Timer 2 is default.
#[derive(Debug, Clone)]
pub struct TimerConfig {
    /// Which timer to use (default: Timer2)
    pub timer_number: TimerNumber,

    /// Timer overflow amount - lower = better precision, more CPU overhead
    ///
    /// At 65.536kHz: 4=~61μs, 16=~244μs, 64=~1ms (default), 256=~4ms, 1024=~16ms
    pub overflow_amount: u16,
}

impl Default for TimerConfig {
    fn default() -> Self {
        Self {
            timer_number: TimerNumber::Timer2,
            overflow_amount: 64, // ~1ms
        }
    }
}

/// GBA timer selection (Timer 0-1 often used by sound)
#[derive(Debug, Clone, Copy)]
pub enum TimerNumber {
    /// Timer 0 (0x4000100) - IE/IF bit 3, often used by sound system
    Timer0,
    /// Timer 1 (0x4000104) - IE/IF bit 4, often used by sound system
    Timer1,
    /// Timer 2 (0x4000108) - IE/IF bit 5 (default)
    Timer2,
    /// Timer 3 (0x400010C) - IE/IF bit 6
    Timer3,
}
