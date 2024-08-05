#![allow(clippy::missing_safety_doc)]
#![allow(static_mut_refs)]

use xsynth_core::{
    channel::{ChannelConfigEvent, ChannelInitOptions},
    channel_group::ChannelGroupConfig,
    AudioPipe,
};

pub(crate) mod consts;
mod realtime;
mod soundfont;
mod utils;
pub use consts::*;
pub use utils::*;
use xsynth_realtime::SynthEvent;

mod handles;
pub use handles::*;

/// Returns the version of XSynth
///
/// --Returns--
/// The XSynth version. For example, 0x010102 (hex), would be version 1.1.2
#[no_mangle]
pub extern "C" fn XSynth_GetVersion() -> u32 {
    env!("XSYNTHVERSION")
        .parse()
        .unwrap_or_else(|_| panic!("Could not parse version number."))
}

/// Parameters of the output audio
/// - sample_rate: Audio sample rate
/// - audio_channels: Number of audio channels
///         Supported: AUDIO_CHANNELS_MONO (mono), AUDIO_CHANNELS_STEREO (stereo)
#[repr(C)]
pub struct XSynth_StreamParams {
    pub sample_rate: u32,
    pub audio_channels: u16,
}

/// Generates the default values for the XSynth_StreamParams struct
/// Default values are:
/// - sample_rate = 44.1kHz
/// - audio_channels = AUDIO_CHANNELS_STEREO
#[no_mangle]
pub extern "C" fn XSynth_GenDefault_StreamParams() -> XSynth_StreamParams {
    XSynth_StreamParams {
        sample_rate: 44100,
        audio_channels: AUDIO_CHANNELS_STEREO,
    }
}

/// Options for initializing a ChannelGroup
/// - stream_params: Output parameters (see XSynth_StreamParams)
/// - channels: Number of MIDI channels
/// - drum_channels: Array with the IDs of channels that should only be used for drums
/// - drum_channels_count: Length of the above array
/// - use_threadpool: Whether or not to use XSynth's threadpool feature
/// - fade_out_killing: Whether of not to fade out notes when killed because of the voice limit
#[repr(C)]
pub struct XSynth_GroupOptions {
    pub stream_params: XSynth_StreamParams,
    pub channels: u32,
    pub drum_channels: *const u32,
    pub drum_channels_count: u32,
    pub use_threadpool: bool,
    pub fade_out_killing: bool,
}

/// Generates the default values for the XSynth_GroupOptions struct
/// Default values are:
/// - stream_params: Defaults for the XSynth_StreamParams struct
/// - channels: 16
/// - drum_channels: [9] (MIDI Channel 10)
/// - drum_channels_count: 1
/// - use_threadpool: True
/// - fade_out_killing: False
#[no_mangle]
pub extern "C" fn XSynth_GenDefault_GroupOptions() -> XSynth_GroupOptions {
    XSynth_GroupOptions {
        stream_params: XSynth_GenDefault_StreamParams(),
        channels: 16,
        drum_channels: [9].as_ptr(),
        drum_channels_count: 1,
        use_threadpool: true,
        fade_out_killing: false,
    }
}

/// Creates a new ChannelGroup. A ChannelGroup is an instance of an XSynth MIDI
/// synthesizer where you can send events and render audio.
///
/// --Parameters--
/// - options: The XSynth_GroupOptions struct which holds all the necessary
///         initialization settings for the channel group. A default configuration
///         can be generated using the XSynth_GenDefault_GroupOptions function.
///
/// --Returns--
/// This function will return the handle of the created channel group. This will be
/// necessary to use other XSynth_ChannelGroup_* functions, as they are specific to
/// each group.
#[no_mangle]
pub extern "C" fn XSynth_ChannelGroup_Create(options: XSynth_GroupOptions) -> XSynth_ChannelGroup {
    unsafe {
        let channel_init_options = ChannelInitOptions {
            fade_out_killing: options.fade_out_killing,
            drums_only: false,
        };

        let drumvec =
            std::slice::from_raw_parts(options.drum_channels, options.drum_channels_count as usize);

        let config = ChannelGroupConfig {
            channel_init_options,
            channel_count: options.channels,
            drums_channels: Vec::from(drumvec),
            audio_params: convert_streamparams_to_rust(options.stream_params),
            use_threadpool: options.use_threadpool,
        };

        let new = ChannelGroup::new(config);
        XSynth_ChannelGroup::from(new)
    }
}

/// Returns the active voice count of the desired channel group.
///
/// --Parameters--
/// - handle: The handle of the channel group instance
///
/// --Returns--
/// A 64bit integer of the voice count
#[no_mangle]
pub extern "C" fn XSynth_ChannelGroup_VoiceCount(handle: XSynth_ChannelGroup) -> u64 {
    handle.as_ref().voice_count()
}

