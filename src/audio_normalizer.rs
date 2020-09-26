use super::pool_byte_array::PoolByteArray;
use ebur128::EbuR128;
use gdnative::prelude::*;
use lewton::inside_ogg::OggStreamReader;
#[derive(NativeClass, Default)]
#[inherit(Reference)]
pub struct AudioNormalizer {}

impl AudioNormalizer {
    fn new(_owner: &Reference) -> Self {
        AudioNormalizer {}
    }
}

const TARGET_VOLUME: f64 = -24.0;

#[methods]
impl AudioNormalizer {
    /// Returns global LUFS loudness for the given OGG file data
    #[export]
    pub fn get_loudness_gobal(&mut self, _owner: &Reference, stream: ByteArray) -> f64 {
        let result = OggStreamReader::new(PoolByteArray::new(stream));
        match result {
            Ok(mut reader) => {
                let mut ebu = EbuR128::new(
                    reader.ident_hdr.audio_channels as u32,
                    reader.ident_hdr.audio_sample_rate,
                    ebur128::Mode::all(),
                )
                .unwrap();

                match reader.ident_hdr.audio_channels {
                    1 => ebu.set_channel(0, ebur128::Channel::Center).unwrap(),
                    2 => {
                        ebu.set_channel(0, ebur128::Channel::Left).unwrap();
                        ebu.set_channel(1, ebur128::Channel::Right).unwrap();
                    }
                    _ => {
                        println!(
                            "Unsupported number of channels in provided file {:?}, expected 1 or 2",
                            reader.ident_hdr.audio_channels
                        );
                        return TARGET_VOLUME;
                    }
                }

                while let Some(mut pck_samples) = reader.read_dec_packet_itl().unwrap() {
                    ebu.add_frames_i16(&mut pck_samples).unwrap();
                }

                return ebu.loudness_global().unwrap();
            }
            Err(err) => {
                godot_print!("Error loading OGG file for normalization {:?}", err);
            }
        }
        TARGET_VOLUME
    }
}
