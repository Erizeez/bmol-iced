//! UI view components: traffic lights, headers, pickers, cards, and status bar.

use iced::{
    Alignment, Background, Border, Color, Element, Length, Padding, Shadow, Theme, Vector,
    font::Weight,
    mouse,
    widget::{button, column, container, row, space, text},
};
use bmol_designs::popover_metrics::{PopoverArrowEdge, PopoverArrowPreset};
use bmol_window_shell::{
    TrafficLightsViewConfig, WindowControlAction, traffic_lights, view_traffic_lights,
};
use liquid_glass::{ContextMenu, ControlAction, UiPalette, ui::font};
use vibrancy_rs::KawasePassPlan;

use crate::menu_content::{demo_metrics, MenuContentPreset};
use crate::state::{DragTarget, Message, State};
use crate::vibrancy::{BlurPreset, WallpaperStyle};

/// Builds the wallpaper style picker.
#[must_use]
pub fn view_wallpaper_picker<'a>(
    state: &'a State,
    palette: &'a UiPalette,
    is_dark: bool,
) -> Element<'a, Message, Theme, iced::Renderer> {
    let mut picker = row![].spacing(4.0).align_y(Alignment::Center);
    for &style in &WallpaperStyle::ALL {
        let is_selected = state.wallpaper == style;
        let btn = button(
            text(style.label())
                .size(11.0)
                .font(font::ui_font(if is_selected {
                    Weight::Semibold
                } else {
                    Weight::Normal
                }))
                .color(if is_selected {
                    Color::WHITE
                } else {
                    palette.text_secondary
                }),
        )
        .padding(Padding {
            top: 4.0,
            right: 10.0,
            bottom: 4.0,
            left: 10.0,
        })
        .style(move |_theme, _status| {
            if is_selected {
                button::Style {
                    background: Some(Background::Color(palette.accent)),
                    border: Border::default().rounded(6.0),
                    ..button::Style::default()
                }
            } else {
                button::Style {
                    background: Some(Background::Color(if is_dark {
                        Color::from_rgba(1.0, 1.0, 1.0, 0.08)
                    } else {
                        Color::from_rgba(0.0, 0.0, 0.0, 0.06)
                    })),
                    border: Border::default().rounded(6.0),
                    ..button::Style::default()
                }
            }
        })
        .on_press(Message::SetWallpaper(style));
        picker = picker.push(btn);
    }
    picker.into()
}

/// Builds the blur strength selector.
#[must_use]
pub fn view_blur_preset_picker<'a>(
    state: &'a State,
    palette: &'a UiPalette,
    is_dark: bool,
) -> Element<'a, Message, Theme, iced::Renderer> {
    let mut picker = row![].spacing(4.0).align_y(Alignment::Center);
    for &preset in &BlurPreset::ALL {
        let is_selected = state.blur_preset == preset;
        let btn = button(
            text(preset.label())
                .size(11.0)
                .font(font::ui_font(if is_selected {
                    Weight::Semibold
                } else {
                    Weight::Normal
                }))
                .color(if is_selected {
                    Color::WHITE
                } else {
                    palette.text_secondary
                }),
        )
        .padding(Padding {
            top: 4.0,
            right: 9.0,
            bottom: 4.0,
            left: 9.0,
        })
        .style(move |_theme, _status| {
            if is_selected {
                button::Style {
                    background: Some(Background::Color(palette.accent)),
                    border: Border::default().rounded(6.0),
                    ..button::Style::default()
                }
            } else {
                button::Style {
                    background: Some(Background::Color(if is_dark {
                        Color::from_rgba(1.0, 1.0, 1.0, 0.08)
                    } else {
                        Color::from_rgba(0.0, 0.0, 0.0, 0.06)
                    })),
                    border: Border::default().rounded(6.0),
                    ..button::Style::default()
                }
            }
        })
        .on_press(Message::SetBlurPreset(preset));
        picker = picker.push(btn);
    }
    picker.into()
}

