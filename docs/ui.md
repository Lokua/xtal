# Keyboard Shortcuts

Xtal uses keyboard shortcuts that work regardless of whether the control panel
(web view) or the main sketch window (OS window) is focused. Some shortcuts are
context-specific or have slightly different behaviors depending on which window
is active.

> **Note:** On macOS, use `Cmd`. On Windows/Linux, use `Ctrl`.

## Playback & Animation

| Shortcut | Action        | Description                                                                                                                                                                                     |
| -------- | ------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `P`      | Play/Pause    | Toggle playback. When paused, use Advance to manually step through frames.                                                                                                                      |
| `A`      | Advance Frame | When paused, manually advances a single frame.                                                                                                                                                  |
| `R`      | Reset         | Reset the frame counter and all animations.                                                                                                                                                     |
| `Space`  | Tap Tempo     | When tap tempo is enabled, tap to set the BPM. Note: Keeping tap tempo enabled will preserve the tapped-in tempo when switching sketches; disabling will revert to the sketch's configured BPM. |

## Snapshots

Snapshots allow you to store and recall up to 10 parameter configurations.

| Shortcut           | Action          | Description                                                                    |
| ------------------ | --------------- | ------------------------------------------------------------------------------ |
| `S`                | Snapshot Editor | Open the Snapshot Editor to store and recall snapshots. _(Control panel only)_ |
| `Shift` + `0-9`    | Store Snapshot  | Save the current UI control states to the specified slot.                      |
| `Cmd/Ctrl` + `0-9` | Recall Snapshot | Load a previously saved snapshot from the specified slot.                      |

## Randomization & Exclusions

| Shortcut         | Action     | Description                                                                                        |
| ---------------- | ---------- | -------------------------------------------------------------------------------------------------- |
| `Cmd/Ctrl` + `R` | Randomize  | Randomize all UI controls (respects exclusions).                                                   |
| `E`              | Exclusions | Open the Exclusions panel to select controls to exclude from randomization. _(Control panel only)_ |

## Saving & Loading

| Shortcut                   | Action | Description                                                       |
| -------------------------- | ------ | ----------------------------------------------------------------- |
| `Cmd/Ctrl` + `S`           | Save   | Save UI control states and MIDI mappings for this sketch to disk. |
| `Shift` + `Cmd/Ctrl` + `R` | Reload | Reload the current sketch back to its last saved state.           |

## Image Capture

| Shortcut         | Action        | Description                                                               |
| ---------------- | ------------- | ------------------------------------------------------------------------- |
| `Cmd/Ctrl` + `I` | Capture Image | Capture a PNG screenshot to disk. _(Control panel)_                       |
| `S`              | Capture Image | Capture a PNG screenshot to disk. _(Main window only, when no modifiers)_ |

## Window Management

| Shortcut | Action            | Description                                                            |
| -------- | ----------------- | ---------------------------------------------------------------------- |
| `F`      | Toggle Fullscreen | Toggle the main sketch window between fullscreen and windowed mode.    |
| `M`      | Toggle Main Focus | Switch focus to the main sketch window.                                |
| `G`      | Toggle GUI Focus  | Switch focus to the control panel. _(Main window only)_                |
| `,`      | Toggle View       | Switch between Controls view and Settings view. _(Control panel only)_ |

## Application

| Shortcut         | Action      | Description                                           |
| ---------------- | ----------- | ----------------------------------------------------- |
| `Cmd/Ctrl` + `Q` | Quit        | Exit the application.                                 |
| `/`              | Toggle Help | Show or hide the help overlay. _(Control panel only)_ |

## UI Control Interactions

These interactions apply to sliders and number boxes in the control panel:

| Interaction               | Action                                   |
| ------------------------- | ---------------------------------------- |
| Click label               | Randomize that single parameter          |
| `Cmd/Ctrl` + Click label  | Revert parameter to its last saved value |
| Drag number box           | Coarse adjustment                        |
| `Shift` + Drag number box | Fine adjustment                          |
| Double-click number box   | Enable manual keyboard entry             |

## Notes

- **Performance Mode:** When enabled, prevents Xtal from applying a sketch's
  default width and height and disables automatic window repositioning.
  Essential for live performance when you want to keep the window fullscreen
  while switching sketches.

- **MIDI Clock Sync:** The frame counter and animations can be synced to an
  external MIDI clock source configured in Settings.

- **Transition Time:** Snapshot recalls and randomizations animate over the
  configured transition time (in beats).
