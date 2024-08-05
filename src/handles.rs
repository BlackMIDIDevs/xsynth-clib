use std::sync::Arc;
pub use xsynth_core::channel_group::ChannelGroup;
use xsynth_core::soundfont::{SampleSoundfont, SoundfontBase};
pub use xsynth_realtime::RealtimeSynth;

/// Handle of an internal ChannelGroup instance in XSynth.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct XSynth_ChannelGroup {
    pub group: *mut ChannelGroup,
}

impl XSynth_ChannelGroup {
    pub(crate) fn from(group: ChannelGroup) -> Self {
        Self {
            group: Box::into_raw(Box::new(group)),
        }
    }

    pub(crate) fn drop(self) {
        unsafe { drop(Box::from_raw(self.group)) }
    }

    pub(crate) fn as_ref(&self) -> &ChannelGroup {
        unsafe { &*self.group }
    }

    #[allow(clippy::mut_from_ref)]
    pub(crate) fn as_mut(&self) -> &mut ChannelGroup {
        unsafe { &mut *self.group }
    }
}

/// Handle of an internal Soundfont object in XSynth.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct XSynth_Soundfont {
    pub soundfont: *mut Arc<SampleSoundfont>,
}

impl XSynth_Soundfont {
    pub(crate) fn from(sf: Arc<SampleSoundfont>) -> Self {
        Self {
            soundfont: Box::into_raw(Box::new(sf)),
        }
    }

    pub(crate) fn drop(self) {
        unsafe { drop(Box::from_raw(self.soundfont)) }
    }

    pub(crate) fn clone(&self) -> Arc<dyn SoundfontBase> {
        unsafe {
            let sf = &*self.soundfont;
            sf.clone()
        }
    }
}

/// Handle of an internal RealtimeSynth instance in XSynth.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct XSynth_RealtimeSynth {
    pub synth: *mut RealtimeSynth,
}

impl XSynth_RealtimeSynth {
    pub(crate) fn from(synth: RealtimeSynth) -> Self {
        Self {
            synth: Box::into_raw(Box::new(synth)),
        }
    }

    pub(crate) fn drop(self) {
        unsafe { drop(Box::from_raw(self.synth)) }
    }

    pub(crate) fn as_ref(&self) -> &RealtimeSynth {
        unsafe { &*self.synth }
    }

    #[allow(clippy::mut_from_ref)]
    pub(crate) fn as_mut(&self) -> &mut RealtimeSynth {
        unsafe { &mut *self.synth }
    }
}