/// Builds the popover arrow / beak (触角) edge selector.
#[must_use]
pub fn view_popover_arrow_picker<'a>(
    state: &'a State,
    palette: &'a UiPalette,
    is_dark: bool,
) -> Element<'a, Message, Theme, iced::Renderer> {
    const EDGES: [(PopoverArrowEdge, &str); 5] = [
        (PopoverArrowEdge::None, "无触角"),
        (PopoverArrowEdge::Top, "▲ 顶"),
        (PopoverArrowEdge::Bottom, "▼ 底"),
        (PopoverArrowEdge::Left, "◀ 左"),
        (PopoverArrowEdge::Right, "▶ 右"),
    ];

    let mut picker = row![].spacing(3.0).align_y(Alignment::Center);
    for &(edge, label) in &EDGES {
        let is_selected = state.popover_arrow_edge == edge;
        let btn = button(
            text(label)
                .size(10.5)
                .font(font::ui_font(if is_selected {
                    Weight::Semibold
                } else {
                    Weight::Normal
                }))
                .color(if is_selected {
                    Color::WHITE
                } else {
                    palette.text_secondary
                }),
        )
        .padding(Padding {
            top: 3.5,
            right: 7.0,
            bottom: 3.5,
            left: 7.0,
        })
        .style(move |_theme, _status| {
            if is_selected {
                button::Style {
                    background: Some(Background::Color(palette.accent)),
                    border: Border::default().rounded(5.0),
                    ..button::Style::default()
                }
            } else {
                button::Style {
                    background: Some(Background::Color(if is_dark {
                        Color::from_rgba(1.0, 1.0, 1.0, 0.08)
                    } else {
                        Color::from_rgba(0.0, 0.0, 0.0, 0.06)
                    })),
                    border: Border::default().rounded(5.0),
                    ..button::Style::default()
                }
            }
        })
        .on_press(Message::SetPopoverArrow(edge));
        picker = picker.push(btn);
    }
    picker.into()
}

