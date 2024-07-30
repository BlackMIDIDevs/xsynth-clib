use crate::{
    convert_event, convert_layer_count, convert_streamparams_to_c, sfids_to_vec,
    XSynth_StreamParams,
};
use xsynth_core::{
    channel::{ChannelConfigEvent, ChannelInitOptions},
    channel_group::SynthEvent,
};
use xsynth_realtime::{config::XSynthRealtimeConfig, RealtimeSynth};

static mut RTSYNTH: Option<RealtimeSynth> = None;

/// Options for initializing the XSynth Realtime module
/// - channels: Number of MIDI channels
/// - drum_channels: Array with the IDs of channels that should only be used for drums
/// - drum_channels_count: Length of the above array
/// - use_threadpool: Whether or not to use XSynth's threadpool feature
/// - fade_out_killing: Whether of not to fade out notes when killed because of the voice limit
/// - render_window_ms: The length of the buffer reader in ms
/// - ignore_range: A range of velocities that will not be played
///         LOBYTE = start (0-127), HIBYTE = end (start-127)
#[repr(C)]
pub struct XSynth_RealtimeConfig {
    pub channels: u32,
    pub drum_channels: *const u32,
    pub drum_channels_count: u32,
    pub use_threadpool: bool,
    pub fade_out_killing: bool,
    pub render_window_ms: f64,
    pub ignore_range: u16,
}

/// Generates the default values for the XSynth_RealtimeConfig struct
/// Default values are:
/// - channels: 16
/// - drum_channels: [9] (MIDI channel 10)
/// - drum_channels_count: 1
/// - use_threadpool: False
/// - fade_out_killing: False
/// - render_window_ms: 10.0ms
/// - ignore_range: 0->0 (Nothing ignored)
#[no_mangle]
pub extern "C" fn XSynth_GenDefault_RealtimeConfig() -> XSynth_RealtimeConfig {
    XSynth_RealtimeConfig {
        channels: 16,
        drum_channels: [9].as_ptr(),
        drum_channels_count: 1,
        use_threadpool: false,
        fade_out_killing: false,
        render_window_ms: 10.0,
        ignore_range: 0,
    }
}

/// A struct that holds all the statistics the realtime module can
/// provide.
/// - voice_count: The amount of active voices
/// - buffer: Number of samples requested in the last read
/// - render_time: Percentage of the renderer load
#[repr(C)]
pub struct XSynth_RealtimeStats {
    pub voice_count: u64,
    pub buffer: i64,
    pub render_time: f64,
}

/// Initializes the XSynth Realtime module with the given configuration.
///
/// --Parameters--
/// - config: The initialization configuration (XSynth_RealtimeConfig struct)
#[no_mangle]
pub extern "C" fn XSynth_Realtime_Init(config: XSynth_RealtimeConfig) {
    unsafe {
        let channel_init_options = ChannelInitOptions {
            fade_out_killing: config.fade_out_killing,
            drums_only: false,
        };

        let drumvec =
            std::slice::from_raw_parts(config.drum_channels, config.drum_channels_count as usize);

        let ignore_range = {
            let low = (config.ignore_range & 255) as u8;
            let high = (config.ignore_range >> 8) as u8;
            low..=high
        };

        let options = XSynthRealtimeConfig {
            channel_init_options,
            render_window_ms: config.render_window_ms,
            channel_count: config.channels,
            drums_channels: Vec::from(drumvec),
            use_threadpool: config.use_threadpool,
            ignore_range,
        };

        RTSYNTH = Some(RealtimeSynth::open_with_default_output(options));
    }
}

/// Sends a MIDI event to the realtime module.
///
/// --Parameters--
/// - channel: The number of the MIDI channel to send the event to (MIDI channel 1 is 0)
/// - event: The type of MIDI event sent (see XSynth_ChannelGroup_SendEvent for available options)
/// - params: Parameters for the event
#[no_mangle]
pub extern "C" fn XSynth_Realtime_SendEvent(channel: u32, event: u16, params: u16) {
    unsafe {
        let ev = convert_event(channel, event, params);

        if let Some(synth) = &mut RTSYNTH {
            synth.send_event(ev);
        }
    }
}

