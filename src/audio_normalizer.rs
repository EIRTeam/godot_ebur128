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
    ogg_stream_reader: Option<OggStreamReader<PoolByteArray>>,
    ebu: Option<EbuR128>,
    remapped_size: i32,
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
            split_audio_instrumental: PoolByteArray::new(ByteArray::new()),
            split_audio_voice: PoolByteArray::new(ByteArray::new()),
            ogg_stream_reader: None,
            ebu: None,
            remapped_size: 0,
            normalization_result: None,
        }
    }
}
#[methods]
impl AudioNormalizer {
    #[export]
    pub fn set_target_loudness(&mut self, _owner: &Reference, target_loudness: f64) {
        self.target_loudness = target_loudness
    }

    #[export]
    pub fn set_target_ogg(&mut self, _owner: &Reference, stream: ByteArray) {
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

    #[export]
    pub fn work_on_normalization(&mut self, _owner: &Reference) -> bool {
        let ebu = self.ebu.as_mut().unwrap();
        let ogg_stream_reader = self.ogg_stream_reader.as_mut().unwrap();

        let packet = ogg_stream_reader.read_dec_packet_itl().unwrap();

        match packet {
            Some(pck_samples) => {
                for chunk in pck_samples.chunks(ogg_stream_reader.ident_hdr.audio_channels as usize)
                {
                    ebu.add_frames_i16(&chunk[0..self.remapped_size as usize])
                        .unwrap();
                }
                false
            }
            None => {
                let start = Instant::now();
                self.normalization_result = Some(ebu.loudness_global().unwrap());
                let duration = start.elapsed().as_secs();
                godot_print!(
                    "Finished processing global loudness, took {} seconds.",
                    duration
                );
                true
            }
        }
    }

    #[export]
    pub fn get_normalization_result(&mut self, _owner: &Reference) -> f64 {
        match self.normalization_result {
            Some(r) => r,
            None => 0.0,
        }
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

                let start = Instant::now();
                let mut packets: Vec<Vec<i16>> = vec![];
                let mut total = 0;
                while let Some(pck_samples) = reader.read_dec_packet_itl().unwrap() {
                    total += pck_samples.len();
                    packets.push(pck_samples);
                }
                let sample_count_per_writer = total;
                let sample_count_per_writer =
                    sample_count_per_writer / (reader.ident_hdr.audio_channels as usize / 4);
                let sample_count_per_writer = sample_count_per_writer / 2;

                let mut writer_i64 = writer.get_i16_writer(sample_count_per_writer as u32);
                let mut writer_i64_v = writer_v.get_i16_writer(sample_count_per_writer as u32);
                let chunk_size = reader.ident_hdr.audio_channels as usize;
                for pck in packets {
                    let it = pck.chunks_exact(chunk_size);

                    for chunk in it {
                        unsafe {
                            let left = chunk.get_unchecked(0);
                            let right = chunk.get_unchecked(1);
                            let left_v = chunk.get_unchecked(2);
                            let right_v = chunk.get_unchecked(3);
                            writer_i64.write_sample_unchecked(*left);
                            writer_i64.write_sample_unchecked(*right);
                            writer_i64_v.write_sample_unchecked(*left_v);
                            writer_i64_v.write_sample_unchecked(*right_v);
                        }
                    }
                }
                writer_i64.flush().unwrap();
                writer_i64_v.flush().unwrap();
                let duration = start.elapsed().as_secs();
                godot_print!("Finished downsampling, took {} seconds.", duration);
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