/// Builds the top fused header bar managed by `loyal_drag_bar`.
#[must_use]
pub fn view_top_header<'a>(
    state: &'a State,
    palette: &'a UiPalette,
    is_dark: bool,
) -> Element<'a, Message, Theme, iced::Renderer> {
    let header_height = state.controller.metrics.header_rect.height;
    let symmetric_margin = ((header_height - traffic_lights::DIAMETER) * 0.5).max(4.0);
    let slop = traffic_lights::control_hover_slop(traffic_lights::DIAMETER);
    let spacer_left = (symmetric_margin - slop).max(0.0);

    let tl_config = TrafficLightsViewConfig::from_state(
        &state.traffic_lights,
        state.controller.is_focused,
        is_dark,
    );

    let traffic_lights = row![
        space().width(Length::Fixed(spacer_left)),
        view_traffic_lights(
            tl_config,
            |action| match action {
                WindowControlAction::Close => Message::WindowControl(ControlAction::Close),
                WindowControlAction::Minimize => Message::WindowControl(ControlAction::Minimize),
                WindowControlAction::Zoom | WindowControlAction::Expand => {
                    Message::WindowControl(ControlAction::Expand)
                }
            },
            Message::TrafficLightsHover,
        ),
    ]
    .align_y(Alignment::Center);

    let title_text = row![
        space().width(Length::Fixed(traffic_lights::TITLE_CLEARANCE)),
        text("Liquid Glass Context Menu")
            .size(13.5)
            .font(font::ui_font(Weight::Semibold))
            .color(palette.text_primary),
    ]
    .align_y(Alignment::Center);

    let arrow_picker = view_popover_arrow_picker(state, palette, is_dark);
    let wallpaper_picker = view_wallpaper_picker(state, palette, is_dark);
    let blur_picker = view_blur_preset_picker(state, palette, is_dark);

    let preset_btn = button(
        text(match state.menu_preset {
            MenuContentPreset::DockReference1To1 => "🍎 1:1参考图模式",
            MenuContentPreset::FullShowcase => "📑 全功能模式",
        })
        .size(10.5)
        .font(font::ui_font(Weight::Semibold))
        .color(if state.menu_preset == MenuContentPreset::DockReference1To1 {
            Color::WHITE
        } else {
            palette.text_primary
        }),
    )
    .padding(Padding {
        top: 4.0,
        right: 8.0,
        bottom: 4.0,
        left: 8.0,
    })
    .style(move |_theme, _status| {
        if state.menu_preset == MenuContentPreset::DockReference1To1 {
            button::Style {
                background: Some(Background::Color(Color::from_rgb(0.18, 0.55, 0.95))),
                border: Border::default().rounded(6.0),
                ..button::Style::default()
            }
        } else {
            button::Style {
                background: Some(Background::Color(if is_dark {
                    Color::from_rgba(1.0, 1.0, 1.0, 0.08)
                } else {
                    Color::from_rgba(0.0, 0.0, 0.0, 0.06)
                })),
                border: Border::default().rounded(6.0),
                ..button::Style::default()
            }
        }
    })
    .on_press(Message::ToggleMenuPreset);

    let arrow_preset_text = match state.arrow_preset {
        PopoverArrowPreset::MenuWide => "▼ 原生菜单 (21×9)",
        PopoverArrowPreset::TooltipNarrow => "▼ 悬浮提示 (20×7)",
        PopoverArrowPreset::AppKitStandard => "▼ 原生NSPopover (27.5×13)",
        PopoverArrowPreset::SubtleCompact => "▼ 微型提示 (12×5)",
    };

    let arrow_preset_btn = button(
        text(arrow_preset_text)
            .size(10.5)
            .font(font::ui_font(Weight::Medium))
            .color(palette.text_primary),
    )
    .padding(Padding {
        top: 4.0,
        right: 8.0,
        bottom: 4.0,
        left: 8.0,
    })
    .style(move |_theme, _status| button::Style {
        background: Some(Background::Color(if is_dark {
            Color::from_rgba(1.0, 1.0, 1.0, 0.08)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.06)
        })),
        border: Border::default().rounded(6.0),
        ..button::Style::default()
    })
    .on_press(Message::CycleArrowPreset);

    let theme_btn = button(
        text(if is_dark { "☀️ Light" } else { "🌙 Dark" })
            .size(11.0)
            .font(font::ui_font(Weight::Medium))
            .color(palette.text_primary),
    )
    .padding(Padding {
        top: 4.0,
        right: 10.0,
        bottom: 4.0,
        left: 10.0,
    })
    .style(move |_theme, _status| button::Style {
        background: Some(Background::Color(if is_dark {
            Color::from_rgba(1.0, 1.0, 1.0, 0.10)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, 0.06)
        })),
        border: Border::default().rounded(6.0),
        ..button::Style::default()
    })
    .on_press(Message::ToggleColorScheme);

    let header_row = row![
        traffic_lights,
        title_text,
        space().width(Length::Fill),
        preset_btn,
        space().width(6.0),
        arrow_preset_btn,
        space().width(6.0),
        arrow_picker,
        space().width(8.0),
        wallpaper_picker,
        space().width(8.0),
        blur_picker,
        space().width(8.0),
        theme_btn,
        space().width(12.0),
    ]
    .align_y(Alignment::Center)
    .height(Length::Fixed(header_height))
    .width(Length::Fill);

    let header_container = container(header_row)
        .width(Length::Fill)
        .height(Length::Fixed(header_height))
        .style(move |_theme| container::Style {
            background: Some(Background::Color(if is_dark {
                Color::from_rgba(0.10, 0.10, 0.13, 0.72)
            } else {
                Color::from_rgba(0.96, 0.96, 0.98, 0.76)
            })),
            border: Border::default().width(0.5).color(if is_dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.10)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.08)
            }),
            ..container::Style::default()
        });

    state.controller.loyal_drag_bar(
        header_height,
        header_container,
        Message::DragWindow,
        Some(Message::ToggleMaximize),
    )
}

/// Visual and metadata configuration for draggable showcase menu cards.
#[derive(Debug, Clone, Copy)]
pub struct MenuCardConfig {
    pub target: DragTarget,
    pub badge_title: &'static str,
    pub badge_sub: &'static str,
    pub is_dark_card: bool,
    pub card_width: f32,
    pub card_height: f32,
}

