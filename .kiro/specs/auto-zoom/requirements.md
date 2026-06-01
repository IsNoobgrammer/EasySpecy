# Requirements Document

## Introduction

Auto-Zoom is an intelligent "virtual cameraman" system for EasySpecy that analyzes the full recording in post-processing and applies smooth, cinematic zoom effects based on user behavior patterns. The system infers user intent from cursor movements, clicks, gestures, keyboard activity, and window focus — then applies appropriate zoom levels with spring-physics camera animation. All zoom computation happens in post-processing where the full timeline is available for look-ahead, enabling smarter decisions than real-time systems.

## Glossary

- **Zoom_Engine**: The core Rust module that analyzes recording metadata and produces a timeline of zoom regions with camera positions
- **Camera_Animator**: The spring-physics subsystem that smoothly interpolates between zoom states using a damped harmonic oscillator
- **Event_Collector**: The subsystem that captures window bounds and keyboard timing during recording, synced to CAPTURE_ARMED
- **Frame_Renderer**: The Rust pipeline stage that crops and scales source frames to the computed zoom viewport
- **Zoom_Region**: A time span with an associated viewport rectangle (x, y, width, height) representing where the camera should focus
- **Zoom_Level**: The magnification factor (1.0 = full screen, 4.0 = maximum zoom)
- **Spring_State**: The current position, velocity, and target of the damped harmonic oscillator driving camera movement
- **Click_Cluster**: A group of 2 or more clicks occurring within a configurable time window and spatial proximity
- **Window_Dwell**: A period where the cursor remains within a single window's bounds with click activity
- **Gesture_Detector**: The subsystem that identifies circle/ellipse and text-selection patterns from cursor path data
- **Sensitivity**: A user-configurable parameter (0.0–1.0) that scales trigger thresholds for all zoom heuristics

## Requirements

### Requirement 1: Event Data Collection During Recording

**User Story:** As a user, I want the app to capture window bounds and keyboard timing during recording, so that the auto-zoom system has sufficient data to make intelligent zoom decisions in post-processing.

#### Acceptance Criteria

1. WHEN CAPTURE_ARMED fires, THE Event_Collector SHALL begin capturing the foreground window's bounding rectangle (x, y, width, height) and window title (truncated to 256 characters) on each click event
2. WHEN CAPTURE_ARMED fires, THE Event_Collector SHALL begin capturing keyboard event timestamps (key-down timing only, not key content)
3. WHEN a click event occurs and the foreground window is queryable, THE Event_Collector SHALL record the window's bounding rectangle and title at that instant with a timestamp relative to CAPTURE_ARMED origin (timestamp 0 = first video frame)
4. IF the foreground window cannot be queried at the time of a click event, THEN THE Event_Collector SHALL record the click timestamp and position with empty window bounds (0, 0, 0, 0) and an empty title
5. IF CAPTURE_ARMED has not fired, THEN THE Event_Collector SHALL discard all captured events to prevent timestamp misalignment
6. WHILE recording is paused, THE Event_Collector SHALL stop capturing window bounds and keyboard timestamps and SHALL NOT record events until recording resumes
7. WHEN recording stops, THE Event_Collector SHALL serialize all collected window bounds and keyboard timestamps into the existing RecordingMetadata structure as JSON via the same serde serialization used for cursor and click data

### Requirement 2: Click Cluster Zoom Trigger

**User Story:** As a user, I want the system to zoom into areas where I click repeatedly, so that viewers can see the details of what I'm interacting with.

#### Acceptance Criteria

