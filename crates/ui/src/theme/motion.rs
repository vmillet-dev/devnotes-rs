//! The reference's durations, the only ones the interface uses. Anything slower than `ENTER`
//! reads as lag on a tool used in bursts.

/// What follows the pointer: a hover fill, a chip's state, a control appearing under it.
pub const FAST: f32 = 0.12;
/// What enters: a menu, a dialog, a transient bar.
pub const ENTER: f32 = 0.16;
/// A spinner's turn, the one motion that lasts.
pub const TURN: f32 = 0.8;
/// Not a motion: how long a mark left by an action takes to fade, so the eye can find it.
pub const MARK: f32 = 2.0;
