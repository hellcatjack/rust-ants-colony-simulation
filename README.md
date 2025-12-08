# Rust Ant Colony Simulation

A high-performance ant colony simulation written in Rust using the [Bevy](https://bevyengine.org/) engine. This simulation demonstrates emergent behavior where ants find optimal paths between their nest and food sources using pheromone trails.

![screenshot](assets/processed/nest.png)

## Features

- **High Performance**: Capable of simulating thousands of ants efficiently using spatial hashing and KD-Trees.
- **Emergent Behavior**: Ants follow simple local rules to form complex global pathfinding networks.
- **Real-time Tuning**: Adjust simulation parameters on the fly without restarting.
- **Interactive UI**: Control visuals and simulation settings via a GUI.

## How to Run (Windows)

**Recommended Method:**
Use the provided PowerShell script to run the project. this avoids common "file in use" / locking errors during compilation on Windows by building in a temporary directory.

```powershell
.\run_safe.ps1
```

**Standard Method:**
```bash
cargo run --release
```
*Note: If you encounter linking errors or "file used by another process" errors, please use the `run_safe.ps1` script.*

## Controls & Shortcuts

### Keyboard Shortcuts
| Key | Action |
| --- | --- |
| **TAB** | Toggle Settings Menu (Open/Close UI) |
| **H** | Toggle Home Pheromone Visibility |
| **F** | Toggle Food Pheromone Visibility |
| **P** | Toggle Debug Paths (Sensor lines & Radius) |
| **A** | Toggle Ant Visibility |
| **-** | Reduce Speed (Limit FPS: 60 -> 30) |
| **=** | Increase Speed (Unlimited FPS) |
| **ESC**| Exit Simulation |

### UI Parameters (Press TAB)
You can tweak these values in real-time to see how they affect the colony's behavior:

- **Env Ph Decay**: How fast pheromones on the ground evaporate.
- **Ant Ph Decay**: How fast the pheromone strength carried by an ant decays.
- **Sensor Dist**: How far ahead an ant looks for pheromones.
- **Sensor Angle**: The width of the ant's sensing field.
- **Randomness**: The amount of random jitter in ant movement.
- **Update Interval**: How often ants make steering decisions (Lower = Smoother/Smarter but more CPU intensive).

### Reset
- **Reset Simulation**: Clears the map and respawns ants at the nest. Useful after drastically changing parameters.

## Configuration
The initial static configuration constants are located in `src/configs.rs`. However, many of these can now be overridden at runtime via the UI.

## Assets
Original assets located in `assets/`.
processed sprites sheets are in `assets/processed/`.

## License
MIT
