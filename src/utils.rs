use crate::{consts::*, XSynth_StreamParams, SOUNDFONTS};
use std::sync::Arc;
use xsynth_core::{
    channel::{ChannelAudioEvent, ControlEvent},
    soundfont::SoundfontBase,
    AudioStreamParams,
};
use xsynth_realtime::SynthEvent;

pub fn convert_streamparams_to_rust(params: XSynth_StreamParams) -> AudioStreamParams {
    AudioStreamParams::new(params.sample_rate, params.audio_channels.into())
}

pub fn convert_streamparams_to_c(params: &AudioStreamParams) -> XSynth_StreamParams {
    XSynth_StreamParams {
        sample_rate: params.sample_rate,
        audio_channels: params.channels.count(),
    }
}

pub fn convert_event(channel: u32, event: u16, params: u16) -> SynthEvent {
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
        _ => panic!("Unexpected MIDI event."),
    };

    SynthEvent::Channel(channel, ev)
}

pub unsafe fn sfids_to_vec(ids: &[u64]) -> Vec<Arc<dyn SoundfontBase>> {
    ids.iter()
        .map(|id| {
            let sf = &SOUNDFONTS
                .iter()
                .find(|sf| sf.id == *id)
                .unwrap_or_else(|| panic!("Soundfont does not exist."))
                .soundfont;
            sf.clone()
        })
        .collect()
}

pub fn convert_layer_count(layers: u64) -> Option<usize> {
    match layers {
        0 => None,
        _ => Some(layers as usize),
    }
}
