use gdnative::prelude::*;

pub mod audio_normalizer;
pub use audio_normalizer::AudioNormalizer;
pub mod pool_byte_array;
fn init(handle: InitHandle) {
    handle.add_class::<audio_normalizer::AudioNormalizer>();
}

godot_init!(init);
