use super::pool_byte_array::PoolByteArray;
use ebur128::EbuR128;
use gdnative::prelude::*;
use lewton::inside_ogg::OggStreamReader;
use std::time::Instant;
#[derive(NativeClass)]
#[inherit(Reference)]
pub struct AudioNormalizer {
    target_loudness: f64,
    ogg_stream_reader: Option<OggStreamReader<PoolByteArray>>,
    ebu: Option<EbuR128>,
    remapped_size: i32,
    decode_start: std::time::Instant,
    normalization_result: Option<f64>,
}

pub fn clamp(val: u8, min: u8, max: u8) -> u8 {
    if val < min {
        min
    } else if val > max {
        max
    } else {
        val
    }
}

impl AudioNormalizer {
    fn new(_owner: &Reference) -> Self {
        AudioNormalizer {
            target_loudness: -24.0,
            ogg_stream_reader: None,
            ebu: None,
            remapped_size: 0,
            decode_start: Instant::now(),
            normalization_result: None,
        }
    }
}

#[methods]
impl AudioNormalizer {
    #[godot]
    pub fn set_target_loudness(&mut self, #[base] _owner: &Reference, target_loudness: f64) {
        self.target_loudness = target_loudness
    }

    #[godot]
    pub fn set_target_ogg(&mut self, #[base] _owner: &Reference, stream: ByteArray) {
        self.decode_start = Instant::now();
        
        let result = OggStreamReader::new(PoolByteArray::new(stream));

        match result {
            Ok(reader) => {
                let mut ebu = EbuR128::new(
                    clamp(reader.ident_hdr.audio_channels, 0, 2) as u32,
                    reader.ident_hdr.audio_sample_rate,
                    ebur128::Mode::all(),
                )
                .unwrap();

                let remapped_size = match reader.ident_hdr.audio_channels {
                    1 => {
                        ebu.set_channel(0, ebur128::Channel::Center).unwrap();
                        1
                    }
                    2 => {
                        ebu.set_channel(0, ebur128::Channel::Left).unwrap();
                        ebu.set_channel(1, ebur128::Channel::Right).unwrap();
                        2
                    }
                    _ => {
                        println!(
                            "Unsupported number of channels in provided file {:?}, expected 1 or 2, looking at first two channels anyways",
                            reader.ident_hdr.audio_channels
                        );
                        ebu.set_channel(0, ebur128::Channel::Left).unwrap();
                        ebu.set_channel(1, ebur128::Channel::Right).unwrap();
                        2
                    }
                };
                self.ogg_stream_reader = Some(reader);
                self.ebu = Some(ebu);
                self.remapped_size = remapped_size;
            }
            Err(err) => {
                godot_print!("Error loading OGG file for normalization {:?}", err);
            }
        }
    }

    #[godot]
    pub fn work_on_normalization(&mut self, #[base] _owner: &Reference) -> bool {
        let ebu = self.ebu.as_mut().unwrap();
        let ogg_stream_reader = self.ogg_stream_reader.as_mut().unwrap();

        let packet = ogg_stream_reader.read_dec_packet_itl().unwrap();


        let file_channel_count: usize = ogg_stream_reader.ident_hdr.audio_channels as usize;
        let target_channel_count: usize = self.remapped_size as usize;

        match packet {
            Some(pck_samples) => {
                let remapped_samples: Vec<i16> = pck_samples.windows(target_channel_count).step_by(file_channel_count).flatten().copied().collect();
                ebu.add_frames_i16(&remapped_samples)
                    .unwrap();
                false
            }
            None => {
                self.normalization_result = Some(ebu.loudness_global().unwrap());
                let duration = self.decode_start.elapsed();
                godot_print!(
                    "Finished processing global loudness, took {:.2?} millis.",
                    duration
                );
                true
            }
        }
    }

    #[godot]
    pub fn get_normalization_result(&mut self, #[base] _owner: &Reference) -> f64 {
        match self.normalization_result {
            Some(r) => r,
            None => 0.0,
        }
    }
}
