use super::pool_byte_array::PoolByteArray;
use ebur128::EbuR128;
use gdnative::prelude::*;
use lewton::inside_ogg::OggStreamReader;
use std::time::Instant;
#[derive(NativeClass)]
#[inherit(Reference)]
pub struct AudioNormalizer {
    target_loudness: f64,
    split_audio_instrumental: PoolByteArray,
    split_audio_voice: PoolByteArray,
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
            split_audio_instrumental: PoolByteArray::new(ByteArray::new()),
            split_audio_voice: PoolByteArray::new(ByteArray::new()),
        }
    }
}
#[methods]
impl AudioNormalizer {
    #[export]
    pub fn set_target_loudness(&mut self, _owner: &Reference, target_loudness: f64) {
        self.target_loudness = target_loudness
    }

    /// Returns global LUFS loudness for the given OGG file data
    #[export]
    pub fn get_loudness_gobal(&mut self, _owner: &Reference, stream: ByteArray) -> f64 {
        let result = OggStreamReader::new(PoolByteArray::new(stream));
        match result {
            Ok(mut reader) => {
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
                let start = Instant::now();
                while let Some(pck_samples) = reader.read_dec_packet_itl().unwrap() {
                    for chunk in pck_samples.chunks(reader.ident_hdr.audio_channels as usize) {
                        ebu.add_frames_i16(&chunk[0..remapped_size]).unwrap();
                    }
                }
                let loudness = ebu.loudness_global().unwrap();
                let duration = start.elapsed().as_secs();
                godot_print!("Finished processing samples, took {} seconds.", duration);
                return loudness;
            }
            Err(err) => {
                godot_print!("Error loading OGG file for normalization {:?}", err);
            }
        }
        self.target_loudness
    }
    #[export]
    pub fn split_dsc_audio(&mut self, _owner: &Reference, stream: ByteArray) {
        let result = OggStreamReader::new(PoolByteArray::new(stream));
        match result {
            Ok(mut reader) => {
                if reader.ident_hdr.audio_channels < 4 {
                    godot_print!("Expected a file with 4 or more channels");
                    return;
                }
                let spec = hound::WavSpec {
                    channels: 2,
                    sample_rate: reader.ident_hdr.audio_sample_rate,
                    bits_per_sample: 16,
                    sample_format: hound::SampleFormat::Int,
                };

                godot_print!("SAMPLE {}", reader.ident_hdr.audio_sample_rate);

                self.split_audio_instrumental.clear();
                self.split_audio_voice.clear();

                let mut writer_v =
                    hound::WavWriter::new(&mut self.split_audio_voice, spec).unwrap();
                let mut writer =
                    hound::WavWriter::new(&mut self.split_audio_instrumental, spec).unwrap();

                while let Some(pck_samples) = reader.read_dec_packet_itl().unwrap() {
                    let it = pck_samples.chunks(reader.ident_hdr.audio_channels as usize);
                    for chunk in it {
                        let left = chunk.get(0).unwrap();
                        let right = chunk.get(1).unwrap();
                        let left_v = chunk.get(2).unwrap();
                        let right_v = chunk.get(3).unwrap();
                        writer.write_sample(*left).unwrap();
                        writer.write_sample(*right).unwrap();
                        writer_v.write_sample(*left_v).unwrap();
                        writer_v.write_sample(*right_v).unwrap();
                    }
                }
            }
            Err(err) => {
                godot_print!("Error loading OGG file for downsampling {:?}", err);
            }
        }
    }
    #[export]
    pub fn get_voice_audio(&mut self, _owner: &Reference) -> &PoolByteArray {
        &self.split_audio_voice
    }
    #[export]
    pub fn get_instrumental_audio(&mut self, _owner: &Reference) -> &PoolByteArray {
        &self.split_audio_instrumental
    }
}
