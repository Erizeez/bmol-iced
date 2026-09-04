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
- `liquid-glass-ui`：Iced `GlassContainer` / `GlassButton` / 任意数量的 `GlassSegmentedControl`、逐段 enabled/disabled 状态，以及 Light/Dark 语义主题；`components` 模块提供 macOS 设置风格的共享组件（`settings_group` / `setting_row` / `setting_link` / `setting_toggle` / `setting_slider` / `setting_pick_list` / `sidebar_item` / `search_field` / `section_heading` 等），`icon` 模块提供从系统提取的 SF Symbols 矢量图标（资产生成见 playground 的 `extract_sf_symbols` 开发工具），`font` 模块负责 UI 字体解析（构建期内嵌 `assets/fonts/`，macOS 运行时回退系统 SF 字体）
- `liquid-glass-platform`：DPI、透明窗口、OS desktop backdrop 配置与平台帧 provider contract
- `liquid-glass`：对外统一 facade

`liquid-glass-render` 已包含真实 `wgpu` compositor：参考项目的背景 Pass、full-resolution horizontal/vertical Gaussian blur、连续超椭圆角 SDF、折射、色散、Fresnel、glare、tint/whiteness 合成，以及按 `z_index` 绘制多个玻璃节点。圆角矩形默认使用 exponent 5 的 continuous curve，也可显式切换为 circular 或自定义 exponent；胶囊则使用 `liquid-glass-geometry` 从 Kyant0/Capsule 移植的 G2 profile，只保留两端外侧圆弧，并以曲率连续的三次 Bézier 肩部接入水平边。算法会随宽高比在正圆和完整胶囊之间渐进。`whiteness` 独立表达中性白覆盖量，使输入框可以在保留主题 tint 的同时比按钮更白；`ShadowStyle` 则将玻璃的边缘光学效果与有意的层级投影分开控制。`liquid-glass-ui` 提供 Iced layout 到 `GlassNode` 的桥接；原生 playground 会使用这份布局结果驱动 compositor。透明窗口现在可以通过 `BackdropFrame` / `DesktopBackdropProvider` 描述和注入真实桌面帧，`set_background_rgba8` 会把它接入完整 shader 链路；具体的 macOS、Windows、Wayland 采集实现仍需按平台权限和窗口排除策略接入。

运行锁定在来源仓库 `d13c3e5` 的原始融合基准：

```bash
cargo run -p liquid-glass-playground --bin liquid-glass-reference-demo
```

这个入口直接编译 `liquid-glass-studio/` 中的原始 WGSL，并保持其 uniform
默认值与背景、横向模糊、纵向模糊、玻璃四 Pass 顺序；项目增强不得进入这条
基准渲染路径。

并排比较原始算法与当前增强渲染器：

```bash
cargo run -p liquid-glass-playground --bin liquid-glass-comparison-demo
```

比较窗口左侧固定为原始 `d13c3e5`，右侧为当前增强版。两侧接收相同的画布
尺寸、形状、指针位置和共同材质参数，用于直接识别算法差异；任意一侧移动
鼠标都以所在半屏的局部坐标同步驱动两侧。

增强版采用分层光学：来源算法的 Snell 边缘折射、RGB 色散和 Fresnel 过渡
作为透射基底；固定上下方向的粗糙界面反射作为表面层；投射阴影最后独立合成，
不再进入玻璃自身的模糊与折射采样。这样保留液态透镜感，同时增加系统控件所需
的稳定上下高光、较暗侧边和悬浮层级。

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
