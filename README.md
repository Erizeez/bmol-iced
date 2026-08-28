# Liquid Glass for Rust

`liquid-glass-rs` 是一个面向桌面应用的 Rust Liquid Glass UI Framework。
它的核心不是 Iced Theme，而是独立的 Backdrop-aware GPU Compositor：

```text
Iced UI → Liquid Scene → Liquid Compositor → wgpu → Metal / Vulkan / DX12
```

## 当前状态

当前是 foundation 版本，已建立以下边界：

- `liquid-glass-scene`：Shape、Material、Backdrop、GlassNode
- `liquid-glass-render`：RenderGraph、TexturePool、Renderer contract
- `liquid-glass-animation`：Spring 基础类型
- `liquid-glass-ui`：Container/Button 到 Scene 的初始映射
- `liquid-glass-platform`：DPI 与窗口配置边界
- `liquid-glass`：对外统一 facade

`liquid-glass-render` 已包含第一个真实 `wgpu` offscreen backend：背景场景 Pass、SDF Glass Pass 和最终离屏纹理。Iced Widget adapter、独立 separable blur 和 Surface/present 生命周期仍在后续阶段。

## 运行 playground

```bash
cargo run -p liquid-glass-playground
```

该命令会创建一个原生 `winit` 窗口，配置 `wgpu` Surface，并持续渲染一个 SDF GlassPanel。窗口支持 Resize、Surface 重建和关闭事件。

如果本机没有可用 GPU，playground 会保留打印纯 Rust foundation 信息，并报告 GPU backend 不可用；这不影响 workspace 的单元测试。

## 参考项目

当前目录下的 [`liquid-glass-studio/`](./liquid-glass-studio/) 保留为视觉、SDF 和 WGSL 算法参考，不作为 Rust workspace 成员。

## 开发顺序

1. 将现有多点采样模糊拆成 downsample + horizontal/vertical blur passes。
2. 将 `GpuRenderer` 接入 Surface 之外的完整 DPI 与色彩空间生命周期。
3. 实现 `GlassContainer` 的 Iced adapter。
4. 在 playground 中加入 blur、refraction、tint 的实时调节。
5. 再扩展 Button、Panel、Toolbar 和 Spring interaction。

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT), at your option.
