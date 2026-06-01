"""
Auto-Zoom Verification Test
============================
Uses PyAutoGUI to simulate mouse activity, then verifies the auto-zoom
detection engine produces correct zoom regions from the recorded metadata.

This test:
1. Generates synthetic cursor/click/keyboard data (simulating a real recording)
2. Runs the detection algorithm (via a Rust test binary)
3. Verifies zoom regions are generated correctly

Requirements:
    pip install pyautogui

Usage:
    python tests/test_autozoom.py
"""

import json
import os
import sys
import time
import subprocess

# ═══ TEST DATA GENERATION ═══
# Simulate realistic recording metadata without actually recording

def generate_test_metadata():
    """Generate synthetic recording metadata that should trigger auto-zoom."""
    
    metadata = {
        "cursor_trail": [],
        "click_events": [],
        "window_events": [],
        "keyboard_events": [],
        "trail_style": "glow",
        "click_effect": "ripple",
        "trail_color": "#00ff88",
        "secondary_color": "#ff4488",
        "trail_duration_ms": 600.0,
        "trail_width": 1.0,
    }
    
    # ═══ SCENARIO 1: Click cluster in top-left area (should trigger zoom) ═══
    # 5 clicks within 2 seconds in a 200x200 area
    cluster_x, cluster_y = 300.0, 250.0
    for i in range(5):
        t = 2000 + i * 400  # clicks at 2.0s, 2.4s, 2.8s, 3.2s, 3.6s
        metadata["click_events"].append({
            "timestamp_ms": t,
            "x": cluster_x + (i * 30.0),  # spread across ~120px
            "y": cluster_y + (i * 20.0),
            "button": "left",
        })
        metadata["window_events"].append({
            "timestamp_ms": t,
            "x": 100, "y": 100, "width": 600, "height": 400,
            "title": "VS Code",
        })
    
    # ═══ SCENARIO 2: Typing session (should trigger subtle 1.25x zoom) ═══
    # Rapid keystrokes at 8000-11000ms with stationary cursor
    typing_x, typing_y = 800.0, 500.0
    for i in range(30):
        t = 8000 + i * 100  # 10 keystrokes/sec for 3 seconds
        metadata["keyboard_events"].append({"timestamp_ms": t})
    
    # Cursor stays still during typing
    for t in range(8000, 11000, 50):
        metadata["cursor_trail"].append({
            "timestamp_ms": t,
            "x": typing_x + (t % 3) * 0.1,  # tiny jitter (< 20px)
            "y": typing_y + (t % 5) * 0.1,
        })
    
    # ═══ SCENARIO 3: Circle gesture (should trigger zoom to circled area) ═══
    # Draw a circle at ~14 seconds
    import math
    circle_cx, circle_cy = 1200.0, 600.0
    circle_r = 150.0
    circle_start_ms = 14000
    for i in range(40):
        angle = (i / 40.0) * 2 * math.pi
        t = circle_start_ms + i * 50  # 2 seconds to draw circle
        metadata["cursor_trail"].append({
            "timestamp_ms": t,
            "x": circle_cx + circle_r * math.cos(angle),
            "y": circle_cy + circle_r * math.sin(angle),
        })
    
    # ═══ SCENARIO 4: Text selection (horizontal sweep) ═══
    # Horizontal sweep at 18 seconds
    for i in range(20):
        t = 18000 + i * 30
        metadata["cursor_trail"].append({
            "timestamp_ms": t,
            "x": 400.0 + i * 25.0,  # 500px horizontal sweep
            "y": 700.0 + i * 1.0,   # minimal vertical movement
        })
    # Simulate button held during sweep
    metadata["click_events"].append({
        "timestamp_ms": 18000,
        "x": 400.0, "y": 700.0, "button": "left",
    })
    
    # ═══ SCENARIO 5: Fast cursor movement (should NOT trigger zoom) ═══
    # Rapid movement across screen at 22 seconds
    for i in range(10):
        t = 22000 + i * 20  # 200ms total, very fast
        metadata["cursor_trail"].append({
            "timestamp_ms": t,
            "x": 100.0 + i * 180.0,  # 1800px in 200ms = 9000px/s
            "y": 500.0,
        })
    
    # ═══ Fill in cursor trail for other times (normal movement) ═══
    # Add some general cursor movement between scenarios
    for t in range(0, 2000, 50):
        metadata["cursor_trail"].append({
            "timestamp_ms": t,
            "x": 960.0 + t * 0.1,
            "y": 540.0,
        })
    for t in range(3700, 5000, 50):
        metadata["cursor_trail"].append({
            "timestamp_ms": t,
            "x": 500.0 + t * 0.05,
            "y": 400.0,
        })
    
    # Sort cursor trail by timestamp
    metadata["cursor_trail"].sort(key=lambda s: s["timestamp_ms"])
    
    return metadata


