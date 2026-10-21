# Controls

| Action | Control |
| --- | --- |
| Sculpt | Left drag on mesh |
| Smooth temporarily | Shift + drag |
| Mask temporarily | Ctrl + drag |
| Invert temporarily | Alt + drag |
| Select brush | 1–9, Morph 0, Pinch P, Twist J, Local Scale L, Drag G |
| Brush radius | `[` / `]` |
| Orbit / pan / zoom | Right drag / middle drag / scroll |
| Frame / X symmetry / wireframe | F / X / W |
| Transform mode / cancel preview | T / Escape |
| Undo / redo | Cmd or Ctrl Z / Cmd or Ctrl Shift Z |
| Save / open | Cmd or Ctrl S / Cmd or Ctrl O |
| Help | H |

Browser pen pressure modulates strength by default; radius modulation is optional. Mouse input retains full nominal pressure. Native force events are consumed when the platform provides them, but physical tablet input, calibration and touch gestures remain unverified. **Import PNG alpha** loads an 8-bit grayscale weight image, up to 1024 × 1024 pixels / 16 MiB; white sculpts, black/transparent pixels protect. Rotation is adjustable. Alpha detail depends on geometry density; brush presets and alpha persistence remain gaps. Small windows expose object/surface controls through the compact toolbar.

Store a shape with **Store target** in Object before sculpting. **Morph** (**0**) brushes back toward it; **Restore shape** recalls all saved points, and **Swap** exchanges the current and stored shapes. Masks protect local Morph strokes, while full Restore keeps paint/masks and restores every point. Targets remain tied to their creation subdivision level; explicit refinement/rebuilding removes them with undo. See [MORPH_TARGETS.md](MORPH_TARGETS.md) for persistence and limits.

