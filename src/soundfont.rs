use function_name::named;
use std::{
    ffi::{c_char, CStr},
    path::PathBuf,
    sync::Arc,
};

use xsynth_core::soundfont::{Interpolator, SampleSoundfont, SoundfontInitOptions};

use crate::{
    consts::*, convert_streamparams_to_rust, Soundfont, XSynth_GenDefault_StreamParams,
    XSynth_StreamParams, SOUNDFONTS,
};

static mut ID_COUNTER: u64 = 0;

fn next_id() -> Result<u64, ()> {
    unsafe {
        if SOUNDFONTS.read().unwrap().len() >= MAX_ITEMS as usize {
            return Err(());
        } else if ID_COUNTER >= MAX_ITEMS {
            for i in 0..MAX_ITEMS {
                if !SOUNDFONTS.read().unwrap().iter().any(|g| g.id == i) {
                    return Ok(i);
                }
            }
        } else {
            let id = ID_COUNTER;
            ID_COUNTER += 1;
            return Ok(id);
        }
    }

    Err(())
}

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
/// This function returns the ID of the loaded soundfont, which can be used
/// to send it to a channel group.
///
/// --Errors--
/// This function will error if XSynth has trouble parsing the soundfont or
/// if the maximum number of active groups is reached.
/// Max: 65.535 soundfonts.
#[named]
#[no_mangle]
pub extern "C" fn XSynth_Soundfont_LoadNew(
    path: *const c_char,
    options: XSynth_SoundfontOptions,
) -> u64 {
    unsafe {
        let path = PathBuf::from(CStr::from_ptr(path).to_str().unwrap_or_else(|_| {
            panic!(
                "{} | Error parsing soundfont path: {:?}",
                function_name!(),
                path
            )
        }));

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

        let new = SampleSoundfont::new(path.clone(), stream_params, sfinit).unwrap_or_else(|_| {
            panic!("{} | Error loading soundfont: {:?}", function_name!(), path)
        });

        match next_id() {
            Ok(id) => {
                SOUNDFONTS.write().unwrap().push(Soundfont {
                    id,
                    soundfont: Arc::new(new),
                });
                id
            }
            Err(..) => panic!(
                "{} | Max number of soundfonts reached, cannot create more.",
                function_name!()
            ),
        }
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
pub extern "C" fn XSynth_Soundfont_Remove(id: u64) {
    unsafe {
        SOUNDFONTS.write().unwrap().retain(|group| group.id != id);
    }
}

/// Removes all soundfonts from the ID list. See the documentation of the
/// XSynth_Soundfont_Remove to find information about clearing the memory
/// a soundfont is using.
#[no_mangle]
pub extern "C" fn XSynth_Soundfont_RemoveAll() {
    unsafe {
        SOUNDFONTS.write().unwrap().clear();
    }
}