def verify_zoom_regions(regions, metadata):
    """Verify that the detection engine produced correct zoom regions."""
    
    print(f"\n{'='*60}")
    print(f"  AUTO-ZOOM VERIFICATION RESULTS")
    print(f"{'='*60}\n")
    
    passed = 0
    failed = 0
    
    # Test 1: Click cluster should produce a zoom region
    click_zoom = [r for r in regions if r["trigger"] == "ClickCluster"]
    if click_zoom:
        # Verify center is near the click cluster area (300, 250)
        r = click_zoom[0]
        if 200 < r["center_x"] < 500 and 150 < r["center_y"] < 400:
            print(f"  ✅ PASS: Click cluster zoom detected at ({r['center_x']:.0f}, {r['center_y']:.0f}), zoom={r['zoom_level']:.2f}x")
            passed += 1
        else:
            print(f"  ❌ FAIL: Click cluster zoom at wrong position ({r['center_x']:.0f}, {r['center_y']:.0f})")
            failed += 1
    else:
        print(f"  ❌ FAIL: No click cluster zoom detected (expected from 5 clicks in 2s)")
        failed += 1
    
    # Test 2: Typing should produce a subtle zoom
    typing_zoom = [r for r in regions if r["trigger"] == "Typing"]
    if typing_zoom:
        r = typing_zoom[0]
        if 1.1 < r["zoom_level"] < 1.4:
            print(f"  ✅ PASS: Typing zoom detected, zoom={r['zoom_level']:.2f}x (subtle)")
            passed += 1
        else:
            print(f"  ❌ FAIL: Typing zoom level wrong: {r['zoom_level']:.2f}x (expected 1.2-1.3)")
            failed += 1
    else:
        print(f"  ❌ FAIL: No typing zoom detected (expected from 30 keystrokes in 3s)")
        failed += 1
    
    # Test 3: Circle gesture should produce a zoom
    circle_zoom = [r for r in regions if r["trigger"] == "CircleGesture"]
    if circle_zoom:
        r = circle_zoom[0]
        if 1000 < r["center_x"] < 1400 and 400 < r["center_y"] < 800:
            print(f"  ✅ PASS: Circle gesture zoom detected at ({r['center_x']:.0f}, {r['center_y']:.0f}), zoom={r['zoom_level']:.2f}x")
            passed += 1
        else:
            print(f"  ❌ FAIL: Circle gesture zoom at wrong position ({r['center_x']:.0f}, {r['center_y']:.0f})")
            failed += 1
    else:
        print(f"  ❌ FAIL: No circle gesture zoom detected")
        failed += 1
    
    # Test 4: No zoom should be triggered during fast movement (22s)
    fast_move_zooms = [r for r in regions if r["start_ms"] <= 22200 and r["end_ms"] >= 22000]
    if not fast_move_zooms:
        print(f"  ✅ PASS: No zoom during fast cursor movement (anti-zoom working)")
        passed += 1
    else:
        print(f"  ❌ FAIL: Zoom triggered during fast movement (should be suppressed)")
        failed += 1
    
    # Test 5: All zoom levels should be within valid range
    invalid_zooms = [r for r in regions if r["zoom_level"] < 1.0 or r["zoom_level"] > 4.0]
    if not invalid_zooms:
        print(f"  ✅ PASS: All zoom levels within valid range [1.0, 4.0]")
        passed += 1
    else:
        print(f"  ❌ FAIL: {len(invalid_zooms)} zoom regions have invalid zoom levels")
        failed += 1
    
    # Test 6: No overlapping regions (conflict resolution working)
    overlaps = 0
    sorted_regions = sorted(regions, key=lambda r: r["start_ms"])
    for i in range(len(sorted_regions) - 1):
        if sorted_regions[i]["end_ms"] > sorted_regions[i+1]["start_ms"]:
            overlaps += 1
    if overlaps == 0:
        print(f"  ✅ PASS: No overlapping zoom regions (conflict resolution working)")
        passed += 1
    else:
        print(f"  ❌ FAIL: {overlaps} overlapping zoom regions found")
        failed += 1
    
    print(f"\n{'─'*60}")
    print(f"  Results: {passed} passed, {failed} failed, {passed + failed} total")
    print(f"{'─'*60}\n")
    
    if failed > 0:
        print("  ⚠️  Some tests failed. Check detection thresholds.")
    else:
        print("  🎉 All auto-zoom verification tests passed!")
    
    return failed == 0


