use sdl2::{
  audio::{AudioQueue, AudioSpecDesired},
  Sdl,
};
use std::time;

use crate::apu::{SAMPLES, SAMPLE_RATE};

pub struct Audio {
  audio_queue: AudioQueue<f32>,
}

impl Audio {
  pub fn new(sdl: &Sdl) -> Audio {
    let audio = sdl
      .audio()
      .expect("faild to initialize SDL audio subsystem");
    let audio_queue: AudioQueue<f32> = audio
      .open_queue(
        None,
        &AudioSpecDesired {
          freq: Some(SAMPLE_RATE as i32),
          channels: Some(2),
          samples: Some(SAMPLES as u16 * 2),
        },
      )
      .expect("faild to create audio queue");
    audio_queue.resume();
    Self { audio_queue }
  }

  pub fn queue(&mut self, buffer: &[f32]) {
    while self.audio_queue.size() > 8192 {
      std::thread::sleep(time::Duration::from_millis(1));
    }
    self.audio_queue.queue_audio(buffer).unwrap();
  }
}