1. WHEN 2 or more clicks occur within a 3-second sliding window and within 300 pixels of each other (Euclidean distance between any two clicks in the cluster), THE Zoom_Engine SHALL identify a Click_Cluster and generate a Zoom_Region targeting the axis-aligned bounding box enclosing all click positions in the cluster
2. WHEN a Click_Cluster is identified, THE Zoom_Engine SHALL calculate the zoom level by dividing the viewport dimension by the corresponding cluster bounding box dimension (plus 10% padding on all sides), using the axis that produces the smaller zoom factor
3. IF the calculated zoom level is below 1.2x, THEN THE Zoom_Engine SHALL discard the Click_Cluster and maintain the current zoom state without any visible change
4. IF the calculated zoom level exceeds 4.0x, THEN THE Zoom_Engine SHALL clamp the zoom to 4.0x
5. WHEN a Zoom_Region is activated, THE Zoom_Engine SHALL animate the zoom transition over a duration of 300 milliseconds
6. IF no new click is added to the active Click_Cluster within 4 seconds, THEN THE Zoom_Engine SHALL animate a zoom-out back to 1.0x over 500 milliseconds
7. WHEN the Sensitivity parameter is adjusted within its range of 1 (lowest) to 5 (highest), THE Zoom_Engine SHALL set the click count threshold to (6 minus Sensitivity value) and the time window to (3 seconds multiplied by (6 minus Sensitivity value) divided by 4), so that at Sensitivity 5 the threshold is 1 click in 0.75 seconds and at Sensitivity 1 the threshold is 5 clicks in 3.75 seconds

### Requirement 3: Window Dwell Zoom Trigger

**User Story:** As a user, I want the system to zoom into a window when I'm clearly focused on it, so that viewers see the relevant content at a readable size.

#### Acceptance Criteria

1. WHEN 2 or more clicks occur within a single window's bounds within a 5-second sliding window while the cursor remains inside that window, THE Zoom_Engine SHALL generate a Zoom_Region targeting that window's bounding rectangle
2. THE Zoom_Engine SHALL calculate the zoom level from the window's dimensions relative to the screen dimensions, filling the viewport with the window plus 5% padding on all sides (padding calculated as 5% of the window's width added to each horizontal edge and 5% of the window's height added to each vertical edge)
3. WHEN a Window_Dwell is detected and the full timeline confirms the cursor stays in the window, THE Zoom_Engine SHALL set the Zoom_Region start time to the moment the cursor first entered that window (look-ahead: zoom begins at window entry since the outcome is known)
4. IF the window occupies more than 85% of the screen area, THEN THE Zoom_Engine SHALL skip the zoom
5. IF the calculated zoom level is below 1.2x, THEN THE Zoom_Engine SHALL skip the zoom
6. IF the calculated zoom level exceeds 4.0x, THEN THE Zoom_Engine SHALL clamp the zoom to 4.0x
7. WHEN the cursor leaves the window bounds and no click occurs in that window for more than 3 seconds, THE Zoom_Engine SHALL end the Window_Dwell Zoom_Region at the timestamp of the last click in that window
8. WHEN the Sensitivity parameter is adjusted, THE Zoom_Engine SHALL scale the click count threshold proportionally (lower sensitivity = more clicks required to trigger window dwell zoom)

### Requirement 4: Circle/Ellipse Gesture Zoom Trigger

**User Story:** As a user, I want to circle something on screen and have the system zoom into that area, so that I can highlight specific content for viewers.

#### Acceptance Criteria

1. WHEN the cursor path forms a closed loop (start and end points within 50 pixels, path length > 200 pixels, completed within 3 seconds), THE Gesture_Detector SHALL identify a circle/ellipse gesture
2. WHEN a circle gesture is detected, THE Zoom_Engine SHALL generate a Zoom_Region targeting the bounding box of the gesture path plus 10% padding on all sides
3. THE Zoom_Engine SHALL calculate the zoom level to fill the viewport with the gesture's bounding area (using the axis that produces the smaller zoom factor)
4. IF the gesture's bounding box is smaller than 50x50 pixels, THEN THE Gesture_Detector SHALL ignore the gesture (too small to be intentional)
5. IF the calculated zoom level exceeds 4.0x, THEN THE Zoom_Engine SHALL clamp the zoom to 4.0x
6. WHEN a circle gesture zoom is activated, THE Camera_Animator SHALL animate the zoom-in over 300 milliseconds and hold the zoom for 2 seconds after the gesture completes, then animate zoom-out over 500 milliseconds

### Requirement 5: Text Selection / Underline Zoom Trigger

**User Story:** As a user, I want a subtle zoom when I select text or underline content, so that viewers can read what I'm highlighting without an aggressive zoom.

#### Acceptance Criteria

