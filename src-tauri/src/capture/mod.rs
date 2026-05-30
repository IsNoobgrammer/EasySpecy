//! Screen capture module
//! Phase 0: API stubs for screen enumeration
//! Phase 1: full capture implementation using native OS APIs

/// Represents a capturable display/monitor
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DisplayInfo {
    pub id: u32,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub is_primary: bool,
}

/// Enumerate available displays
/// Phase 0: returns placeholder. Phase 1: uses native APIs.
pub fn get_displays() -> Vec<DisplayInfo> {
    // TODO Phase 1: use Windows Graphics Capture API / ScreenCaptureKit / PipeWire
    vec![DisplayInfo {
        id: 0,
        name: "Primary Display".to_string(),
        width: 1920,
        height: 1080,
        is_primary: true,
    }]
}
