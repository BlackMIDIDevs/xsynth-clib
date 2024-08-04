use crate::{
    convert_event, convert_layer_count, convert_streamparams_to_c, sfids_to_vec, XSynth_Realtime,
    XSynth_Soundfont, XSynth_StreamParams,
};
use xsynth_core::{
    channel::{ChannelConfigEvent, ChannelInitOptions},
    channel_group::SynthEvent,
};
use xsynth_realtime::{RealtimeSynth, XSynthRealtimeConfig};

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
///
/// --Returns--
/// This function will return the pointer (handle) of the created realtime
/// synthesizer. This will be necessary to use other XSynth_Realtime_*
/// functions, for the specific synthesizer instance.
#[no_mangle]
pub extern "C" fn XSynth_Realtime_Create(config: XSynth_RealtimeConfig) -> *mut XSynth_Realtime {
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

        let new = RealtimeSynth::open_with_default_output(options);
        let new = Box::new(new);

        Box::into_raw(new) as *mut XSynth_Realtime
    }
}

/// Sends a MIDI event to the specified realtime synth instance.
///
/// --Parameters--
/// - handle: The pointer of the realtime synthesizer instance
/// - channel: The number of the MIDI channel to send the event to
///         (MIDI channel 1 is 0)
/// - event: The type of MIDI event sent (see XSynth_ChannelGroup_SendEvent
///         for available options)
/// - params: Parameters for the event
#[no_mangle]
pub extern "C" fn XSynth_Realtime_SendEvent(
    handle: *mut XSynth_Realtime,
    channel: u32,
    event: u16,
    params: u16,
) {
    unsafe {
        let ev = convert_event(channel, event, params);
        let synth = (handle as *mut RealtimeSynth).as_mut().unwrap();
        synth.send_event(ev);
    }
}

/// Returns the audio stream parameters of the specified realtime synth
/// instance as an XSynth_StreamParams struct. This may be useful when loading
/// a new soundfont which is meant to be used here.
///
/// --Parameters--
/// - handle: The pointer of the realtime synthesizer instance
///
/// --Returns--
/// This function returns an XSynth_StreamParams struct.
#[no_mangle]
pub extern "C" fn XSynth_Realtime_GetStreamParams(
    handle: *mut XSynth_Realtime,
) -> XSynth_StreamParams {
    unsafe {
        let synth = (handle as *mut RealtimeSynth).as_ref().unwrap();
        let sp = synth.stream_params();

        convert_streamparams_to_c(&sp)
    }
}

/// Returns the statistics of the specified realtime synth instance as an
/// XSynth_RealtimeStats struct.
///
/// --Parameters--
/// - handle: The pointer of the realtime synthesizer instance
///
/// --Returns--
/// This function returns an XSynth_RealtimeStats struct.
#[no_mangle]
pub extern "C" fn XSynth_Realtime_GetStats(handle: *mut XSynth_Realtime) -> XSynth_RealtimeStats {
    unsafe {
        let synth = (handle as *mut RealtimeSynth).as_ref().unwrap();
        let stats = synth.get_stats();

        XSynth_RealtimeStats {
            voice_count: stats.voice_count(),
            buffer: stats.buffer().last_samples_after_read(),
            render_time: stats.buffer().average_renderer_load(),
        }
    }
}

/// Sets the desired layer limit on the specified realtime synth instance.
/// One layer corresponds to one voice per key per channel.
///
/// --Parameters--
/// - handle: The pointer of the realtime synthesizer instance
/// - layers: The layer limit (0 = no limit, 1-MAX = limit)
///         Where MAX is the maximum value of an unsigned 64bit integer
#[no_mangle]
pub extern "C" fn XSynth_Realtime_SetLayerCount(handle: *mut XSynth_Realtime, layers: u64) {
    unsafe {
        let synth = (handle as *mut RealtimeSynth).as_mut().unwrap();
        synth.send_event(SynthEvent::ChannelConfig(
            ChannelConfigEvent::SetLayerCount(convert_layer_count(layers)),
        ));
    }
}

/// Sets a list of soundfonts to be used in the specified realtime synth
/// instance. To load a new soundfont, see the XSynth_Soundfont_LoadNew
/// function.
///
/// --Parameters--
/// - handle: The pointer of the realtime synthesizer instance
/// - sf_ids: Pointer to an array of soundfont IDs
/// - count: The length of the above array
///
/// --Errors--
/// This function will panic if any of the given soundfont IDs is invalid.
#[no_mangle]
pub extern "C" fn XSynth_Realtime_SetSoundfonts(
    handle: *mut XSynth_Realtime,
    sf_ids: *const *mut XSynth_Soundfont,
    count: u64,
) {
    unsafe {
        let ids = std::slice::from_raw_parts(sf_ids, count as usize);
        let sfvec = sfids_to_vec(ids);
        let synth = (handle as *mut RealtimeSynth).as_mut().unwrap();
        synth.send_event(SynthEvent::ChannelConfig(
            ChannelConfigEvent::SetSoundfonts(sfvec),
        ));
    }
}

/// Removes all the soundfonts used in the specified realtime synth instance.
///
/// --Parameters--
/// - handle: The pointer of the channel group instance
#[no_mangle]
pub extern "C" fn XSynth_Realtime_ClearSoundfonts(handle: *mut XSynth_Realtime) {
    unsafe {
        let synth = (handle as *mut RealtimeSynth).as_mut().unwrap();
        synth.send_event(SynthEvent::ChannelConfig(
            ChannelConfigEvent::SetSoundfonts(Vec::new()),
        ));
    }
}

/// Resets the specified realtime synth instance. Kills all active notes
/// and resets all control change.
///
/// --Parameters--
/// - handle: The pointer of the channel group instance
#[no_mangle]
pub extern "C" fn XSynth_Realtime_Reset(handle: *mut XSynth_Realtime) {
    unsafe {
        let synth = (handle as *mut RealtimeSynth).as_mut().unwrap();
        synth.get_senders().reset_synth();
    }
}

/// Drops the specified realtime synth instance.
///
/// --Parameters--
/// - handle: The pointer of the channel group instance
#[no_mangle]
pub extern "C" fn XSynth_Realtime_Drop(handle: *mut XSynth_Realtime) {
    unsafe {
        let h = handle as *mut RealtimeSynth;
        let _ = Box::from_raw(h);
    }
}
