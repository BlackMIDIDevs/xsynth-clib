#![allow(clippy::missing_safety_doc)]

use std::ffi::*;
use std::sync::Arc;

use xsynth_core::{
    channel::{ChannelAudioEvent, ChannelConfigEvent, ChannelInitOptions, ControlEvent},
    channel_group::{ChannelGroup, ChannelGroupConfig},
    soundfont::SoundfontBase,
    AudioPipe, AudioStreamParams,
};

mod consts;
mod soundfont;
pub use consts::*;
use xsynth_realtime::SynthEvent;

struct XSynthGroup {
    id: u64,
    group: ChannelGroup,
}
static mut GROUPS: Vec<XSynthGroup> = Vec::new();

struct Soundfont {
    pub id: u64,
    pub soundfont: Arc<dyn SoundfontBase>,
}
static mut SOUNDFONTS: Vec<Soundfont> = Vec::new();

static mut ID_COUNTER: u64 = 0;

fn next_id() -> u64 {
    unsafe {
        let max = c_ulong::MAX;

        if GROUPS.len() >= max as usize {
            panic!("Max number of groups reached, cannot create more.")
        } else if ID_COUNTER >= max {
            for i in 0..GROUPS.len() as u64 {
                if !GROUPS.iter().any(|g| g.id == i) {
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

/// Parameters of the output audio
/// - sample_rate: Audio sample rate
/// - audio_channels: Number of audio channels (only mono (1) and stereo (2) are supported)
#[repr(C)]
pub struct XSynth_StreamParams {
    pub sample_rate: c_uint,
    pub audio_channels: c_ushort,
}

/// Generates the default values for the XSynth_StreamParams struct
/// Default values are:
/// - sample_rate = 44.1kHz
/// - audio_channels = 2 (stereo)
#[no_mangle]
pub extern "C" fn XSynth_GenDefault_StreamParams() -> XSynth_StreamParams {
    XSynth_StreamParams {
        sample_rate: 44100,
        audio_channels: 2,
    }
}

pub fn convert_streamparams_to_rust(params: XSynth_StreamParams) -> AudioStreamParams {
    AudioStreamParams::new(params.sample_rate, params.audio_channels.into())
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
    pub channels: c_uint,
    pub drum_channels: *const c_uint,
    pub drum_channels_count: c_uint,
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
/// - options: The XSynth_GroupOptions struct which holds all the necessary initialization
///         settings for the channel group. A default configuration can be generated
///         using the XSynth_GenDefault_GroupOptions function.
///
/// --Returns--
/// An unsigned 64bit integer which acts as the ID of the generated channel group.
/// This ID will be necessary to use other XSynth_ChannelGroup_* functions, as
/// they are specific to each group.
///
/// --Errors--
/// This function will panic if the maximum number of active groups is reached (which
/// is about eighteen quintillion).
#[no_mangle]
pub extern "C" fn XSynth_ChannelGroup_Create(options: XSynth_GroupOptions) -> c_ulong {
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

        let id = next_id();
        GROUPS.push(XSynthGroup { id, group: new });
        id
    }
}

/// Returns the active voice count of the desired channel group.
///
/// --Parameters--
/// - id: The ID of the desired channel group
///
/// --Returns--
/// An unsigned long integer of the voice count
///
/// --Errors--
/// This function will panic if the given channel group ID does not exist.
#[no_mangle]
pub extern "C" fn XSynth_ChannelGroup_VoiceCount(id: c_ulong) -> c_ulong {
    unsafe {
        GROUPS
            .iter()
            .find(|g| g.id == id)
            .unwrap_or_else(|| panic!("Group does not exist."))
            .group
            .voice_count()
    }
}

/// Sends a MIDI event to the desired channel group.
///
/// --Parameters--
/// - id: The ID of the desired channel group
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
///
/// --Errors--
/// This function will panic if the given channel group ID does not exist.
#[no_mangle]
pub extern "C" fn XSynth_ChannelGroup_SendEvent(
    id: c_ulong,
    channel: c_uint,
    event: c_uint,
    params: c_uint,
) {
    unsafe {
        let group = &mut GROUPS
            .iter_mut()
            .find(|g| g.id == id)
            .unwrap_or_else(|| panic!("Group does not exist."));

        let ev = match event {
            MIDI_EVENT_NOTEON => {
                let key = (params & 255) as u8;
                let vel = (params >> 8) as u8;
                ChannelAudioEvent::NoteOn { key, vel }
            }
            MIDI_EVENT_NOTEOFF => ChannelAudioEvent::NoteOff {
                key: (params & 255) as u8,
            },
            MIDI_EVENT_ALLNOTESKILLED => ChannelAudioEvent::AllNotesKilled,
            MIDI_EVENT_ALLNOTESOFF => ChannelAudioEvent::AllNotesOff,
            MIDI_EVENT_RESETCONTROL => ChannelAudioEvent::ResetControl,
            MIDI_EVENT_PROGRAMCHANGE => ChannelAudioEvent::ProgramChange((params & 255) as u8),
            MIDI_EVENT_CONTROL => {
                let val1 = (params & 255) as u8;
                let val2 = (params >> 8) as u8;
                ChannelAudioEvent::Control(ControlEvent::Raw(val1, val2))
            }
            MIDI_EVENT_PITCH => ChannelAudioEvent::Control(ControlEvent::PitchBend(params as f32)),
            _ => return,
        };

        group.group.send_event(SynthEvent::Channel(channel, ev));
    }
}

/// Reads audio samples from the desired channel group. The amount of samples
/// determines the time of the current active MIDI events. For example if we
/// send a note on event and read 44100 samples (with a 44.1kHz sample rate),
/// then the note will be audible for 1 second. If after reading those samples
/// we send a note off event for the same key, then on the next read the key
/// will be released. If we don't, then the note will keep playing.
///
/// --Parameters--
/// - id: The ID of the desired channel group
/// - buffer: Pointer to a mutable buffer to receive the audio samples
/// - length: Length of the above buffer, or number of samples to read
///
/// --Errors--
/// This function will panic if the given channel group ID does not exist.
#[no_mangle]
pub unsafe extern "C" fn XSynth_ChannelGroup_ReadSamples(
    id: c_ulong,
    buffer: *mut f32,
    length: c_ulong,
) {
    unsafe {
        if buffer.is_null() {
            return;
        }

        let slc = std::slice::from_raw_parts_mut(buffer, length as usize);

        GROUPS
            .iter_mut()
            .find(|g| g.id == id)
            .unwrap_or_else(|| panic!("Group does not exist."))
            .group
            .read_samples(slc);
    }
}

/// Returns the audio stream parameters of the desired channel group as an
/// XSynth_StreamParams struct. This may be useful when loading a new soundfont
/// which is meant to be used in that channel group.
///
/// --Parameters--
/// - id: The ID of the desired channel group
///
/// --Returns--
/// This function returns an XSynth_StreamParams struct.
///
/// --Errors--
/// This function will panic if the given channel group ID does not exist.
#[no_mangle]
pub extern "C" fn XSynth_ChannelGroup_GetStreamParams(id: c_ulong) -> XSynth_StreamParams {
    unsafe {
        let sp = GROUPS
            .iter()
            .find(|g| g.id == id)
            .unwrap_or_else(|| panic!("Group does not exist."))
            .group
            .stream_params();

        XSynth_StreamParams {
            sample_rate: sp.sample_rate,
            audio_channels: sp.channels.count(),
        }
    }
}

/// Sets the given layer limit for the desired channel group. One layer
/// corresponds to one voice per key per channel.
///
/// --Parameters--
/// - id: The ID of the desired channel group
/// - layers: The layer limit (0 = no limit, 1-MAX = limit)
///         Where MAX is the maximum value of an unsigned 64bit integer
///
/// --Errors--
/// This function will panic if the given channel group ID does not exist.
#[no_mangle]
pub extern "C" fn XSynth_ChannelGroup_SetLayerCount(id: c_ulong, layers: c_ulong) {
    let layercount = match layers {
        0 => None,
        _ => Some(layers as usize),
    };

    unsafe {
        GROUPS
            .iter_mut()
            .find(|g| g.id == id)
            .unwrap_or_else(|| panic!("Group does not exist."))
            .group
            .send_event(SynthEvent::ChannelConfig(
                ChannelConfigEvent::SetLayerCount(layercount),
            ));
    }
}

/// Sets a list of soundfonts to be used in the desired channel group. To load
/// a new soundfont, see the XSynth_Soundfont_LoadNew function.
///
/// --Parameters--
/// - id: The ID of the desired channel group
/// - sf_ids: Pointer to an array of soundfont IDs
/// - count: The length of the above array
///
/// --Errors--
/// This function will panic if the given channel group ID does not exist, or
/// if any of the given soundfont IDs is invalid.
#[no_mangle]
pub unsafe extern "C" fn XSynth_ChannelGroup_SetSoundfonts(
    id: c_ulong,
    sf_ids: *const c_ulong,
    count: c_ulong,
) {
    unsafe {
        let group = &mut GROUPS
            .iter_mut()
            .find(|g| g.id == id)
            .unwrap_or_else(|| panic!("Group does not exist."));

        let ids = std::slice::from_raw_parts(sf_ids, count as usize);

        let sfvec: Vec<Arc<dyn SoundfontBase>> = ids
            .iter()
            .map(|id| {
                let sf = &SOUNDFONTS
                    .iter()
                    .find(|sf| sf.id == *id)
                    .unwrap_or_else(|| panic!("Soundfont does not exist."))
                    .soundfont;
                sf.clone()
            })
            .collect();

        group.group.send_event(SynthEvent::ChannelConfig(
            ChannelConfigEvent::SetSoundfonts(sfvec),
        ));
    }
}

/// Removes all the soundfonts used in the desired channel group.
///
/// --Parameters--
/// - id: The ID of the desired channel group
///
/// --Errors--
/// This function will panic if the given channel group ID does not exist.
#[no_mangle]
pub extern "C" fn XSynth_ChannelGroup_ClearSoundfonts(id: c_ulong) {
    unsafe {
        GROUPS
            .iter_mut()
            .find(|g| g.id == id)
            .unwrap_or_else(|| panic!("Group does not exist."))
            .group
            .send_event(SynthEvent::ChannelConfig(
                ChannelConfigEvent::SetSoundfonts(Vec::new()),
            ));
    }
}

/// Removes the desired channel group.
///
/// --Parameters--
/// - id: The ID of the desired channel group to be removed
#[no_mangle]
pub extern "C" fn XSynth_ChannelGroup_Remove(id: c_ulong) {
    unsafe {
        GROUPS.retain(|group| group.id != id);
    }
}

/// Removes all the active channel groups.
#[no_mangle]
pub extern "C" fn XSynth_ChannelGroup_RemoveAll() {
    unsafe {
        GROUPS.clear();
    }
}
