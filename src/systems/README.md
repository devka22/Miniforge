# MiniForge runtime systems

The runtime systems are deliberately small facades over specialized engines and
shared data structures. Keep game state in `GameObject`; keep expensive queries
behind the libraries listed below.

| Area | Backend / strategy | Runtime contract |
| --- | --- | --- |
| Physics | `rapier2d` validation + `rstar` broad phase | Finite `dt`, maximum 50 ms step, unique contact pairs and frame metrics |
| Rendering | `rstar` camera culling | Submit enabled, active, visible entities intersecting the camera AABB |
| Movement | `rayon` above 256 entities | Skip inactive entities, sanitize non-finite state, maximum 100 ms step |
| Particles | `rayon` above 256 live particles per emitter | Bounded particle counts and finite, clamped steps |
| Navigation / commands | `pathfinding` A*, Dijkstra and shared flow fields | Clean blocked targets before issuing orders |
| Audio | `kira` playback/tweens | Per-bus gain, pause/stop state and a bounded 256-command diagnostic history |
| Animation | controller graph + allocation-free key sampling | State speed, loop/finish, pause, transitions and event emission |
| Input | queued frame events | Platform events are applied after the previous-state snapshot so edges last one frame |
| UI / editor | `UiRuntime` + validation facade | Observable events and frame diagnostics rather than marker structs |
| Visual scripting | guarded graph interpreter | Finite `dt`, 128-node chain guard and 4096-node global frame budget |
| Gameplay / RTS / 2D runtime | component-driven simulation | Reject non-finite frame deltas and cap simulation spikes |

The spatial index is rebuilt per frame from lightweight AABBs. This fits the
current `Vec<GameObject>` ownership model and avoids introducing persistent
handles or synchronization into scene serialization. If scenes grow beyond the
point where rebuilding dominates, move the index to an incremental resource and
update entries only when transforms or colliders change.

No C layer is used in this pass. Rust libraries already cover the measured hot
paths without an FFI boundary; native C/C++ remains available through the engine's
existing plugin ABI for third-party integrations that genuinely need it.
