//! The task that plays what [`SoundSystem`](crate::SoundSystem) asks for, and keeps a connection to the audio device.

mod connection;
mod output;
mod output_device;
#[allow(clippy::module_inception)]
mod sound_manager;
mod sound_manager_message;

pub(crate) use output_device::OutputDevice;
pub(crate) use sound_manager::SoundManager;
pub(crate) use sound_manager_message::SoundManagerMessage;