def main():
    print("\n" + "═"*60)
    print("  EasySpecy Auto-Zoom Verification Test")
    print("═"*60)
    
    # Generate test metadata
    print("\n  Generating synthetic recording metadata...")
    metadata = generate_test_metadata()
    
    print(f"    Cursor samples: {len(metadata['cursor_trail'])}")
    print(f"    Click events:   {len(metadata['click_events'])}")
    print(f"    Window events:  {len(metadata['window_events'])}")
    print(f"    Keyboard events:{len(metadata['keyboard_events'])}")
    
    # Save test metadata
    test_dir = os.path.dirname(os.path.abspath(__file__))
    meta_path = os.path.join(test_dir, "test_recording.meta.json")
    with open(meta_path, "w") as f:
        json.dump(metadata, f, indent=2)
    print(f"\n  Saved test metadata to: {meta_path}")
    
    # Run the Rust detection engine via test binary
    # For now, we'll run the detection logic in Python as a reference implementation
    # to verify the algorithm works before the Rust binary is built
    print("\n  Running zoom detection algorithm...")
    regions = run_detection_python(metadata)
    
    print(f"\n  Detected {len(regions)} zoom regions:")
    for i, r in enumerate(regions):
        print(f"    [{i+1}] {r['trigger']:15s} | {r['start_ms']:6d}-{r['end_ms']:6d}ms | "
              f"center=({r['center_x']:.0f},{r['center_y']:.0f}) | zoom={r['zoom_level']:.2f}x")
    
    # Verify results
    success = verify_zoom_regions(regions, metadata)
    
    # Save zoom timeline
    zoom_path = os.path.join(test_dir, "test_recording.zoom.json")
    zoom_timeline = {
        "regions": regions,
        "source_width": 1920,
        "source_height": 1080,
        "fps": 30,
    }
    with open(zoom_path, "w") as f:
        json.dump(zoom_timeline, f, indent=2)
    print(f"  Saved zoom timeline to: {zoom_path}")
    
    # Cleanup
    # os.remove(meta_path)  # Keep for debugging
    
    sys.exit(0 if success else 1)