1. WHEN the cursor moves horizontally (horizontal displacement > 5x vertical displacement) over a distance of at least 100 pixels while any mouse button is held, THE Gesture_Detector SHALL identify a text selection gesture
2. WHEN a text selection gesture is detected, THE Zoom_Engine SHALL generate a Zoom_Region with a zoom level between 1.2x and 1.3x centered on the midpoint between the button-down position and the current cursor position, with the region width equal to the horizontal selection distance plus 20% padding on each side, animated over 300 to 500 milliseconds
3. WHILE the cursor moves vertically at less than 200 pixels per second (measured over a rolling 50-millisecond window) after a text selection zoom is active, THE Camera_Animator SHALL maintain the zoom level and pan the Zoom_Region center vertically to match the cursor vertical position
4. WHEN the cursor speed exceeds 800 pixels per second (measured over a rolling 50-millisecond window) after a text selection zoom is active, THE Camera_Animator SHALL animate the zoom level back to 1.0x over 300 to 500 milliseconds
5. IF the mouse button is released while a text selection zoom is active and cursor speed is below 800 pixels per second, THEN THE Camera_Animator SHALL hold the current zoom level and position for 1500 milliseconds, then animate back to 1.0x over 300 to 500 milliseconds

### Requirement 6: Typing Detection Zoom Trigger

**User Story:** As a user, I want a subtle zoom when I'm typing in an input field, so that viewers can see what I'm entering.

#### Acceptance Criteria

1. WHILE the cursor remains stationary (moves less than 20 pixels from its position at the time of the first keystroke), WHEN 3 or more keyboard events occur within a 1.5-second sliding window, THE Zoom_Engine SHALL identify a typing session
2. WHEN a typing session is detected, THE Zoom_Engine SHALL generate a Zoom_Region with a zoom level of 1.25x centered on the cursor position, transitioning to the zoomed state over a duration of 300 milliseconds
3. WHEN keyboard activity stops for more than 2 seconds during a typing session, THE Zoom_Engine SHALL end the typing session and transition back to the previous zoom state over a duration of 500 milliseconds
4. IF the cursor moves more than 100 pixels from the position recorded at typing session start, THEN THE Zoom_Engine SHALL immediately end the typing session and transition back to the previous zoom state over a duration of 300 milliseconds
5. IF a new typing session is detected while the Zoom_Engine is transitioning out of a previous typing session, THEN THE Zoom_Engine SHALL cancel the outgoing transition and transition to the new typing session zoom state from the current intermediate zoom level

### Requirement 7: Fast Cursor Movement Anti-Zoom

**User Story:** As a user, I want the system to zoom out when I move the cursor quickly across the screen, so that viewers maintain spatial context during navigation.

#### Acceptance Criteria

1. WHEN the cursor speed exceeds 1500 pixels per second for more than 200 milliseconds, THE Zoom_Engine SHALL generate a transition back to 1.0x (full screen) with a transition duration between 600 and 1000 milliseconds
2. THE Camera_Animator SHALL execute the zoom-out transition at a duration no less than 1.5 times the duration used for the most recent zoom-in transition
3. WHILE the cursor is idle (speed below 5 pixels per second) at full screen for more than 10 seconds, THE Zoom_Engine SHALL suppress all zoom triggers until cursor speed exceeds 50 pixels per second
4. IF the cursor speed drops below 1500 pixels per second during an in-progress zoom-out transition, THEN THE Zoom_Engine SHALL complete the transition to 1.0x rather than reversing it

### Requirement 8: Alt+Tab Window Switch Handling

**User Story:** As a user, I want window switches via Alt+Tab to appear as smooth transitions rather than showing the messy switching animation, so that the recording looks polished.

#### Acceptance Criteria

1. WHEN the active window changes 2 or more times within a 1-second sliding window (detected via window bounds data from Event_Collector), THE Zoom_Engine SHALL mark the period from the first window change to 500 milliseconds after the last window change as a window-switch transition, and record the final settled window as the transition target
2. WHEN a window-switch transition is detected, THE Frame_Renderer SHALL hold the last pre-switch frame for the duration of the intermediate switching period, then render a spring-physics camera transition from the old window viewport to the new window's bounds
3. THE Camera_Animator SHALL complete the window-switch camera transition within 400 milliseconds using the same damped harmonic oscillator as other zoom transitions (stiffness and damping from Requirement 9)
4. IF the final settled window occupies more than 85% of the screen area, THEN THE Camera_Animator SHALL transition to 1.0x full-screen view instead of zooming to the window bounds
5. IF the active window changes only once (single switch, not rapid), THEN THE Zoom_Engine SHALL NOT mark it as a window-switch transition and SHALL allow normal zoom triggers (click cluster, window dwell) to handle the new window

