# Liquid Glass for Rust

`liquid-glass-rs` 是一个面向桌面应用的 Rust Liquid Glass UI Framework。
它的核心不是 Iced Theme，而是独立的 Backdrop-aware GPU Compositor：

```text
Iced UI → Liquid Scene → Liquid Compositor → wgpu → Metal / Vulkan / DX12
```

## 当前状态

当前是 foundation 版本，已建立以下边界：

- `liquid-glass-scene`：Shape、Material、Backdrop、GlassNode、z-order scene
- `liquid-glass-render`：RenderGraph、TexturePool、Renderer contract
- `liquid-glass-animation`：Spring 基础类型
- `liquid-glass-ui`：Iced `GlassContainer` layout bridge 与可交互 `GlassButton`
- `liquid-glass-platform`：DPI 与窗口配置边界
- `liquid-glass`：对外统一 facade

`liquid-glass-render` 已包含真实 `wgpu` compositor：参考项目的背景 Pass、full-resolution horizontal/vertical Gaussian blur、SDF、折射、色散、Fresnel、glare、tint 合成，以及按 `z_index` 绘制多个玻璃节点。`liquid-glass-ui` 提供 Iced layout 到 `GlassNode` 的桥接；原生 playground 会使用这份布局结果驱动 compositor。

## 运行 playground

```bash
cargo run -p liquid-glass-playground --bin liquid-glass-playground
```

该命令会创建一个原生 `winit` 窗口，配置 `wgpu` Surface，并持续渲染由多个玻璃节点组成的场景。它是查看完整液态玻璃 shader 效果的入口，包含 full-resolution blur、refraction、dispersion、Fresnel 和 glare；窗口支持 Resize、Surface 重建和关闭事件。

运行 Iced custom widget 示例：

```bash
cargo run -p liquid-glass-playground --bin liquid-glass-iced-demo
```

该示例是一个可交互的 Liquid Glass Dashboard layout preview：背景包含网格、光斑、环形高光和斜向纹理，前景组合了侧栏、工具栏、统计卡片、搜索框、滑杆、进度条、开关、复选框、活动列表和两个 `GlassButton`。当前标准 Iced runtime 负责控件和交互；要查看真实 compositor shader，请运行上面的原生 playground。

如果本机没有可用 GPU，playground 会保留打印纯 Rust foundation 信息，并报告 GPU backend 不可用；这不影响 workspace 的单元测试。

## 参考项目

当前目录下的 [`liquid-glass-studio/`](./liquid-glass-studio/) 保留为视觉、SDF 和 WGSL 算法参考，不作为 Rust workspace 成员。

## 开发顺序

1. 将完整 compositor 接入 Iced renderer 的 Surface 生命周期。
2. 在 playground 中加入 blur、refraction、tint 的实时调节。
3. 将任意 Iced 子树放入真实玻璃 backdrop，并继续扩展 Spring interaction。

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT), at your option.
