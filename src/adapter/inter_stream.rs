//! InterStreamEvent

/// Intermediary StreamEvent
pub enum InterStreamEvent {
	Start,
	Chunk(String),
	End,
}
