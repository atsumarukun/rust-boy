use crate::{audio::Audio, gameboy};
use std::cmp::{max, min};

pub const SAMPLES: usize = 512;
pub const SAMPLE_RATE: u128 = 48000;

const WAVE_DUTY: [[f32; 8]; 4] = [
  [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
  [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0],
  [0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0],
  [0.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0],
];

pub struct Apu {
  enabled: bool,
  nr50: u8,
  nr51: u8,
  cycles: u128,
  fs: u8,
  channel1: Channel1,
  channel2: Channel2,
  channel3: Channel3,
  channel4: Channel4,
  samples: Box<[f32; SAMPLES * 2]>,
  sample_idx: usize,
  audio: Audio,
}

impl Apu {
  pub fn new(audio: Audio) -> Self {
    Self {
      enabled: false,
      nr50: 0,
      nr51: 0,
      cycles: 0,
      fs: 0,
      channel1: Channel1::default(),
      channel2: Channel2::default(),
      channel3: Channel3::default(),
      channel4: Channel4::default(),
      samples: Box::new([0.0; SAMPLES * 2]),
      sample_idx: 0,
      audio,
    }
  }

  pub fn emulate_cycle(&mut self) {
    for _ in 0..4 {
      self.cycles += 1;

      self.channel1.emulate_t_cycle();
      self.channel2.emulate_t_cycle();
      self.channel3.emulate_t_cycle();
      self.channel4.emulate_t_cycle();

      if self.cycles & 0x1FFF == 0 {
        self.channel1.emulate_fs_cycle(self.fs);
        self.channel2.emulate_fs_cycle(self.fs);
        self.channel3.emulate_fs_cycle(self.fs);
        self.channel4.emulate_fs_cycle(self.fs);
        self.cycles = 0;
        self.fs = (self.fs + 1) & 7;
      }

      if self.cycles % (gameboy::CPU_CLOCK_HZ / SAMPLE_RATE) == 0 {
        let left_sample = ((((self.nr51 >> 7) & 0b1) as f32) * self.channel4.dac_output()
          + (((self.nr51 >> 6) & 0b1) as f32) * self.channel3.dac_output()
          + (((self.nr51 >> 5) & 0b1) as f32) * self.channel2.dac_output()
          + (((self.nr51 >> 4) & 0b1) as f32) * self.channel1.dac_output())
          / 4.0;
        let right_sample = ((((self.nr51 >> 3) & 0b1) as f32) * self.channel4.dac_output()
          + (((self.nr51 >> 2) & 0b1) as f32) * self.channel3.dac_output()
          + (((self.nr51 >> 1) & 0b1) as f32) * self.channel2.dac_output()
          + (((self.nr51) & 0b1) as f32) * self.channel1.dac_output())
          / 4.0;
        self.samples[self.sample_idx * 2] = (((self.nr50 >> 4) & 0x7) as f32 / 7.0) * left_sample;
        self.samples[self.sample_idx * 2 + 1] = ((self.nr50 & 0x7) as f32 / 7.0) * right_sample;
        self.sample_idx += 1;
      }

      if self.sample_idx >= SAMPLES {
        self.audio.queue(self.samples.as_ref());
        self.sample_idx = 0;
      }
    }
  }

  pub fn read(&self, addr: u16) -> u8 {
    match addr {
      0xFF24 => self.nr50,
      0xFF25 => self.nr51,
      0xFF26 => {
        self.channel1.enabled as u8
          | ((self.channel2.enabled as u8) << 1)
          | ((self.channel3.enabled as u8) << 2)
          | ((self.channel4.enabled as u8) << 3)
          | 0x70
          | ((self.enabled as u8) << 7)
      }
      0xFF10..=0xFF14 => self.channel1.read_nr1x(addr - 0xFF10),
      0xFF15..=0xFF19 => self.channel2.read_nr2x(addr - 0xFF15),
      0xFF1A..=0xFF1E => self.channel3.read_nr3x(addr - 0xFF1A),
      0xFF1F..=0xFF23 => self.channel4.read_nr4x(addr - 0xFF1F),
      0xFF30..=0xFF3F => self.channel3.wave_ram[(addr - 0xFF30) as usize],
      _ => unreachable!(),
    }
  }

  pub fn write(&mut self, addr: u16, mut val: u8) {
    if !self.enabled
      && ![0xFF11, 0xFF16, 0xFF1B, 0xFF20, 0xFF26].contains(&addr)
      && !(0xFF30..=0xFF3F).contains(&addr)
    {
      return;
    }
    if !self.enabled && [0xFF11, 0xFF16, 0xFF20].contains(&addr) {
      val &= 0b0011_1111;
    }

    match addr {
      0xFF24 => self.nr50 = val,
      0xFF25 => self.nr51 = val,
      0xFF26 => {
        let enabled = val & 0x80 > 0;
        if !enabled && self.enabled {
          for addr in 0xFF10..=0xFF25 {
            self.write(addr, 0x00);
          }
        } else if enabled && !self.enabled {
          self.fs = 0;
          self.channel1.wave_duty_position = 0;
          self.channel2.wave_duty_position = 0;
          self.channel3.wave_duty_position = 0;
        }
        self.enabled = enabled;
      }
      0xFF10..=0xFF14 => self.channel1.write_nr1x(addr - 0xFF10, val),
      0xFF15..=0xFF19 => self.channel2.write_nr2x(addr - 0xFF15, val),
      0xFF1A..=0xFF1E => self.channel3.write_nr3x(addr - 0xFF1A, val),
      0xFF1F..=0xFF23 => self.channel4.write_nr4x(addr - 0xFF1F, val),
      0xFF30..=0xFF3F => self.channel3.wave_ram[(addr - 0xFF30) as usize] = val,
      _ => unreachable!(),
    }
  }
}

#[derive(Default)]
struct Channel1 {
  length_timer: u8,
  dac_enabled: bool,
  enabled: bool,
  frequency: u16,
  length_enabled: bool,
  frequency_timer: u16,
  wave_duty_position: usize,

  wave_duty_pattern: u8,
  period_timer: u8,
  current_volume: u8,

  shadow_frequency: u16,
  is_decrementing: bool,
  sweep_period: u8,
  sweep_shift: u8,
  sweep_timer: u8,
  sweep_enabled: bool,

  initial_volume: u8,
  is_upwards: bool,
  period: u8,
}

impl Channel1 {
  fn emulate_fs_cycle(&mut self, fs: u8) {
    if fs & 0b1 == 0 {
      self.length()
    }
    if fs == 7 {
      self.envelope();
    }
    if fs == 2 || fs == 6 {
      self.sweep();
    }
  }

  fn emulate_t_cycle(&mut self) {
    if self.frequency_timer == 0 {
      self.frequency_timer = (2048 - self.frequency) * 4;
      self.wave_duty_position = (self.wave_duty_position + 1) & 7;
    }
    self.frequency_timer -= 1;
  }

  fn length(&mut self) {
    if self.length_enabled && self.length_timer > 0 {
      self.length_timer -= 1;
      self.enabled &= self.length_timer > 0;
    }
  }

  fn envelope(&mut self) {
    if self.period != 0 {
      if self.period_timer > 0 {
        self.period_timer -= 1;
      }

      if self.period_timer == 0 {
        self.period_timer = self.period;

        if self.current_volume < 0xF && self.is_upwards {
          self.current_volume += 1;
        } else if self.current_volume > 0x0 && !self.is_upwards {
          self.current_volume -= 1;
        }
      }
    }
  }

  fn sweep(&mut self) {
    if self.sweep_timer > 0 {
      self.sweep_timer -= 1;
    }

    if self.sweep_timer == 0 {
      self.sweep_timer = self.sweep_period;

      if self.sweep_enabled {
        self.frequency = self.calculate_frequency();
        self.shadow_frequency = self.frequency;
      }
    }
  }

  fn calculate_frequency(&mut self) -> u16 {
    let new_frequency = if self.is_decrementing {
      if self.shadow_frequency >= (self.shadow_frequency >> self.sweep_shift) {
        self.shadow_frequency - (self.shadow_frequency >> self.sweep_shift)
      } else {
        0
      }
    } else {
      min(
        0x3FF,
        self.shadow_frequency + (self.shadow_frequency >> self.sweep_shift),
      )
    };
    new_frequency
  }

  fn dac_output(&self) -> f32 {
    if self.dac_enabled && self.enabled {
      let ret = WAVE_DUTY[self.wave_duty_pattern as usize][self.wave_duty_position]
        * self.current_volume as f32;
      (ret / 7.5) - 1.0
    } else {
      0.0
    }
  }

  fn read_nr1x(&self, x: u16) -> u8 {
    match x {
      0 => (self.sweep_period << 4) | ((self.is_decrementing as u8) << 3) | self.sweep_shift | 0x80,
      1 => (self.wave_duty_pattern << 6) | 0b0011_1111,
      2 => (self.initial_volume << 4) | ((self.is_upwards as u8) << 3) | self.period,
      3 => 0xFF,
      4 => ((self.length_enabled as u8) << 6) | 0b1011_1111,
      _ => 0xFF,
    }
  }

  fn write_nr1x(&mut self, x: u16, val: u8) {
    match x {
      0 => {
        self.sweep_period = (val >> 4) & 0x07;
        self.is_decrementing = val & 0x08 > 0;
        self.sweep_shift = val & 0x07;
      }
      1 => {
        self.wave_duty_pattern = (val >> 6) & 0b11;
        self.length_timer = 64 - (val & 0x3f);
      }
      2 => {
        self.is_upwards = val & 0x08 > 0;
        self.initial_volume = val >> 4;
        self.period = val & 0x07;
        self.dac_enabled = val & 0b1111_1000 > 0;
        self.enabled &= self.dac_enabled;
      }
      3 => {
        self.frequency = (self.frequency & 0x0700) | val as u16;
      }
      4 => {
        self.frequency = (self.frequency & 0xFF) | (((val & 0x07) as u16) << 8);
        self.length_enabled = val & 0x40 > 0;
        if self.length_timer == 0 {
          self.length_timer = 64;
        }
        let trigger = val & 0x80 > 0;
        if trigger && self.dac_enabled {
          self.enabled = true;
          self.period_timer = self.period;
          self.current_volume = self.initial_volume;
          self.shadow_frequency = self.frequency;
          self.sweep_timer = self.sweep_period;
          self.sweep_enabled = self.sweep_period > 0 || self.sweep_shift > 0;
          self.calculate_frequency();
        }
      }
      _ => {}
    }
  }
}

#[derive(Default)]
struct Channel2 {
  length_timer: u8,
  dac_enabled: bool,
  enabled: bool,
  frequency: u16,
  length_enabled: bool,
  frequency_timer: u16,
  wave_duty_position: usize,

  wave_duty_pattern: u8,
  period_timer: u8,
  current_volume: u8,

  initial_volume: u8,
  is_upwards: bool,
  period: u8,
}

impl Channel2 {
  fn emulate_fs_cycle(&mut self, fs: u8) {
    if fs & 0b1 == 0 {
      self.length()
    }
    if fs == 7 {
      self.envelope();
    }
  }

  fn emulate_t_cycle(&mut self) {
    if self.frequency_timer == 0 {
      self.frequency_timer = (2048 - self.frequency) * 4;
      self.wave_duty_position = (self.wave_duty_position + 1) & 7;
    }
    self.frequency_timer -= 1;
  }

  fn length(&mut self) {
    if self.length_enabled && self.length_timer > 0 {
      self.length_timer -= 1;
      self.enabled &= self.length_timer > 0;
    }
  }

  fn envelope(&mut self) {
    if self.period != 0 {
      if self.period_timer > 0 {
        self.period_timer -= 1;
      }

      if self.period_timer == 0 {
        self.period_timer = self.period;

        if self.current_volume < 0xF && self.is_upwards {
          self.current_volume += 1;
        } else if self.current_volume > 0x0 && !self.is_upwards {
          self.current_volume -= 1;
        }
      }
    }
  }

  fn dac_output(&self) -> f32 {
    if self.dac_enabled && self.enabled {
      let ret = WAVE_DUTY[self.wave_duty_pattern as usize][self.wave_duty_position]
        * self.current_volume as f32;
      (ret / 7.5) - 1.0
    } else {
      0.0
    }
  }

  fn read_nr2x(&self, x: u16) -> u8 {
    match x {
      0 => 0xFF,
      1 => (self.wave_duty_pattern << 6) | 0b0011_1111,
      2 => (self.initial_volume << 4) | ((self.is_upwards as u8) << 3) | self.period,
      3 => 0xFF,
      4 => ((self.length_enabled as u8) << 6) | 0b1011_1111,
      _ => 0xFF,
    }
  }

  fn write_nr2x(&mut self, x: u16, val: u8) {
    match x {
      0 => {}
      1 => {
        self.wave_duty_pattern = (val >> 6) & 0b11;
        self.length_timer = 64 - (val & 0x3f);
      }
      2 => {
        self.is_upwards = val & 0x08 > 0;
        self.initial_volume = val >> 4;
        self.period = val & 0x07;
        self.dac_enabled = val & 0b1111_1000 > 0;
        self.enabled &= self.dac_enabled;
      }
      3 => {
        self.frequency = (self.frequency & 0x0700) | val as u16;
      }
      4 => {
        self.frequency = (self.frequency & 0xFF) | (((val & 0x07) as u16) << 8);
        self.length_enabled = val & 0x40 > 0;
        if self.length_timer == 0 {
          self.length_timer = 64;
        }
        let trigger = val & 0x80 > 0;
        if trigger && self.dac_enabled {
          self.enabled = true;
          self.period_timer = self.period;
          self.current_volume = self.initial_volume;
        }
      }
      _ => {}
    }
  }
}

#[derive(Default)]
struct Channel3 {
  length_timer: u16,
  dac_enabled: bool,
  enabled: bool,
  frequency: u16,
  length_enabled: bool,
  frequency_timer: u16,
  wave_duty_position: usize,

  output_level: u8,
  volume_shift: u8,
  pub wave_ram: Box<[u8; 0x10]>,
}

impl Channel3 {
  fn emulate_fs_cycle(&mut self, fs: u8) {
    if fs & 0b1 == 0 {
      self.length()
    }
  }

  fn emulate_t_cycle(&mut self) {
    if self.frequency_timer == 0 {
      self.frequency_timer = (2048 - self.frequency) * 2;
      self.wave_duty_position = (self.wave_duty_position + 1) & 31;
    }
    self.frequency_timer -= 1;
  }

  fn length(&mut self) {
    if self.length_enabled && self.length_timer > 0 {
      self.length_timer -= 1;
      self.enabled &= self.length_timer > 0;
    }
  }

  fn dac_output(&self) -> f32 {
    if self.dac_enabled && self.enabled {
      let ret = ((0xF
        & (self.wave_ram[self.wave_duty_position >> 1] >> ((self.wave_duty_position & 1) << 2)))
        >> self.volume_shift) as f32;
      (ret / 7.5) - 1.0
    } else {
      0.0
    }
  }

  fn read_nr3x(&self, x: u16) -> u8 {
    match x {
      0 => ((self.dac_enabled as u8) << 7) | 0x7F,
      1 => 0xFF,
      2 => (self.output_level << 5) | 0x9F,
      3 => 0xFF,
      4 => ((self.length_enabled as u8) << 6) | 0b1011_1111,
      _ => 0xFF,
    }
  }

  fn write_nr3x(&mut self, x: u16, val: u8) {
    match x {
      0 => {
        self.dac_enabled = val & 0x80 > 0;
        self.enabled &= self.dac_enabled;
      }
      1 => {
        self.length_timer = 256 - val as u16;
      }
      2 => {
        self.output_level = (val >> 5) & 0x03;
        self.volume_shift = min(4, self.output_level.wrapping_sub(1));
      }
      3 => {
        self.frequency = (self.frequency & 0x0700) | val as u16;
      }
      4 => {
        self.frequency = (self.frequency & 0xFF) | (((val & 0x07) as u16) << 8);
        self.length_enabled = val & 0x40 > 0;
        if self.length_timer == 0 {
          self.length_timer = 256;
        }
        let trigger = val & 0x80 > 0;
        if trigger && self.dac_enabled {
          self.enabled = true;
        }
      }
      _ => {}
    }
  }
}

#[derive(Default)]
struct Channel4 {
  length_timer: u8,
  dac_enabled: bool,
  enabled: bool,
  length_enabled: bool,
  frequency_timer: u32,

  period_timer: u8,
  current_volume: u8,

  initial_volume: u8,
  is_upwards: bool,
  period: u8,

  lfsr: u16,
  shift_amount: usize,
  width_mode: bool,
  divisor_code: u16,
}

impl Channel4 {
  fn emulate_fs_cycle(&mut self, fs: u8) {
    if fs & 0b1 == 0 {
      self.length();
    }
    if fs == 7 {
      self.envelope();
    }
  }

  fn emulate_t_cycle(&mut self) {
    if self.frequency_timer == 0 {
      self.frequency_timer = max(8, (self.divisor_code as u32) << 4) << (self.shift_amount as u32);

      let xor = (self.lfsr & 0b01) ^ ((self.lfsr & 0b10) >> 1);
      self.lfsr = (self.lfsr >> 1) | (xor << 14);
      if self.width_mode {
        self.lfsr &= !(1 << 6);
        self.lfsr |= xor << 6;
      }
    }
    self.frequency_timer -= 1;
  }

  fn length(&mut self) {
    if self.length_enabled && self.length_timer > 0 {
      self.length_timer -= 1;
      self.enabled &= self.length_timer > 0;
    }
  }

  fn envelope(&mut self) {
    if self.period != 0 {
      if self.period_timer > 0 {
        self.period_timer -= 1;
      }

      if self.period_timer == 0 {
        self.period_timer = self.period;

        if self.current_volume < 0xF && self.is_upwards {
          self.current_volume += 1;
        } else if self.current_volume > 0x0 && !self.is_upwards {
          self.current_volume -= 1;
        }
      }
    }
  }

  fn dac_output(&self) -> f32 {
    if self.dac_enabled && self.enabled {
      let ret = (self.lfsr & 1) as f32 * self.current_volume as f32;
      (ret / 7.5) - 1.0
    } else {
      0.0
    }
  }

  fn read_nr4x(&self, x: u16) -> u8 {
    match x {
      0 => 0xFF,
      1 => 0xFF,
      2 => (self.initial_volume << 4) | ((self.is_upwards as u8) << 3) | self.period,
      3 => (self.shift_amount as u8) << 4 | (self.width_mode as u8) << 3 | self.divisor_code as u8,
      4 => ((self.length_enabled as u8) << 6) | 0b1011_1111,
      _ => 0xFF,
    }
  }

  fn write_nr4x(&mut self, x: u16, val: u8) {
    match x {
      0 => {}
      1 => {
        self.length_timer = 64 - (val & 0x3f);
      }
      2 => {
        self.is_upwards = val & 0x08 > 0;
        self.initial_volume = val >> 4;
        self.period = val & 0x07;
        self.dac_enabled = val & 0b1111_1000 > 0;
        self.enabled &= self.dac_enabled;
      }
      3 => {
        self.shift_amount = ((val >> 4) & 0x0F) as usize;
        self.width_mode = val & 0x08 > 0;
        self.divisor_code = (val & 0x07) as u16;
      }
      4 => {
        self.length_enabled = val & 0x40 > 0;
        if self.length_timer == 0 {
          self.length_timer = 64;
        }
        let trigger = val & 0x80 > 0;
        if trigger && self.dac_enabled {
          self.enabled = true;
        }
        if trigger {
          self.lfsr = 0x7FFF;
          self.period_timer = self.period;
          self.current_volume = self.initial_volume;
        }
      }
      _ => {}
    }
  }
}