### Requirement 9: Spring-Physics Camera Animation

**User Story:** As a user, I want all camera movements to feel smooth and cinematic with natural spring physics, so that the recording looks professional.

#### Acceptance Criteria

1. THE Camera_Animator SHALL use a damped harmonic oscillator model with a natural frequency of 4.0 rad/s for all camera position and zoom level transitions
2. THE Camera_Animator SHALL complete zoom-in transitions faster than zoom-out transitions (zoom-in damping ratio: 0.7, zoom-out damping ratio: 0.85)
3. WHILE the camera is zoomed and the cursor moves, THE Camera_Animator SHALL pan the viewport to follow the cursor using spring-physics smoothing, maintaining a maximum trailing distance of 15% of the viewport width between the camera center and the cursor position
4. THE Camera_Animator SHALL enforce a minimum zoom duration of 1.5 seconds (no flash-zooms shorter than this)
5. WHILE a drag operation is in progress (mouse button held and cursor moving), THE Camera_Animator SHALL bypass spring smoothing and use raw cursor positions for panning (immediate response during drags)
6. IF a new zoom target is triggered while a zoom transition is still in progress, THEN THE Camera_Animator SHALL redirect the in-progress animation toward the new target by re-initializing the spring from the current interpolated position and velocity
7. WHEN a drag operation ends (mouse button released), THE Camera_Animator SHALL resume spring-physics smoothing from the current camera position with zero initial velocity

### Requirement 10: Zoom Viewport Rendering

**User Story:** As a user, I want the zoom to be rendered as a crop-and-scale operation in the Rust pipeline, so that the output quality is high and performance is good.

#### Acceptance Criteria

1. THE Frame_Renderer SHALL read the source frame, crop to the computed zoom viewport rectangle, and scale the cropped region to the configured output resolution (resolution_width × resolution_height)
2. THE Frame_Renderer SHALL use bilinear interpolation as the default scaling method, with bicubic interpolation applied when the zoom level exceeds 2.0x, to produce smooth upscaled output without visible block artifacts
3. IF the computed zoom viewport rectangle would extend beyond the source frame boundaries, THEN THE Frame_Renderer SHALL shift the viewport position inward until it fits entirely within the source frame dimensions without resizing the viewport
4. WHEN the zoom viewport is computed, THE Frame_Renderer SHALL transform cursor trail coordinates from screen space to viewport space by subtracting the viewport origin (top-left corner) and multiplying by the ratio of output resolution to viewport size
5. THE Frame_Renderer SHALL render cursor trail and click effects on top of the zoomed and scaled frame, with all effect coordinates expressed in viewport space
6. THE Frame_Renderer SHALL process frames in parallel batches of 2x available CPU cores using rayon, writing completed batches sequentially to the output pipe in frame order
7. IF the zoom viewport dimensions are smaller than 16x16 pixels, THEN THE Frame_Renderer SHALL clamp the viewport to a minimum size of 16x16 pixels to prevent degenerate scaling

### Requirement 11: Post-Processing Pipeline Integration

**User Story:** As a user, I want auto-zoom to integrate seamlessly with the existing effects pipeline, so that all effects (trail, clicks, zoom) work together correctly.

#### Acceptance Criteria

1. THE Frame_Renderer SHALL execute the pipeline in this order for each frame: read source frame → compute zoom viewport from pre-computed timeline → crop source frame to viewport and scale to output resolution → transform cursor trail coordinates from source-space to cropped-viewport-space → render cursor trail onto the scaled frame → render click effects using the same transformed coordinates → pipe the composited frame to FFmpeg encoder
2. IF auto-zoom is disabled in settings, THEN THE Frame_Renderer SHALL skip zoom viewport computation, apply no coordinate transformation to trail or click effect positions, and render all effects at 1.0x scale onto the full source frame
3. THE Zoom_Engine SHALL pre-compute the entire zoom timeline (one viewport rectangle per video frame) before the Frame_Renderer begins rendering, completing within 2 seconds per 1000 frames of recorded video
4. THE Zoom_Engine SHALL derive all zoom-event timestamps relative to the CAPTURE_ARMED instant (t=0), the same origin used by the cursor trail subsystem and the click-event subsystem
5. WHEN the zoom viewport is active for a frame, THE Frame_Renderer SHALL translate each trail point and click-effect position by subtracting the viewport origin and multiplying by the scale factor (output_width / viewport_width), discarding any points that fall outside the viewport bounds

