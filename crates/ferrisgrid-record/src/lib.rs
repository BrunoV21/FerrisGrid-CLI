mod macos;
mod recorder;
mod reducer;
mod replay;
mod rolling;
mod sequence;

pub use macos::{
    MacOsEventSource, RecordingPermissionReport, native_event_source, recording_permission_report,
};
pub use recorder::{
    EventSource, EventSourceCapabilities, FakeEventSource, RecordRequest, RecordResult, record,
    render_record_result,
};
pub use reducer::{
    CheckpointReason, ControlEvent, Modifiers, RawInputEvent, SemanticReducer, SemanticStep,
};
pub use replay::{ReplayRequest, ReplayResult, render_replay_result, replay};
pub use sequence::{Sequence, SequenceScreen, SequenceStep, TextMode, parse_sequence};