/// Checks if the XSynth Realtime module is loaded.
///
/// --Returns--
/// True if it is loaded, false if it is not.
#[no_mangle]
pub extern "C" fn XSynth_Realtime_IsActive() -> bool {
    unsafe { RTSYNTH.is_some() }
}

/// Returns the audio stream parameters of the realtime module as an
/// XSynth_StreamParams struct. This may be useful when loading a new
/// soundfont which is meant to be used here.
///
/// --Returns--
/// This function returns an XSynth_StreamParams struct.
///
/// --Errors--
/// This function will panic if the realtime module is not loaded.
#[no_mangle]
pub extern "C" fn XSynth_Realtime_GetStreamParams() -> XSynth_StreamParams {
    unsafe {
        if let Some(synth) = &RTSYNTH {
            convert_streamparams_to_c(&synth.stream_params())
        } else {
            panic!("Realtime synth not loaded")
        }
    }
}

/// Returns the statistics of the realtime module as an
/// XSynth_RealtimeStats struct.
///
/// --Returns--
/// This function returns an XSynth_RealtimeStats struct.
///
/// --Errors--
/// This function will panic if the realtime module is not loaded.
#[no_mangle]
pub extern "C" fn XSynth_Realtime_GetStats() -> XSynth_RealtimeStats {
    unsafe {
        if let Some(synth) = &RTSYNTH {
            let stats = synth.get_stats();

            XSynth_RealtimeStats {
                voice_count: stats.voice_count(),
                buffer: stats.buffer().last_samples_after_read(),
                render_time: stats.buffer().average_renderer_load(),
            }
        } else {
            panic!("Realtime synth not loaded")
        }
    }
}

/// Sets the desired layer limit on the realtime module. One layer
/// corresponds to one voice per key per channel.
///
/// --Parameters--
/// - layers: The layer limit (0 = no limit, 1-MAX = limit)
///         Where MAX is the maximum value of an unsigned 64bit integer
#[no_mangle]
pub extern "C" fn XSynth_Realtime_SetLayerCount(layers: u64) {
    unsafe {
        if let Some(synth) = &mut RTSYNTH {
            let layercount = convert_layer_count(layers);

            synth.send_event(SynthEvent::ChannelConfig(
                ChannelConfigEvent::SetLayerCount(layercount),
            ));
        }
    }
}

/// Sets a list of soundfonts to be used in the realtime module. To load
/// a new soundfont, see the XSynth_Soundfont_LoadNew function.
///
/// --Parameters--
/// - sf_ids: Pointer to an array of soundfont IDs
/// - count: The length of the above array
///
/// --Errors--
/// This function will panic if any of the given soundfont IDs is invalid.
#[no_mangle]
pub extern "C" fn XSynth_Realtime_SetSoundfonts(sf_ids: *const u64, count: u64) {
    unsafe {
        let ids = std::slice::from_raw_parts(sf_ids, count as usize);
        let sfvec = sfids_to_vec(ids);

        if let Some(synth) = &mut RTSYNTH {
            synth.send_event(SynthEvent::ChannelConfig(
                ChannelConfigEvent::SetSoundfonts(sfvec),
            ));
        }
    }
}

/// Resets the realtime module. Kills all active notes and resets
/// all control change.
#[no_mangle]
pub extern "C" fn XSynth_Realtime_Reset() {
    unsafe {
        if let Some(synth) = &mut RTSYNTH {
            synth.get_senders().reset_synth();
        }
    }
}

/// Terminates the instance of the realtime module.
#[no_mangle]
pub extern "C" fn XSynth_Realtime_Drop() {
    unsafe {
        RTSYNTH.take();
    }
}