### Requirement 12: User Configuration

**User Story:** As a user, I want to control auto-zoom behavior through settings, so that I can tune it to my preferences or disable it entirely.

#### Acceptance Criteria

1. THE Settings_Panel SHALL provide a toggle to enable or disable auto-zoom, defaulting to disabled on fresh install
2. THE Settings_Panel SHALL provide a sensitivity slider with a range of 0.0 to 1.0 in increments of 0.05, defaulting to 0.5, where the value acts as a multiplier applied to all zoom trigger thresholds (e.g., a sensitivity of 0.5 doubles the threshold required to trigger a zoom)
3. THE Settings_Panel SHALL provide a zoom speed slider with a range of 0.1 to 2.0 in increments of 0.1, defaulting to 1.0, that scales the spring stiffness parameter of the Camera_Animator (higher values produce faster zoom transitions)
4. WHILE auto-zoom is disabled, THE Zoom_Engine SHALL produce no Zoom_Regions and the Frame_Renderer SHALL skip all zoom processing
5. WHEN the user saves settings, THE Configuration SHALL persist the auto-zoom enabled state, sensitivity value, and zoom speed value to the TOML config file at the standard config path
6. IF the Configuration loads a sensitivity or zoom speed value outside its valid range, THEN THE Configuration SHALL clamp the value to the nearest bound and apply the clamped value

### Requirement 13: Zoom Priority and Conflict Resolution

**User Story:** As a user, I want the system to handle overlapping zoom triggers gracefully, so that the camera doesn't jump erratically between targets.

#### Acceptance Criteria

1. WHEN multiple zoom triggers overlap in time, THE Zoom_Engine SHALL select the trigger with the highest priority according to the priority order: circle gesture (highest) > click cluster > window dwell > typing > text selection (lowest)
2. THE Zoom_Engine SHALL enforce a minimum 1.5-second cooldown between consecutive zoom transitions; IF a new trigger arrives during the cooldown period, THEN THE Zoom_Engine SHALL discard the new trigger unless it has a higher priority than the active zoom, in which case it SHALL be queued and executed when the cooldown expires
3. WHEN a higher-priority zoom trigger activates during an existing zoom transition, THE Camera_Animator SHALL interrupt the current transition and begin a new transition to the new target using spring physics, completing within 400 milliseconds
4. THE Zoom_Engine SHALL assign priority order: circle gesture (highest) > click cluster > window dwell > typing > text selection (lowest)
5. IF two overlapping zoom triggers have equal priority, THEN THE Zoom_Engine SHALL select the trigger that was activated most recently (last-in wins)

### Requirement 14: Zoom Timeline Serialization

**User Story:** As a user, I want the computed zoom timeline to be saved alongside the recording metadata, so that zoom decisions can be inspected, debugged, or re-applied.

#### Acceptance Criteria

1. WHEN the Zoom_Engine completes timeline computation, THE Zoom_Engine SHALL serialize the zoom timeline to a valid JSON file containing the list of Zoom_Regions
2. THE Zoom_Engine SHALL save the zoom timeline file alongside the existing .meta.json file using the pattern `{recording_name}.zoom.json`
3. THE Zoom_Engine SHALL include in each serialized Zoom_Region: trigger type (click_cluster, window_dwell, circle_gesture, text_selection, or typing), start timestamp in milliseconds from CAPTURE_ARMED, end timestamp in milliseconds from CAPTURE_ARMED, viewport rectangle (x, y, width, height in pixels relative to the source recording resolution), and zoom level (1.0-4.0)
4. THE Zoom_Engine SHALL include in the serialized file the source recording resolution (width and height in pixels) used during zoom computation
5. IF the zoom timeline file cannot be written to disk, THEN THE Zoom_Engine SHALL log the failure and continue post-processing without interrupting the pipeline
6. WHEN the Zoom_Engine produces zero Zoom_Regions, THE Zoom_Engine SHALL still write the zoom timeline file containing an empty regions list and the source recording resolution