/// Sends a MIDI event to the desired channel group.
///
/// --Parameters--
/// - handle: The handle of the channel group instance
/// - channel: The number of the MIDI channel to send the event to (MIDI channel 1 is 0)
/// - event: The type of MIDI event sent (see below for available options)
/// - params: Parameters for the event
///
/// --Events--
/// - MIDI_EVENT_NOTEON: A MIDI note on event,
///         params: LOBYTE = key number (0-127), HIBYTE = velocity (0-127)
/// - MIDI_EVENT_NOTEOFF: A MIDI note on event
///         params: Key number (0-127)
/// - MIDI_EVENT_ALLNOTESOFF: Release all notes (No parameters)
/// - MIDI_EVENT_ALLNOTESKILLED: Kill all notes (No parameters)
/// - MIDI_EVENT_RESETCONTROL: Reset all control change data (No parameters)
/// - MIDI_EVENT_CONTROL: A MIDI control change event
///         params: LOBYTE = controller number, HIBYTE = controller value
/// - MIDI_EVENT_PROGRAMCHANGE: A MIDI program change event
///         params: preset number
/// - MIDI_EVENT_PITCH: Changes the pitch wheel position
///         params: pitch wheel position (0-16383, 8192=normal/middle)
/// - MIDI_EVENT_FINETUNE: Changes the fine tuning
///         params: fine tune value in cents (0-8192, 4096=normal/middle)
/// - MIDI_EVENT_COARSETUNE: Changes the coarse tuning
///         params: coarse tune value in semitones (0-128, 64=normal/middle)
#[no_mangle]
pub extern "C" fn XSynth_ChannelGroup_SendEvent(
    handle: XSynth_ChannelGroup,
    channel: u32,
    event: u16,
    params: u16,
) {
    let ev = convert_event(channel, event, params);
    handle.as_mut().send_event(ev);
}

/// Reads audio samples from the desired channel group. The amount of samples
/// determines the time of the current active MIDI events. For example if we
/// send a note on event and read 44100 samples (with a 44.1kHz sample rate),
/// then the note will be audible for 1 second. If after reading those samples
/// we send a note off event for the same key, then on the next read the key
/// will be released. If we don't, then the note will keep playing.
///
/// --Parameters--
/// - handle: The handle of the channel group instance
/// - buffer: Pointer to a mutable buffer to receive the audio samples. Each
///         item of the buffer should correspond to an audio sample of type
///         32bit float.
/// - length: Length of the above buffer, or number of samples to read
#[no_mangle]
pub unsafe extern "C" fn XSynth_ChannelGroup_ReadSamples(
    handle: XSynth_ChannelGroup,
    buffer: *mut f32,
    length: u64,
) {
    unsafe {
        if buffer.is_null() {
            return;
        }

        let slc = std::slice::from_raw_parts_mut(buffer, length as usize);
        handle.as_mut().read_samples(slc);
    }
}

/// Returns the audio stream parameters of the desired channel group as an
/// XSynth_StreamParams struct. This may be useful when loading a new soundfont
/// which is meant to be used in that channel group.
///
/// --Parameters--
/// - handle: The handle of the channel group instance
///
/// --Returns--
/// This function returns an XSynth_StreamParams struct.
#[no_mangle]
pub extern "C" fn XSynth_ChannelGroup_GetStreamParams(
    handle: XSynth_ChannelGroup,
) -> XSynth_StreamParams {
    convert_streamparams_to_c(handle.as_ref().stream_params())
}

/// Sets the given layer limit for the desired channel group. One layer
/// corresponds to one voice per key per channel.
///
/// --Parameters--
/// - handle: The handle of the channel group instance
/// - layers: The layer limit (0 = no limit, 1-MAX = limit)
///         Where MAX is the maximum value of an unsigned 64bit integer
#[no_mangle]
pub extern "C" fn XSynth_ChannelGroup_SetLayerCount(handle: XSynth_ChannelGroup, layers: u64) {
    let layercount = convert_layer_count(layers);
    handle.as_mut().send_event(SynthEvent::ChannelConfig(
        ChannelConfigEvent::SetLayerCount(layercount),
    ));
}

/// Sets a list of soundfonts to be used in the desired channel group. To load
/// a new soundfont, see the XSynth_Soundfont_LoadNew function.
///
/// --Parameters--
/// - handle: The handle of the channel group instance
/// - sf_ids: Pointer to an array of soundfont handles
/// - count: The length of the above array
#[no_mangle]
pub unsafe extern "C" fn XSynth_ChannelGroup_SetSoundfonts(
    handle: XSynth_ChannelGroup,
    sf_ids: *const XSynth_Soundfont,
    count: u64,
) {
    unsafe {
        let ids = std::slice::from_raw_parts(sf_ids, count as usize);
        let sfvec = sfids_to_vec(ids);
        handle.as_mut().send_event(SynthEvent::ChannelConfig(
            ChannelConfigEvent::SetSoundfonts(sfvec),
        ));
    }
}

/// Removes all the soundfonts used in the desired channel group.
///
/// --Parameters--
/// - handle: The handle of the channel group instance
#[no_mangle]
pub extern "C" fn XSynth_ChannelGroup_ClearSoundfonts(handle: XSynth_ChannelGroup) {
    handle.as_mut().send_event(SynthEvent::ChannelConfig(
        ChannelConfigEvent::SetSoundfonts(Vec::new()),
    ));
}

/// Drops the desired channel group.
///
/// --Parameters--
/// - handle: The handle of the channel group instance
#[no_mangle]
pub extern "C" fn XSynth_ChannelGroup_Drop(handle: XSynth_ChannelGroup) {
    handle.drop();
}
