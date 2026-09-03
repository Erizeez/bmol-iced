# Liquid Glass for Rust

`liquid-glass-rs` 是一个面向桌面应用的 Rust Liquid Glass UI Framework。
它的核心不是 Iced Theme，而是独立的 Backdrop-aware GPU Compositor：

```text
Iced UI → Liquid Scene → Liquid Compositor → wgpu → Metal / Vulkan / DX12
```

## 当前状态

当前是 foundation 版本，已建立以下边界：

- `liquid-glass-scene`：Shape、continuous/circular `CornerCurve`、Material、Backdrop、GlassNode、z-order scene
- `liquid-glass-geometry`：独立的 G2 continuous corner/capsule 曲线、路径段和宽高比渐进算法
- `liquid-glass-render`：RenderGraph、TexturePool、Renderer contract
- `liquid-glass-animation`：Spring 基础类型
- `liquid-glass-ui`：Iced `GlassContainer` / `GlassButton` / 任意数量的 `GlassSegmentedControl`、逐段 enabled/disabled 状态，以及 Light/Dark 语义主题
- `liquid-glass-platform`：DPI、透明窗口、OS desktop backdrop 配置与平台帧 provider contract
- `liquid-glass`：对外统一 facade

`liquid-glass-render` 已包含真实 `wgpu` compositor：参考项目的背景 Pass、full-resolution horizontal/vertical Gaussian blur、连续超椭圆角 SDF、折射、色散、Fresnel、glare、tint/whiteness 合成，以及按 `z_index` 绘制多个玻璃节点。圆角矩形默认使用 exponent 5 的 continuous curve，也可显式切换为 circular 或自定义 exponent；胶囊则使用 `liquid-glass-geometry` 从 Kyant0/Capsule 移植的 G2 profile，只保留两端外侧圆弧，并以曲率连续的三次 Bézier 肩部接入水平边。算法会随宽高比在正圆和完整胶囊之间渐进。`whiteness` 独立表达中性白覆盖量，使输入框可以在保留主题 tint 的同时比按钮更白；`ShadowStyle` 则将玻璃的边缘光学效果与有意的层级投影分开控制。`liquid-glass-ui` 提供 Iced layout 到 `GlassNode` 的桥接；原生 playground 会使用这份布局结果驱动 compositor。透明窗口现在可以通过 `BackdropFrame` / `DesktopBackdropProvider` 描述和注入真实桌面帧，`set_background_rgba8` 会把它接入完整 shader 链路；具体的 macOS、Windows、Wayland 采集实现仍需按平台权限和窗口排除策略接入。

## 运行 playground

```bash
cargo run -p liquid-glass-playground --bin liquid-glass-playground
```

运行来源算法的最小融合基准：

```bash
cargo run -p liquid-glass-playground --bin liquid-glass-reference-demo
```

这个基准只绘制一个玻璃节点，复现来源仓库的 `smin(circle, roundedRect)`
融合，以及融合边界上的折射、色散、Fresnel、高光、模糊和阴影；它与设置页
demo 的多节点层级合成分开，用于判断基础液态玻璃光学效果是否正确。

该命令会创建一个原生 `winit` 窗口，配置 `wgpu` Surface，并持续渲染由多个玻璃节点组成的场景。它是查看完整液态玻璃 shader 效果的入口，包含 full-resolution blur、refraction、dispersion、Fresnel 和 glare；窗口支持 Resize、Surface 重建和关闭事件。

运行 Iced custom widget 示例：

```bash
cargo run -p liquid-glass-playground --bin liquid-glass-iced-demo
```

该示例按 macOS System Settings 的 split-view 结构组织：232px 侧栏、侧栏搜索、右侧工具栏和收窄的分组设置列表。侧栏使用独立的 `GlassRole::Sidebar`：高模糊、较白、半透明，窗口透明区域由 OS 直接透出真实桌面；macOS 使用窗口 compositor blur，Windows 使用 Acrylic，Wayland 则使用 compositor 提供的 blur 能力。这里不把 NSVisualEffectView 叠到 wgpu 的 CAMetalLayer 上方，以免遮住 Iced 内容。列表、分割线和设置条目保持常规 UI 材质，顶部工具栏、搜索框以及前进/后退分段控件选择性使用玻璃。示例中的“前进”段为 disabled，用来检查弱化图标以及 hover 不改变圆形反馈和分隔线的行为。`Automatic / Light / Dark` 会同步切换 Iced 标准控件、普通 UI 语义颜色以及玻璃 material/chrome，用来验证玻璃组件与普通 UI 在两种外观下的共存关系。当前示例已验证透明窗口与 Iced 内容可以共存；要让自定义玻璃内部显示桌面纹理，还需要接入上面的 platform frame provider。

如果本机没有可用 GPU，playground 会保留打印纯 Rust foundation 信息，并报告 GPU backend 不可用；这不影响 workspace 的单元测试。

## 参考项目

当前目录下的 [`liquid-glass-studio/`](./liquid-glass-studio/) 保留为视觉、SDF 和 WGSL 算法参考，不作为 Rust workspace 成员。

## 开发顺序

1. 为 macOS、Windows、Wayland 分别实现 `DesktopBackdropProvider`，将实时帧接入 `set_background_rgba8`，使自定义 refraction/dispersion 采样真实桌面，而不仅由 OS window blur 提供底层视觉。
2. 将任意 Iced 子树的实际布局边界接入 backdrop capture 区域，而不是使用 demo 场景映射。
3. 继续扩展 pointer spring、merge、动态背景纹理与多窗口 Surface 生命周期。

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT), at your option.

`liquid-glass-geometry` 中的 G2 构造移植自 Apache-2.0 的 Kyant0/Capsule；固定来源提交、改动范围和归属信息见该 crate 的 `NOTICE`。
