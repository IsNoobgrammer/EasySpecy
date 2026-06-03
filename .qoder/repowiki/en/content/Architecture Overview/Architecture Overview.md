# Architecture Overview

<cite>
**Referenced Files in This Document**
- [Cargo.toml](file://src-tauri/Cargo.toml)
- [tauri.conf.json](file://src-tauri/tauri.conf.json)
- [lib.rs](file://src-tauri/src/lib.rs)
- [main.rs](file://src-tauri/src/main.rs)
- [mod.rs](file://src-tauri/src/commands/mod.rs)
- [mod.rs](file://src-tauri/src/capture/mod.rs)
- [mod.rs](file://src-tauri/src/audio/mod.rs)
- [mod.rs](file://src-tauri/src/config/mod.rs)
- [mod.rs](file://src-tauri/src/postprocess/mod.rs)
- [mod.rs](file://src-tauri/src/webcam/mod.rs)
- [mod.rs](file://src-tauri/src/autozoom/mod.rs)
- [App.tsx](file://src/App.tsx)
- [main.tsx](file://src/main.tsx)
- [Dashboard.tsx](file://src/components/Dashboard.tsx)
- [recording.ts](file://src/stores/recording.ts)
- [README.md](file://README.md)
</cite>

## Table of Contents
1. [Introduction](#introduction)
2. [System Architecture](#system-architecture)
3. [Technology Stack](#technology-stack)
4. [IPC Communication Pattern](#ipc-communication-pattern)
5. [Recording Pipeline](#recording-pipeline)
6. [Cross-Platform Capture Implementation](#cross-platform-capture-implementation)
7. [Audio Processing Architecture](#audio-processing-architecture)
8. [Visual Effects Rendering](#visual-effects-rendering)
9. [Command System Design](#command-system-design)
10. [Performance and Synchronization](#performance-and-synchronization)
11. [System Context Diagrams](#system-context-diagrams)
12. [Conclusion](#conclusion)

## Introduction

EasySpecy is a cross-platform screen recording application built with Tauri 2 that seamlessly integrates a React frontend with a Rust backend. The application provides professional-grade screen recording capabilities with advanced features including auto-zoom, cursor effects, webcam overlays, and cinematic visual enhancements. The architecture follows a hybrid desktop design pattern where the frontend handles user interface and user interactions, while the Rust backend manages system-level operations, native OS integrations, and performance-critical processing.

The application targets Windows, macOS, and Linux platforms, leveraging native APIs for optimal performance while maintaining cross-platform compatibility through Tauri's framework. The system is designed around strict synchronization principles to ensure perfect video-audio synchronization and minimal latency throughout the recording pipeline.

## System Architecture

EasySpecy employs a clean separation between frontend and backend components, utilizing Tauri's IPC (Inter-Process Communication) mechanism for seamless interaction. The architecture follows a layered design pattern with clear boundaries between presentation, business logic, and system integration layers.

```mermaid
graph TB
subgraph "Frontend Layer"
UI[React UI Components]
Store[Zustand State Management]
IPC[Tauri IPC Bridge]
end
subgraph "Backend Layer"
Commands[Tauri Commands]
Capture[Screen Capture Module]
Audio[Audio Processing Module]
PostProcess[Post-Processing Module]
Webcam[Webcam Capture Module]
Config[Configuration Manager]
end
subgraph "System Integration"
OS_API[Native OS APIs]
FFmpeg[FFmpeg Encoder]
GPU[GPU Acceleration]
end
UI --> Store
Store --> IPC
IPC --> Commands
Commands --> Capture
Commands --> Audio
Commands --> PostProcess
Commands --> Webcam
Commands --> Config
Capture --> OS_API
Audio --> OS_API
PostProcess --> FFmpeg
Webcam --> FFmpeg
FFmpeg --> GPU
UI -.-> Events[Event Listeners]
Events --> Store
```

**Diagram sources**
- [lib.rs:75-125](file://src-tauri/src/lib.rs#L75-L125)
- [mod.rs:11-117](file://src-tauri/src/commands/mod.rs#L11-L117)

The architecture ensures loose coupling between components while maintaining high cohesion within functional modules. Each module has well-defined responsibilities and communicates through explicit interfaces, enabling maintainability and extensibility.

**Section sources**
- [lib.rs:37-125](file://src-tauri/src/lib.rs#L37-L125)
- [tauri.conf.json:1-48](file://src-tauri/tauri.conf.json#L1-L48)

## Technology Stack

The technology stack is carefully selected to balance performance, cross-platform compatibility, and development efficiency:

### Core Framework
- **Tauri 2**: Provides native application framework with minimal overhead (~3MB bundle size)
- **React 19**: Modern frontend framework with concurrent features and improved performance
- **TypeScript**: Type safety and enhanced developer experience
- **Tailwind CSS**: Utility-first styling framework for rapid UI development

### Backend Technologies
- **Rust**: Memory-safe systems programming language for performance-critical operations
- **CPAL**: Cross-platform audio I/O library for audio capture and playback
- **FFmpeg**: Industry-standard multimedia framework for encoding and processing
- **Rayon**: Parallel processing library for CPU-intensive tasks

### Platform-Specific Integrations
- **Windows**: Windows Graphics Capture API for efficient screen capture
- **macOS**: ScreenCaptureKit for modern screen recording capabilities
- **Linux**: PipeWire for standardized multimedia capture
- **Nokhwa**: Cross-platform webcam capture library

### Additional Libraries
- **Serde**: Serialization framework for configuration and data exchange
- **Tracing**: Structured logging and observability
- **Dirs**: Cross-platform configuration and data directory management

**Section sources**
- [Cargo.toml:26-61](file://src-tauri/Cargo.toml#L26-L61)
- [README.md:17-25](file://README.md#L17-L25)

## IPC Communication Pattern

The IPC communication follows Tauri's command-based architecture, providing type-safe bidirectional communication between frontend and backend. The system uses a centralized command handler that routes requests to appropriate backend modules.

```mermaid
sequenceDiagram
participant Frontend as "React Frontend"
participant IPC as "Tauri IPC"
participant Command as "Command Handler"
participant Module as "Backend Module"
participant System as "System API"
Frontend->>IPC : invoke("start_recording")
IPC->>Command : generate_handler![start_recording]
Command->>Module : capture : : start_recording(config)
Module->>System : Initialize capture pipeline
System-->>Module : Capture initialized
Module-->>Command : Success/Failure
Command-->>IPC : Result
IPC-->>Frontend : Recording started
Note over Frontend,System : Real-time communication
Frontend->>IPC : invoke("get_encoding_progress")
IPC->>Command : generate_handler![get_encoding_progress]
Command->>Module : capture : : get_encoding_progress()
Module-->>Command : (progress, stage)
Command-->>IPC : Result
IPC-->>Frontend : Progress update
```

**Diagram sources**
- [mod.rs:216-311](file://src-tauri/src/commands/mod.rs#L216-L311)
- [mod.rs:502-714](file://src-tauri/src/capture/mod.rs#L502-L714)

The IPC pattern supports both synchronous and asynchronous operations, with proper error handling and timeout mechanisms. Commands are strongly typed using Serde serialization, ensuring compile-time safety and runtime reliability.

**Section sources**
- [mod.rs:11-117](file://src-tauri/src/commands/mod.rs#L11-L117)
- [lib.rs:75-125](file://src-tauri/src/lib.rs#L75-L125)

## Recording Pipeline

The recording pipeline implements a sophisticated multi-stage process that ensures perfect synchronization between video, audio, and metadata capture. The pipeline follows a producer-consumer pattern with strict timing guarantees.

```mermaid
flowchart TD
Start([Start Recording]) --> InitConfig["Initialize Configuration"]
InitConfig --> SetupCapture["Setup Capture Pipeline"]
SetupCapture --> StartVideo["Start Video Capture Thread"]
SetupCapture --> StartAudio["Start Audio Capture Threads"]
SetupCapture --> StartWebcam["Start Webcam Capture Thread"]
StartVideo --> WaitArmed["Wait for Capture Armed"]
StartAudio --> WaitArmed
StartWebcam --> WaitArmed
WaitArmed --> FirstFrame["First Video Frame Arrives"]
FirstFrame --> ArmAudio["Arm Audio Capture"]
FirstFrame --> ArmWebcam["Arm Webcam Capture"]
FirstFrame --> ResetTimestamps["Reset Session Timestamps"]
ArmAudio --> StartRecording["Begin Recording"]
ArmWebcam --> StartRecording
ResetTimestamps --> StartRecording
StartRecording --> CaptureLoop["Main Capture Loop"]
CaptureLoop --> CollectMetadata["Collect Cursor/Metadata"]
CaptureLoop --> ProcessFrames["Process Video Frames"]
CaptureLoop --> MonitorAudio["Monitor Audio Levels"]
ProcessFrames --> EncodePipeline["FFmpeg Encoding Pipeline"]
CollectMetadata --> MetadataPipeline["Metadata Processing"]
MonitorAudio --> AudioPipeline["Audio Processing"]
EncodePipeline --> MergeAudio["Merge Audio Tracks"]
MetadataPipeline --> EffectPipeline["Apply Visual Effects"]
AudioPipeline --> FinalizeRecording["Finalize Recording"]
MergeAudio --> VerifySync["Verify Sync Accuracy"]
EffectPipeline --> VerifySync
VerifySync --> SaveOutput["Save Final Output"]
SaveOutput --> End([Recording Complete])
```

**Diagram sources**
- [mod.rs:160-404](file://src-tauri/src/capture/mod.rs#L160-L404)
- [mod.rs:225-553](file://src-tauri/src/audio/mod.rs#L225-L553)
- [mod.rs:76-181](file://src-tauri/src/postprocess/mod.rs#L76-L181)

The pipeline ensures that all components start simultaneously within 1ms of each other, using atomic flags and synchronization primitives to coordinate the capture threads. This design eliminates audio-video drift and provides consistent timing across all recording sessions.

**Section sources**
- [mod.rs:1-12](file://src-tauri/src/capture/mod.rs#L1-L12)
- [mod.rs:160-404](file://src-tauri/src/capture/mod.rs#L160-L404)

## Cross-Platform Capture Implementation

The capture implementation leverages platform-specific APIs optimized for each operating system while maintaining a unified interface through Rust traits and conditional compilation.

```mermaid
graph TB
subgraph "Windows"
WinGC[Windows Graphics Capture]
WinAPI[Win32 APIs]
WinGPU[NVIDIA NVENC]
end
subgraph "macOS"
MacSC[ScreenCaptureKit]
MacGPU[Apple Silicon Video Toolbox]
end
subgraph "Linux"
LinPW[PipeWire]
LinGPU[VA-API/VAAPI]
end
subgraph "Shared Layer"
SharedAPI[Unified Capture API]
Sync[Timing Synchronization]
Buffer[Frame Buffering]
end
WinGC --> SharedAPI
MacSC --> SharedAPI
LinPW --> SharedAPI
SharedAPI --> Sync
SharedAPI --> Buffer
WinAPI --> WinGPU
MacGPU --> Sync
LinGPU --> Sync
```

**Diagram sources**
- [Cargo.toml:52-61](file://src-tauri/Cargo.toml#L52-L61)
- [mod.rs:19-30](file://src-tauri/src/capture/mod.rs#L19-L30)

Each platform implementation provides optimized capture performance while maintaining identical behavior for the higher-level application logic. The Windows implementation uses the modern Graphics Capture API for efficient screen capture, macOS leverages ScreenCaptureKit for system-integrated recording, and Linux utilizes PipeWire for standardized multimedia capture.

**Section sources**
- [Cargo.toml:52-61](file://src-tauri/Cargo.toml#L52-L61)
- [mod.rs:1-12](file://src-tauri/src/capture/mod.rs#L1-L12)

## Audio Processing Architecture

The audio processing system implements a sophisticated multi-threaded architecture that captures, processes, and synchronizes audio streams with video capture. The system supports both microphone and system audio capture with advanced noise reduction capabilities.

```mermaid
sequenceDiagram
participant Frontend as "Frontend"
participant AudioCmd as "Audio Commands"
participant AudioMgr as "Audio Manager"
participant MicStream as "Microphone Stream"
participant SysStream as "System Audio Stream"
participant Processor as "Audio Processor"
participant WAVWriter as "WAV Writer"
Frontend->>AudioCmd : start_audio_monitor_cmd()
AudioCmd->>AudioMgr : start_audio_monitor(source)
AudioMgr->>MicStream : Create microphone stream
AudioMgr->>SysStream : Create system audio stream
MicStream->>Processor : Process microphone samples
SysStream->>Processor : Process system audio samples
Note over Processor : Real-time audio processing
Processor->>Processor : Noise reduction (RNN/Spectral)
Processor->>Processor : Level adjustment
Processor->>Processor : Channel conversion
Frontend->>AudioCmd : stop_audio_monitor_cmd()
AudioCmd->>AudioMgr : stop_audio_monitor()
AudioMgr->>MicStream : Drop microphone stream
AudioMgr->>SysStream : Drop system audio stream
Frontend->>AudioCmd : get_audio_levels()
AudioCmd->>AudioMgr : get_audio_levels()
AudioMgr-->>Frontend : Current audio levels
Note over Frontend,AudioMgr : Recording phase
Frontend->>AudioCmd : start_recording()
AudioCmd->>AudioMgr : start_recording()
AudioMgr->>MicStream : Start capturing
AudioMgr->>SysStream : Start capturing
AudioMgr->>Processor : Set armed=true
Processor->>Processor : Begin synchronized recording
Frontend->>AudioCmd : stop_recording()
AudioCmd->>AudioMgr : stop_recording()
AudioMgr->>Processor : Stop and process
Processor->>WAVWriter : Write processed audio
WAVWriter-->>Frontend : Audio file path
```

**Diagram sources**
- [mod.rs:82-187](file://src-tauri/src/audio/mod.rs#L82-L187)
- [mod.rs:225-553](file://src-tauri/src/audio/mod.rs#L225-L553)

The audio system implements advanced noise reduction algorithms including RNN-based denoising, spectral subtraction, and adaptive noise gating. The processing maintains real-time performance while providing high-quality audio output suitable for professional recording scenarios.

**Section sources**
- [mod.rs:1-14](file://src-tauri/src/audio/mod.rs#L1-L14)
- [mod.rs:82-187](file://src-tauri/src/audio/mod.rs#L82-L187)

## Visual Effects Rendering

The visual effects system provides real-time cursor trails, click animations, and webcam overlays through a combination of hardware-accelerated rendering and FFmpeg-based post-processing. The system maintains smooth 60fps performance while delivering cinematic visual enhancements.

```mermaid
flowchart LR
subgraph "Real-time Effects"
Cursor[Cursor Tracking]
Trail[Trail Renderer]
Click[Click Effects]
Webcam[Webcam Overlay]
end
subgraph "Post-processing"
Meta[Metadata Collection]
FFmpeg[FFmpeg Pipeline]
Compose[Video Composition]
end
subgraph "Hardware Acceleration"
GPU[GPU Rendering]
CUDA[NVIDIA CUDA]
Metal[Apple Metal]
Vulkan[Vulkan]
end
Cursor --> Trail
Trail --> Click
Click --> Webcam
Webcam --> Meta
Meta --> FFmpeg
Trail --> FFmpeg
Click --> FFmpeg
Webcam --> FFmpeg
FFmpeg --> GPU
GPU --> CUDA
GPU --> Metal
GPU --> Vulkan
Compose --> Output[Final Video]
```

**Diagram sources**
- [mod.rs:76-181](file://src-tauri/src/postprocess/mod.rs#L76-L181)
- [mod.rs:38-101](file://src-tauri/src/webcam/mod.rs#L38-L101)

The effects system uses a hybrid approach where real-time cursor tracking and basic effects are handled by the Rust backend, while complex post-processing and final composition utilize FFmpeg with GPU acceleration. This design ensures responsive user feedback during recording while maintaining high-quality output.

**Section sources**
- [mod.rs:1-8](file://src-tauri/src/postprocess/mod.rs#L1-L8)
- [mod.rs:1-14](file://src-tauri/src/webcam/mod.rs#L1-L14)

## Command System Design

The command system serves as the central interface between the frontend and backend, implementing a comprehensive set of operations for recording control, configuration management, and system integration. The system uses Tauri's derive macros for automatic serialization and deserialization.

```mermaid
classDiagram
class CommandHandler {
+get_config() AppConfig
+save_config(config) Result
+update_config_field(key, value) Result
+get_audio_devices() Result~Vec~String~~
+get_screens() Result~Vec~DisplayInfo~~
+start_recording(output_path) Result
+stop_recording() Result~RecordingResult~
+pause_recording_cmd() void
+resume_recording_cmd() void
+get_recording_status() (bool, bool, u32)
+get_encoding_progress() (u32, String)
+create_effects_overlay(app) Result
+destroy_effects_overlay(app) Result
+create_webcam_overlay(app) Result
+destroy_webcam_overlay(app) Result
+start_audio_monitor_cmd() Result
+stop_audio_monitor_cmd() void
+get_audio_levels() AudioLevels
+get_keyboard_events() Vec~KeyEvent~
}
class RecordingConfig {
+output_path String
+enable_audio bool
+audio_source String
+audio_sample_rate u32
+fps u32
+webcam_enabled bool
+webcam_device String
+webcam_size u32
+webcam_x i32
+webcam_y i32
+webcam_shape String
+webcam_border_color String
+webcam_border_width u32
+webcam_opacity f32
}
class AudioLevels {
+mic_rms f32
+mic_peak f32
+mic_db f32
+sys_rms f32
+sys_peak f32
+sys_db f32
}
CommandHandler --> RecordingConfig : "uses"
CommandHandler --> AudioLevels : "returns"
```

**Diagram sources**
- [mod.rs:11-117](file://src-tauri/src/commands/mod.rs#L11-L117)
- [mod.rs:40-57](file://src-tauri/src/commands/mod.rs#L40-L57)

The command system provides comprehensive coverage of all application functionality, from basic configuration management to complex recording orchestration. Each command is designed with proper error handling and follows consistent naming conventions for maintainability.

**Section sources**
- [mod.rs:1-690](file://src-tauri/src/commands/mod.rs#L1-L690)

## Performance and Synchronization

The system implements several advanced synchronization mechanisms to ensure perfect timing between all recording components. The architecture prioritizes real-time performance while maintaining high-quality output through careful resource management and optimization strategies.

### Synchronization Mechanisms

The core synchronization relies on atomic flags and coordinated startup sequences:

1. **Capture Armed Gate**: All capture components wait for the first video frame before starting
2. **Audio Arming**: Audio capture begins simultaneously with video at frame 0
3. **Webcam Synchronization**: Webcam capture aligns with video and audio timing
4. **Metadata Collection**: Cursor and click events are timestamped relative to capture start

### Performance Optimizations

- **Producer-Consumer Patterns**: Asynchronous processing separates capture from I/O operations
- **Parallel Processing**: Rayon enables multi-core utilization for CPU-intensive tasks
- **Memory Management**: Efficient buffer reuse and minimal allocations
- **GPU Acceleration**: Hardware-accelerated encoding reduces CPU load
- **Lazy Initialization**: Components initialize only when needed

### Timing Guarantees

The system maintains sub-millisecond synchronization accuracy between video, audio, and metadata streams. This precision is achieved through:

- Atomic flag coordination for component startup
- Precise timestamping at capture initiation
- Consistent frame rate handling across all components
- Real-time progress reporting during encoding

**Section sources**
- [mod.rs:105-158](file://src-tauri/src/capture/mod.rs#L105-L158)
- [mod.rs:373-381](file://src-tauri/src/audio/mod.rs#L373-L381)

## System Context Diagrams

The following diagrams illustrate the complete system context showing relationships between all major components and external dependencies.

### Complete System Architecture

```mermaid
graph TB
subgraph "User Interface Layer"
Dashboard[Dashboard Component]
Settings[Settings Panel]
Controls[Recording Controls]
Preview[Preview Windows]
end
subgraph "Application Core"
Store[State Management]
Commands[Command Handler]
Config[Configuration Manager]
History[Recording History]
end
subgraph "Capture Subsystem"
ScreenCapture[Screen Capture]
AudioCapture[Audio Capture]
WebcamCapture[Webcam Capture]
CursorTracker[Cursor Tracker]
end
subgraph "Processing Pipeline"
FFmpeg[FFmpeg Encoder]
PostProcess[Post-Processing]
Effects[Visual Effects]
SyncVerifier[Sync Verification]
end
subgraph "System Integration"
WindowsAPI[Windows Graphics Capture]
MacAPI[ScreenCaptureKit]
LinuxAPI[PipeWire]
CPAL[CPAL Audio]
Nokhwa[Nokhwa Webcam]
end
Dashboard --> Store
Settings --> Store
Controls --> Commands
Preview --> Store
Store --> Commands
Commands --> ScreenCapture
Commands --> AudioCapture
Commands --> WebcamCapture
Commands --> CursorTracker
ScreenCapture --> WindowsAPI
ScreenCapture --> MacAPI
ScreenCapture --> LinuxAPI
AudioCapture --> CPAL
WebcamCapture --> Nokhwa
ScreenCapture --> FFmpeg
AudioCapture --> FFmpeg
CursorTracker --> PostProcess
PostProcess --> Effects
Effects --> SyncVerifier
FFmpeg --> History
SyncVerifier --> History
```

**Diagram sources**
- [Dashboard.tsx:218-573](file://src/components/Dashboard.tsx#L218-L573)
- [recording.ts:176-403](file://src/stores/recording.ts#L176-L403)
- [mod.rs:11-117](file://src-tauri/src/commands/mod.rs#L11-L117)

### Recording Workflow Context

```mermaid
sequenceDiagram
participant User as "User"
participant UI as "Dashboard UI"
participant Store as "Zustand Store"
participant IPC as "Tauri IPC"
participant Backend as "Rust Backend"
participant Capture as "Capture System"
participant Encode as "Encoding Pipeline"
User->>UI : Click Record Button
UI->>Store : startRecording()
Store->>IPC : invoke("start_recording")
IPC->>Backend : commands : : start_recording()
Backend->>Capture : capture : : start_recording()
Capture->>Capture : Initialize capture pipeline
Capture->>Capture : Wait for first frame
Note over Capture : Perfect synchronization point
Capture->>Backend : Capture armed (frame 0)
Backend->>Store : Update recording state
User->>UI : View recording timer
UI->>Store : Poll recording status
Store->>IPC : invoke("get_recording_status")
IPC->>Backend : commands : : get_recording_status()
Backend->>Store : (true, false, frame_count)
User->>UI : Stop recording
UI->>Store : stopRecording()
Store->>IPC : invoke("stop_recording")
IPC->>Backend : commands : : stop_recording()
Backend->>Encode : capture : : stop_recording()
Encode->>Encode : FFmpeg encoding pipeline
Encode->>Store : Final recording result
Store->>UI : Show recording result
UI->>Store : Save to history
```

**Diagram sources**
- [Dashboard.tsx:283-336](file://src/components/Dashboard.tsx#L283-L336)
- [recording.ts:283-336](file://src/stores/recording.ts#L283-L336)
- [mod.rs:326-370](file://src-tauri/src/commands/mod.rs#L326-L370)

These diagrams demonstrate the complete flow from user interaction through the entire recording and encoding process, highlighting the coordination between frontend state management and backend processing capabilities.

**Section sources**
- [Dashboard.tsx:218-573](file://src/components/Dashboard.tsx#L218-L573)
- [recording.ts:176-403](file://src/stores/recording.ts#L176-L403)

## Conclusion

EasySpecy represents a sophisticated hybrid desktop application that successfully balances performance, features, and cross-platform compatibility. The architecture demonstrates excellent separation of concerns with clear boundaries between frontend presentation and backend processing, while leveraging native OS capabilities for optimal performance.

Key architectural strengths include:

- **Clean Separation**: Well-defined boundaries between frontend React components and Rust backend modules
- **Synchronization Excellence**: Advanced timing mechanisms ensure perfect video-audio synchronization
- **Cross-Platform Design**: Platform-specific optimizations while maintaining unified interfaces
- **Performance Focus**: Multi-threaded architecture with GPU acceleration and efficient resource management
- **Extensibility**: Modular design allows for easy addition of new features and platforms

The system achieves native performance characteristics typical of desktop applications while maintaining the development efficiency and distribution advantages of cross-platform solutions. The comprehensive IPC system, robust error handling, and thoughtful synchronization mechanisms position EasySpecy as a professional-grade screen recording solution suitable for demanding use cases.

Future enhancements could focus on expanding platform support, adding advanced editing capabilities, and further optimizing the encoding pipeline for specific use cases and hardware configurations.