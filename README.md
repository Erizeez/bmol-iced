# bmol-iced

Modern Apple-style Liquid Glass UI framework, spring physics, and desktop window integrations for [Iced](https://github.com/iced-rs/iced).

---

## Ecosystem Architecture

`bmol-iced` is built on top of three focused, decoupled foundational crates:

```text
┌─────────────────────────────────────────────────────────────┐
│             liquid-rs (GPU Renderer & WGSL Compositor)       │
└──────────────────────────────┬──────────────────────────────┘
                               │
       ┌───────────────────────┴───────────────────────┐
       ▼                                               ▼
┌──────────────────────────────┐        ┌──────────────────────────────┐
│       bmol-window-shell      │        │         bmol-designs         │
│  - macOS SkyLight Blur       │        │  - Design Tokens & Colors    │
│  - EDR / HDR CAMetalLayer    │        │  - Apple Squircle Metrics    │
│  - Traffic Lights & Bounds   │        │  - Material Preset Tiers     │
└──────────────┬───────────────┘        └──────────────┬───────────────┘
               │                                       │
               └───────────────────┬───────────────────┘
                                   ▼
┌─────────────────────────────────────────────────────────────┐
│                          bmol-iced                          │
│  - GlassButton, GlassSegmentedControl, GlassChrome          │
│  - SpringScrollView (16ms Apple rubber-band momentum physics)│
│  - IcedWindowController (macOS borderless lifecycle glue)   │
└─────────────────────────────────────────────────────────────┘
```

- **[`liquid-rs`](https://github.com/Erizeez/liquid-rs)**: Framework-agnostic GPU compositor and real-time physical optical shaders.
- **[`bmol-window-shell`](../bmol-window-shell)**: Generic frameless macOS window shell, Stage Manager flicker guard, and EDR configuration.
- **[`bmol-designs`](../bmol-designs)**: Framework-neutral design tokens, Apple continuous squircle parameters, and semantic color palettes.
- **[`bmol-iced`](crates/bmol-iced)**: Iced widget implementations, spring-animated momentum scroll views, and window controllers.

---

## Running the Demos

All foundation and UI demos are located in `examples/playground`:

### 1. Liquid Glass Iced macOS Settings Demo
```bash
cargo run -p liquid-glass-playground --bin liquid-glass-iced-demo
```

### 2. Window Controls & Traffic Lights Demo
```bash
cargo run -p liquid-glass-playground --bin liquid-glass-window-controls-demo
```

### 3. Shader Comparison Demo
```bash
cargo run -p liquid-glass-playground --bin liquid-glass-comparison-demo
```

### 4. Raw Upstream Reference Benchmark Demo
```bash
cargo run -p liquid-glass-playground --bin liquid-glass-reference-demo
```

---

## License

Dual-licensed under either of:
- [MIT License](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)
