use std::{ffi::*, path::PathBuf, sync::Arc};

use xsynth_core::soundfont::{Interpolator, SampleSoundfont, SoundfontInitOptions};

use crate::{
    convert_streamparams_to_rust, Soundfont, XSynth_GenDefault_StreamParams, XSynth_StreamParams,
    SOUNDFONTS,
};

static mut ID_COUNTER: u64 = 0;

fn next_id() -> u64 {
    unsafe {
        let max = c_ulong::MAX;

        if SOUNDFONTS.len() >= max as usize {
            panic!("Max number of soundfonts reached, cannot create more.")
        } else if ID_COUNTER >= max {
            for i in 0..SOUNDFONTS.len() as u64 {
                if !SOUNDFONTS.iter().any(|g| g.id == i) {
                    return i;
                }
            }
        } else {
            let id = ID_COUNTER;
            ID_COUNTER += 1;
            return id;
        }
    }

    0
}

fn convert_value_to_option(val: i16) -> Option<u8> {
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
///         0 = Nearest Neighbor, 1 = Linear
#[repr(C)]
pub struct XSynth_SoundfontOptions {
    pub stream_params: XSynth_StreamParams,
    pub bank: c_short,
    pub preset: c_short,
    pub linear_release: bool,
    pub use_effects: bool,
    pub interpolator: c_uchar,
}

/// Generates the default values for the XSynth_SoundfontOptions struct
/// Default values are:
/// - stream_params: Defaults for the XSynth_StreamParams struct
/// - bank: -1
/// - preset: -1
/// - linear_release: False
/// - use_effects: True
/// - interpolator: 0 (Nearest Neighbor)
#[no_mangle]
pub extern "C" fn XSynth_GenDefault_SoundfontOptions() -> XSynth_SoundfontOptions {
    XSynth_SoundfontOptions {
        stream_params: XSynth_GenDefault_StreamParams(),
        bank: -1,
        preset: -1,
        linear_release: false,
        use_effects: true,
        interpolator: 0,
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
/// This function returns the ID of the loaded soundfont, which can be used
/// to send it to a channel group.
///
/// --Errors--
/// This function will error if XSynth has trouble parsing the soundfont.
#[no_mangle]
pub extern "C" fn XSynth_Soundfont_LoadNew(
    path: *const c_char,
    options: XSynth_SoundfontOptions,
) -> c_ulong {
    unsafe {
        let path = PathBuf::from(CStr::from_ptr(path).to_str().expect("Unexpected error."));

        let sfinit = SoundfontInitOptions {
            bank: convert_value_to_option(options.bank),
            preset: convert_value_to_option(options.preset),
            linear_release: options.linear_release,
            use_effects: options.use_effects,
            interpolator: match options.interpolator {
                1 => Interpolator::Linear,
                _ => Interpolator::Nearest,
            },
        };

        let stream_params = convert_streamparams_to_rust(options.stream_params);

        let new =
            SampleSoundfont::new(path, stream_params, sfinit).expect("Error loading soundfont.");

        let id = next_id();
        SOUNDFONTS.push(Soundfont {
            id,
            soundfont: Arc::new(new),
        });
        id
    }
}

/// Removes the desired soundfont from the ID list.
///
/// Keep in mind that this does not clear the memory the soundfont is
/// using. To free the used memory the soundfont has to be unloaded/
/// replaced in the channel groups where it was sent. The function
/// XSynth_ChannelGroup_ClearSoundfonts can be used for this purpose.
///
/// To completely free the memory a soundfont is using it has to both
/// be removed from the ID list and also from any channel groups using it.
///
/// --Parameters--
/// - id: The ID of the desired soundfont to be removed
#[no_mangle]
pub extern "C" fn XSynth_Soundfont_Remove(id: c_ulong) {
    unsafe {
        SOUNDFONTS.retain(|group| group.id != id);
    }
}

/// Removes all soundfonts from the ID list. See the documentation of the
/// XSynth_Soundfont_Remove to find information about clearing the memory
/// a soundfont is using.
#[no_mangle]
pub extern "C" fn XSynth_Soundfont_RemoveAll() {
    unsafe {
        SOUNDFONTS.clear();
    }
}
