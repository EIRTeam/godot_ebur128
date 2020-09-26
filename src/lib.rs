use gdnative::prelude::*;

pub mod audio_normalizer;
pub use audio_normalizer::AudioNormalizer;
mod pool_byte_array;
fn init(handle: InitHandle) {
    handle.add_class::<audio_normalizer::AudioNormalizer>();
}

// Macros that create the entry-points of the dynamic library.
godot_gdnative_init!();
godot_nativescript_init!(init);
godot_gdnative_terminate!();