/// Builds a draggable menu showcase card with grip bar and transparent background.
#[must_use]
pub fn view_menu_card<'a>(
    config: MenuCardConfig,
    menu: &'a ContextMenu<Message>,
    is_dragging: bool,
    theme: &'a Theme,
    palette: &'a UiPalette,
) -> Element<'a, Message, Theme, iced::Renderer> {
    let grip_pill = row![
        text("⠿")
            .size(14.0)
            .font(font::ui_font(Weight::Bold))
            .color(palette.accent),
        column![
            text(config.badge_title)
                .size(11.0)
                .font(font::ui_font(Weight::Bold))
                .color(palette.text_primary),
            text(config.badge_sub)
                .size(9.5)
                .font(font::ui_font(Weight::Normal))
                .color(palette.text_secondary),
        ]
        .spacing(1.0),
        space().width(Length::Fill),
        text(if is_dragging { "松开" } else { "拖拽" })
            .size(9.5)
            .font(font::ui_font(Weight::Medium))
            .color(if is_dragging { palette.accent } else { palette.text_tertiary }),
    ]
    .spacing(6.0)
    .align_y(Alignment::Center);

    let drag_header = iced::widget::mouse_area(
        container(grip_pill)
            .width(Length::Fixed(config.card_width))
            .height(Length::Fixed(demo_metrics::DRAG_HEADER_HEIGHT))
            .padding(Padding {
                top: 4.0,
                right: 8.0,
                bottom: 4.0,
                left: 8.0,
            })
            .style(move |_theme| container::Style {
                background: Some(Background::Color(if config.is_dark_card {
                    Color::from_rgba(0.12, 0.12, 0.16, 0.90)
                } else {
                    Color::from_rgba(0.96, 0.96, 0.98, 0.92)
                })),
                border: Border::default()
                    .rounded(8.0)
                    .width(1.0)
                    .color(if is_dragging {
                        palette.accent
                    } else if config.is_dark_card {
                        Color::from_rgba(1.0, 1.0, 1.0, 0.22)
                    } else {
                        Color::from_rgba(0.0, 0.0, 0.0, 0.15)
                    }),
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, if is_dragging { 0.35 } else { 0.18 }),
                    offset: Vector::new(0.0, 4.0),
                    blur_radius: 12.0,
                },
                ..container::Style::default()
            }),
    )
    .interaction(if is_dragging {
        mouse::Interaction::Grabbing
    } else {
        mouse::Interaction::Grab
    })
    .on_press(Message::StartDragCard(config.target));

    let menu_element = container(menu.view::<iced::Renderer>(theme))
        .width(Length::Fixed(config.card_width))
        .height(Length::Fixed(config.card_height));

    let card_box = column![drag_header, menu_element]
        .spacing(demo_metrics::HEADER_MENU_GAP)
        .width(Length::Fixed(config.card_width));

    container(card_box).into()
}

/// Builds the bottom status bar.
#[must_use]
pub fn view_status_bar<'a>(
    state: &'a State,
    palette: &'a UiPalette,
    is_dark: bool,
) -> Element<'a, Message, Theme, iced::Renderer> {
    let status_text = state
        .last_action
        .as_deref()
        .unwrap_or("就绪。按住卡片顶部手柄 ⠿ 可拖曳至任意彩条边界，实时观测 Dual-Kawase 柔和空间扩散");

    let blur_radius = state.blur_preset.radius();
    let plan = KawasePassPlan::new(1280, 800, blur_radius);
    let deep_tier = if plan.use_deep_blur {
        "1/2→1/4→1/8"
    } else {
        "1/2→1/4"
    };

    let status_content = row![
        text("STATUS: ")
            .size(11.0)
            .font(font::ui_font(Weight::Bold))
            .color(palette.accent),
        text(status_text)
            .size(11.0)
            .font(font::ui_font(Weight::Normal))
            .color(palette.text_primary),
        space().width(Length::Fill),
        text(format!(
            "vibrancy-rs: Dual-Kawase R={blur_radius:.0}pt ({deep_tier}) · IGN Dither · BT.709 Sat+25% | 白底(深86/浅255) 黑底(深33/浅185)"
        ))
        .size(10.5)
        .font(font::ui_font(Weight::Medium))
        .color(palette.text_secondary),
    ]
    .align_y(Alignment::Center);

    container(status_content)
        .width(Length::Fill)
        .padding(Padding {
            top: 6.0,
            right: 16.0,
            bottom: 6.0,
            left: 16.0,
        })
        .style(move |_theme| container::Style {
            background: Some(Background::Color(if is_dark {
                Color::from_rgba(0.08, 0.08, 0.10, 0.85)
            } else {
                Color::from_rgba(0.96, 0.96, 0.98, 0.90)
            })),
            border: Border::default().width(0.5).color(if is_dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.12)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.10)
            }),
            ..container::Style::default()
        })
        .into()
}