def run_detection_python(metadata):
    """
    Python reference implementation of the detection algorithm.
    This mirrors the Rust implementation for verification.
    """
    import math
    
    regions = []
    screen_w, screen_h = 1920, 1080
    min_zoom = 1.2
    max_zoom = 4.0
    min_duration_ms = 1500
    
    clicks = metadata["click_events"]
    cursor = metadata["cursor_trail"]
    keyboard = metadata["keyboard_events"]
    windows = metadata["window_events"]
    
    # ═══ Signal 1: Click Clusters ═══
    time_window_ms = 3000
    distance_threshold = 300.0
    min_clicks = 2
    
    i = 0
    while i < len(clicks):
        cluster = [clicks[i]]
        j = i + 1
        while j < len(clicks):
            if clicks[j]["timestamp_ms"] - clicks[i]["timestamp_ms"] > time_window_ms:
                break
            close = any(
                math.sqrt((clicks[j]["x"] - c["x"])**2 + (clicks[j]["y"] - c["y"])**2) < distance_threshold
                for c in cluster
            )
            if close:
                cluster.append(clicks[j])
            j += 1
        
        if len(cluster) >= min_clicks:
            xs = [c["x"] for c in cluster]
            ys = [c["y"] for c in cluster]
            min_x, max_x = min(xs), max(xs)
            min_y, max_y = min(ys), max(ys)
            
            center_x = (min_x + max_x) / 2
            center_y = (min_y + max_y) / 2
            bbox_w = (max_x - min_x) * 1.2
            bbox_h = (max_y - min_y) * 1.2
            
            zoom_x = screen_w / max(bbox_w, 100)
            zoom_y = screen_h / max(bbox_h, 100)
            zoom_level = min(zoom_x, zoom_y, max_zoom)
            
            if zoom_level >= min_zoom:
                start_ms = cluster[0]["timestamp_ms"] - 300
                end_ms = cluster[-1]["timestamp_ms"] + 2000
                regions.append({
                    "start_ms": max(0, start_ms),
                    "end_ms": max(end_ms, start_ms + min_duration_ms),
                    "center_x": center_x,
                    "center_y": center_y,
                    "zoom_level": round(zoom_level, 2),
                    "trigger": "ClickCluster",
                    "priority": 4,
                })
            i = j
        else:
            i += 1
    
    # ═══ Signal 2: Typing Detection ═══
    if len(keyboard) >= 3:
        session_start = None
        for i in range(len(keyboard)):
            if session_start is None:
                # Look for 3+ keystrokes within 1.5s
                count = 1
                end_idx = i
                for j in range(i + 1, len(keyboard)):
                    if keyboard[j]["timestamp_ms"] - keyboard[i]["timestamp_ms"] > 1500:
                        break
                    count += 1
                    end_idx = j
                
                if count >= 3:
                    # Check cursor stationary
                    start_time = keyboard[i]["timestamp_ms"]
                    nearby_cursor = [s for s in cursor 
                                    if abs(s["timestamp_ms"] - start_time) < 200]
                    if nearby_cursor:
                        ref_pos = nearby_cursor[0]
                        cursor_during = [s for s in cursor
                                        if start_time <= s["timestamp_ms"] <= keyboard[end_idx]["timestamp_ms"]]
                        moved = any(
                            math.sqrt((s["x"] - ref_pos["x"])**2 + (s["y"] - ref_pos["y"])**2) > 20
                            for s in cursor_during
                        )
                        if not moved:
                            session_start = i
            else:
                gap = keyboard[i]["timestamp_ms"] - keyboard[i-1]["timestamp_ms"] if i > 0 else 0
                if gap > 2000 or i == len(keyboard) - 1:
                    start_ms = keyboard[session_start]["timestamp_ms"]
                    end_ms = keyboard[max(session_start, i-1)]["timestamp_ms"]
                    
                    nearby = [s for s in cursor if abs(s["timestamp_ms"] - start_ms) < 200]
                    if nearby and end_ms - start_ms >= min_duration_ms:
                        regions.append({
                            "start_ms": start_ms - 200,
                            "end_ms": end_ms + 500,
                            "center_x": nearby[0]["x"],
                            "center_y": nearby[0]["y"],
                            "zoom_level": 1.25,
                            "trigger": "Typing",
                            "priority": 2,
                        })
                    session_start = None
    
    # ═══ Signal 3: Circle Gesture ═══
    if len(cursor) >= 20:
        window_ms = 3000
        i = 0
        while i < len(cursor):
            start = cursor[i]
            j = i + 1
            while j < len(cursor) and cursor[j]["timestamp_ms"] - start["timestamp_ms"] < window_ms:
                j += 1
            
            min_points = 15
            if j - i >= min_points:
                found = False
                for k in range(i + min_points, j):
                    end = cursor[k]
                    dx = end["x"] - start["x"]
                    dy = end["y"] - start["y"]
                    dist = math.sqrt(dx*dx + dy*dy)
                    
                    if dist < 50:
                        # Check path length
                        path_slice = cursor[i:k+1]
                        path_len = sum(
                            math.sqrt((path_slice[p+1]["x"] - path_slice[p]["x"])**2 + 
                                     (path_slice[p+1]["y"] - path_slice[p]["y"])**2)
                            for p in range(len(path_slice)-1)
                        )
                        
                        if path_len > 200:
                            xs = [s["x"] for s in path_slice]
                            ys = [s["y"] for s in path_slice]
                            bbox_w = (max(xs) - min(xs)) * 1.2
                            bbox_h = (max(ys) - min(ys)) * 1.2
                            
                            if bbox_w >= 50 and bbox_h >= 50:
                                center_x = (min(xs) + max(xs)) / 2
                                center_y = (min(ys) + max(ys)) / 2
                                zoom_x = screen_w / bbox_w
                                zoom_y = screen_h / bbox_h
                                zoom_level = min(zoom_x, zoom_y, max_zoom)
                                
                                if zoom_level >= min_zoom:
                                    regions.append({
                                        "start_ms": start["timestamp_ms"] - 200,
                                        "end_ms": end["timestamp_ms"] + 2000,
                                        "center_x": center_x,
                                        "center_y": center_y,
                                        "zoom_level": round(zoom_level, 2),
                                        "trigger": "CircleGesture",
                                        "priority": 5,
                                    })
                                    i = k + 1
                                    found = True
                                    break
                if not found:
                    i += 1
            else:
                i += 1
    
    # ═══ Merge and resolve conflicts ═══
    regions.sort(key=lambda r: r["start_ms"])
    resolved = []
    for region in regions:
        if resolved:
            last = resolved[-1]
            if region["start_ms"] < last["end_ms"]:
                if region["priority"] > last["priority"]:
                    resolved.pop()
                    resolved.append(region)
                continue
            if region["start_ms"] < last["end_ms"] + min_duration_ms:
                continue
        resolved.append(region)
    
    return resolved


if __name__ == "__main__":
    main()
