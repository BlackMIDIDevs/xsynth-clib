use std::{
    ffi::{c_char, CStr},
    path::PathBuf,
    sync::Arc,
};

use xsynth_core::soundfont::{Interpolator, SampleSoundfont, SoundfontBase, SoundfontInitOptions};

use crate::{
    consts::*, convert_streamparams_to_rust, XSynth_GenDefault_StreamParams, XSynth_Soundfont,
    XSynth_StreamParams,
};

fn convert_program_value(val: i16) -> Option<u8> {
    if val < 0 {
        None
    } else {
        Some(val as u8)
    }
}

/// Options for loading a new XSynth sample soundfont.
/// - stream_params: Output parameters (see XSynth_StreamParams)
/// - bank: The bank number (0-128) to extract and use from the soundfont
///         A value of -1 means to use all available banks (bank 0 for SFZ)
/// - preset: The preset number (0-127) to extract and use from the soundfont
///         A value of -1 means to use all available presets (preset 0 for SFZ)
/// - linear_release: Whether or not to use a linear release envelope
/// - use_effects: Whether or not to apply audio effects to the soundfont. Currently
///         only affecting the use of the low pass filter. Setting to false may
///         improve performance slightly.
/// - interpolator: The type of interpolator to use for the new soundfont
///         Available values: INTERPOLATION_NEAREST (Nearest Neighbor interpolation),
///         INTERPOLATION_LINEAR (Linear interpolation)
#[repr(C)]
pub struct XSynth_SoundfontOptions {
    pub stream_params: XSynth_StreamParams,
    pub bank: i16,
    pub preset: i16,
    pub linear_release: bool,
    pub use_effects: bool,
    pub interpolator: u16,
}

/// Generates the default values for the XSynth_SoundfontOptions struct
/// Default values are:
/// - stream_params: Defaults for the XSynth_StreamParams struct
/// - bank: -1
/// - preset: -1
/// - linear_release: False
/// - use_effects: True
/// - interpolator: INTERPOLATION_NEAREST
#[no_mangle]
pub extern "C" fn XSynth_GenDefault_SoundfontOptions() -> XSynth_SoundfontOptions {
    XSynth_SoundfontOptions {
        stream_params: XSynth_GenDefault_StreamParams(),
        bank: -1,
        preset: -1,
        linear_release: false,
        use_effects: true,
        interpolator: INTERPOLATION_NEAREST,
    }
}

/// Loads a new XSynth sample soundfont in memory.
///
/// --Parameters--
/// - path: The path of the soundfont to be loaded
/// - options: The soundfont initialization options
///         (XSynth_SoundfontOptions struct)
///
/// --Returns--
/// This function returns the pointer (handle) of the loaded soundfont,
/// which can be used to send it to a channel group or realtime synth.
#[no_mangle]
pub extern "C" fn XSynth_Soundfont_LoadNew(
    path: *const c_char,
    options: XSynth_SoundfontOptions,
) -> *mut XSynth_Soundfont {
    unsafe {
        let path = PathBuf::from(
            CStr::from_ptr(path)
                .to_str()
                .unwrap_or_else(|_| panic!("Error parsing soundfont path: {:?}", path)),
        );

        let sfinit = SoundfontInitOptions {
            bank: convert_program_value(options.bank),
            preset: convert_program_value(options.preset),
            linear_release: options.linear_release,
            use_effects: options.use_effects,
            interpolator: match options.interpolator {
                INTERPOLATION_LINEAR => Interpolator::Linear,
                _ => Interpolator::Nearest,
            },
        };

        let stream_params = convert_streamparams_to_rust(options.stream_params);

        let new = SampleSoundfont::new(path.clone(), stream_params, sfinit)
            .unwrap_or_else(|_| panic!("Error loading soundfont: {:?}", path));
        let new: Arc<dyn SoundfontBase> = Arc::new(new);
        let new = Box::new(new);

        Box::into_raw(new) as *mut XSynth_Soundfont
    }
}

/// Frees the handle of the desired soundfont.
///
/// Keep in mind that this does not free the memory the soundfont is
/// using. To clear the used memory the soundfont has to be unloaded/
/// replaced in the channel groups/realtime synthesizers where it was
/// sent. The following functions can be used for this purpose:
/// - XSynth_ChannelGroup_ClearSoundfonts
/// - XSynth_Realtime_ClearSoundfonts
///
/// To completely free the memory a soundfont is using you first need
/// to clear its handle and then remove it from any other places it is
/// being used.
///
/// --Parameters--
/// - handle: The pointer of the soundfont
#[no_mangle]
pub extern "C" fn XSynth_Soundfont_Remove(handle: *mut XSynth_Soundfont) {
    unsafe {
        let handle = handle as *mut Arc<SampleSoundfont>;
        let _ = Box::from_raw(handle);
    }
}
