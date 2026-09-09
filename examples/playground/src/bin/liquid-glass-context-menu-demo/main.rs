//! Standalone showcase for authentic Apple-style Liquid Glass Context Menus.
//!
//! Fully powered by `bmol-window-shell`:
//! - Complete frameless window shell integration with physical non-client rim
//! - Authentic macOS traffic lights with dynamic symmetric margins (`margin_left == margin_top`)
//! - Interactive 8-direction border resize handles and loyal titlebar drag bar
//! - Native macOS squircle corner clipping and Stage Manager guard
//! - Dual Appearance: Side-by-side demonstration of both Light and Dark physical glass menus
//! - Ultra-Heavy Backdrop Blur (64pt Dual-Kawase equivalent) completely dissolving high-frequency details
//! - Continuous 12.0 pt squircle container curvature matching Apple HIG Liquid Glass specifications
//! - 1px fine edge highlight rim with deep elevation drop shadows (36pt blur)
//! - Interactive right-click popup at cursor location with outside-click dismissal

#![allow(
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::struct_excessive_bools,
    clippy::many_single_char_names,
    clippy::excessive_precision,
    clippy::unreadable_literal,
    clippy::doc_markdown
)]

pub mod menu_content;
pub mod popover_render;
pub mod state;
pub mod vibrancy;
pub mod views;

pub use menu_content::*;
pub use popover_render::*;
pub use state::*;
pub use vibrancy::*;
pub use views::*;

use bmol_window_shell::window_metrics;
use iced::{
    Alignment, Background, Border, Color, Element, Length, Padding, Rectangle, Size, Theme,
    font::Weight,
    widget::{
        button,
        canvas::Canvas,
        column, container, row, space, stack, text,
    },
    window,
};
use liquid_glass::{UiColorScheme, ui::font};

#[must_use]
pub fn view(state: &State) -> Element<'_, Message, Theme, iced::Renderer> {
    let is_dark = state.controller.is_dark;
    let theme = &state.theme;
    let palette = &state.palette;

    // 1. Top fused header bar
    let draggable_header = view_top_header(state, palette, is_dark);

    // 2. Draggable Light & Dark menu cards over television color test blocks
    let is_light_dragging = matches!(state.active_drag, Some((DragTarget::LightCard, _)));
    let is_dark_dragging = matches!(state.active_drag, Some((DragTarget::DarkCard, _)));

    let (card_w, card_h) = state.menu_preset.menu_size();
    let (light_title, light_sub) = match state.menu_preset {
        MenuContentPreset::DockReference1To1 => ("☀️ 1:1 Dock 菜单 (浅色)", "参考截图 1:1 还原: 154×117 pt"),
        MenuContentPreset::FullShowcase => ("☀️ Light Mode Menu", "校准值: 白255 | 黑185 (α=72.5%)"),
    };
    let (dark_title, dark_sub) = match state.menu_preset {
        MenuContentPreset::DockReference1To1 => ("🌙 1:1 Dock 菜单 (深色)", "参考截图 1:1 还原: 154×117 pt"),
        MenuContentPreset::FullShowcase => ("🌙 Dark Mode Menu", "校准值: 白86 | 黑33 (α=79.2%)"),
    };

    let light_card = view_menu_card(
        MenuCardConfig {
            target: DragTarget::LightCard,
            badge_title: light_title,
            badge_sub: light_sub,
            is_dark_card: false,
            card_width: card_w,
            card_height: card_h,
        },
        &state.light_menu,
        is_light_dragging,
        theme,
        palette,
    );

    let dark_card = view_menu_card(
        MenuCardConfig {
            target: DragTarget::DarkCard,
            badge_title: dark_title,
            badge_sub: dark_sub,
            is_dark_card: true,
            card_width: card_w,
            card_height: card_h,
        },
        &state.dark_menu,
        is_dark_dragging,
        theme,
        palette,
    );

    let positioned_light = container(light_card)
        .padding(Padding {
            top: state.light_pos.y.max(10.0),
            left: state.light_pos.x.max(10.0),
            right: 0.0,
            bottom: 0.0,
        })
        .width(Length::Fill)
        .height(Length::Fill);

    let positioned_dark = container(dark_card)
        .padding(Padding {
            top: state.dark_pos.y.max(10.0),
            left: state.dark_pos.x.max(10.0),
            right: 0.0,
            bottom: 0.0,
        })
        .width(Length::Fill)
        .height(Length::Fill);

    let cards_stack = if state.top_card == DragTarget::LightCard {
        stack![positioned_dark, positioned_light]
    } else {
        stack![positioned_light, positioned_dark]
    }
    .width(Length::Fill)
    .height(Length::Fill);

    let header_height = state.controller.metrics.header_rect.height;
    let guide_height = 36.0;
    let top_offset = header_height + guide_height;

    let arrow_cfg = state.current_arrow_config();

    let card_corner_radius = state.menu_preset.corner_radius();

    let mut occlusions = vec![
        MenuOcclusion {
            bounds: Rectangle {
                x: state.light_pos.x.max(10.0),
                y: state.light_pos.y.max(10.0) + demo_metrics::MENU_Y_INSET,
                width: card_w,
                height: card_h,
            },
            corner_radius: card_corner_radius,
            arrow: arrow_cfg,
            is_dark: false,
        },
        MenuOcclusion {
            bounds: Rectangle {
                x: state.dark_pos.x.max(10.0),
                y: state.dark_pos.y.max(10.0) + demo_metrics::MENU_Y_INSET,
                width: card_w,
                height: card_h,
            },
            corner_radius: card_corner_radius,
            arrow: arrow_cfg,
            is_dark: true,
        },
    ];

    if let Some(pos) = state.floating_menu {
        occlusions.push(MenuOcclusion {
            bounds: Rectangle {
                x: (pos.x - 10.0).max(10.0),
                y: (pos.y - 10.0 - top_offset).max(0.0),
                width: card_w,
                height: card_h,
            },
            corner_radius: card_corner_radius,
            arrow: arrow_cfg,
            is_dark: match state.resolved_floating_scheme() {
                UiColorScheme::Dark => true,
                UiColorScheme::Light => false,
            },
        });
    }

    let wallpaper_canvas = Canvas::new(WallpaperCanvas {
        style: state.wallpaper,
        blur_preset: state.blur_preset,
        occlusions,
        show_calibration_grid: state.show_calibration_grid,
    })
    .width(Length::Fill)
    .height(Length::Fill);

    let wallpaper_layer = container(wallpaper_canvas)
        .width(Length::Fill)
        .height(Length::Fill);

    let stage_stack = stack![wallpaper_layer, cards_stack]
        .width(Length::Fill)
        .height(Length::Fill);

    // Calibration guide and reset toolbar
    let guide_bar = row![
        text("📺 电视彩色色块校准台:")
            .size(11.5)
            .font(font::ui_font(Weight::Bold))
            .color(palette.accent),
        text("按住菜单卡片顶栏 ⠿ 即可自由拖动，移至各色彩条/色块上方取色校验")
            .size(11.0)
            .font(font::ui_font(Weight::Normal))
            .color(palette.text_primary),
        space().width(Length::Fill),
        button(
            text(if state.show_calibration_grid {
                "📐 1px测试网格: 开启"
            } else {
                "📐 1px测试网格: 关闭"
            })
            .size(10.5)
            .font(font::ui_font(Weight::Medium))
            .color(palette.text_primary),
        )
        .padding(Padding {
            top: 3.0,
            right: 8.0,
            bottom: 3.0,
            left: 8.0,
        })
        .style(move |_theme, _status| button::Style {
            background: Some(Background::Color(if is_dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.10)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.06)
            })),
            border: Border::default().rounded(5.0),
            ..button::Style::default()
        })
        .on_press(Message::ToggleCalibrationGrid),
        button(
            text("↺ 重置位置")
                .size(10.5)
                .font(font::ui_font(Weight::Medium))
                .color(palette.text_primary)
        )
        .padding(Padding {
            top: 3.0,
            right: 8.0,
            bottom: 3.0,
            left: 8.0,
        })
        .style(move |_theme, _status| button::Style {
            background: Some(Background::Color(if is_dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.10)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.06)
            })),
            border: Border::default().rounded(5.0),
            ..button::Style::default()
        })
        .on_press(Message::ResetCardPositions),
    ]
    .spacing(8.0)
    .align_y(Alignment::Center)
    .padding(Padding {
        top: 4.0,
        right: 14.0,
        bottom: 4.0,
        left: 14.0,
    });

    let guide_container = container(guide_bar)
        .width(Length::Fill)
        .style(move |_theme| container::Style {
            background: Some(Background::Color(if is_dark {
                Color::from_rgba(0.10, 0.10, 0.14, 0.85)
            } else {
                Color::from_rgba(0.96, 0.96, 0.98, 0.85)
            })),
            border: Border::default().width(0.5).color(if is_dark {
                Color::from_rgba(1.0, 1.0, 1.0, 0.12)
            } else {
                Color::from_rgba(0.0, 0.0, 0.0, 0.10)
            }),
            ..container::Style::default()
        });

    let page_content = if state.show_status_bar {
        column![
            draggable_header,
            guide_container,
            stage_stack,
            view_status_bar(state, palette, is_dark)
        ]
    } else {
        column![draggable_header, guide_container, stage_stack]
    }
    .width(Length::Fill)
    .height(Length::Fill);

    let mut layers: Vec<Element<'_, Message, Theme, iced::Renderer>> = vec![page_content.into()];

    // 3. Optional floating context menu at cursor
    if let Some(pos) = state.floating_menu {
        let dismiss_backdrop = iced::widget::mouse_area(
            container(space())
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .on_press(Message::DismissFloatingMenu);

        let floating_menu_widget = state.floating_menu_cached.view::<iced::Renderer>(theme);

        let positioned_menu = container(
            container(floating_menu_widget)
                .width(Length::Fixed(demo_metrics::MENU_WIDTH))
                .height(Length::Fixed(demo_metrics::MENU_HEIGHT)),
        )
        .padding(Padding {
            top: (pos.y - 10.0).max(10.0),
            left: (pos.x - 10.0).max(10.0),
            right: 0.0,
            bottom: 0.0,
        })
        .width(Length::Fill)
        .height(Length::Fill);

        let overlay_stack = stack![dismiss_backdrop, positioned_menu]
            .width(Length::Fill)
            .height(Length::Fill);

        layers.push(overlay_stack.into());
    }

    let root_page = container(iced::widget::Stack::with_children(layers))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_theme| container::Style {
            background: Some(Background::Color(if is_dark {
                Color::from_rgb8(18, 18, 22)
            } else {
                Color::from_rgb8(248, 249, 251)
            })),
            border: Border {
                radius: window_metrics::DEFAULT_CORNER_RADIUS.into(),
                ..Default::default()
            },
            ..Default::default()
        });

    // 4. Wrap with bmol-window-shell 8-direction resize handles and physical non-client rim
    state.controller.wrap_window_with_resizer(
        root_page,
        window_metrics::DEFAULT_CORNER_RADIUS,
        Message::ResizeWindow,
    )
}

fn main() -> iced::Result {
    let fonts = font::ui_fonts();
    let window_settings = window::Settings {
        size: Size::new(1180.0, 780.0),
        transparent: true,
        decorations: false,
        ..Default::default()
    };

    let mut app = iced::application::<State, Message, Theme, iced::Renderer>(boot, update, view)
        .title("Liquid Glass Context Menu - Window Shell Edition")
        .theme(app_theme)
        .subscription(subscription)
        .window(window_settings);

    for bytes in &fonts.bytes {
        app = app.font(bytes.clone());
    }
    if let Some(ui_font) = fonts.font() {
        app = app.default_font(ui_font);
    }
    app.run()
}

#[cfg(test)]
#[allow(
    clippy::similar_names,
    clippy::float_cmp,
    clippy::uninlined_format_args,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::field_reassign_with_default,
    clippy::type_complexity
)]
mod tests {
    use super::*;
    use bmol_designs::{
        menu_metrics,
        popover_metrics::{PopoverArrowConfig, PopoverArrowEdge, PopoverArrowPreset},
    };
    use liquid_glass::{
        MenuItem,
        geometry::{squircle_path_commands, SquircleParams, APPLE_CORNER_SMOOTHING},
    };
    use vibrancy_rs::KawasePassPlan;
    use iced::{Point, Size, window};

    #[test]
    fn test_initial_state_defaults() {
        let state = State::default();
        assert_eq!(state.wallpaper, WallpaperStyle::TvColorBars);
        assert_eq!(state.blur_preset, BlurPreset::UltraHeavy64);
        assert_eq!(state.floating_appearance, FloatingAppearance::FollowTheme);
        assert!(state.show_status_bar);
        assert!(state.show_line_numbers);
        assert!(!state.word_wrap);
        assert_eq!(state.floating_menu, None);
        assert_eq!(state.controller.window_id, None);
        assert_eq!(state.light_pos, Point::new(70.0, 50.0));
        assert_eq!(state.dark_pos, Point::new(450.0, 50.0));
        assert_eq!(state.active_drag, None);
        assert_eq!(state.menu_preset, MenuContentPreset::DockReference1To1);
        assert_eq!(state.popover_arrow_edge, PopoverArrowEdge::Bottom);
        assert_eq!(state.arrow_preset, PopoverArrowPreset::MenuWide);
        assert_eq!(state.arrow_offset, demo_metrics::DOCK_REF_ARROW_OFFSET);
    }

    #[test]
    fn test_window_shell_controller_lifecycle() {
        let mut state = State::default();
        let test_id = window::Id::unique();

        // 1. WindowOpened associates window id
        let _ = update(&mut state, Message::WindowOpened(test_id));
        assert_eq!(state.controller.window_id, Some(test_id));

        // 2. WindowResized updates controller metrics
        let _ = update(&mut state, Message::WindowResized(Size::new(1200.0, 800.0)));
        assert_eq!(state.controller.window_size, (1200.0, 800.0));
        assert_eq!(state.controller.metrics.window_size, (1200.0, 800.0));

        // 3. Traffic lights hover interaction
        assert_eq!(state.controller.traffic_lights.hover_target, 0.0);
        let _ = update(
            &mut state,
            Message::TrafficLights(bmol_window_shell::TrafficLightsEvent::GroupHover(true)),
        );
        assert_eq!(state.controller.traffic_lights.hover_target, 1.0);
        let _ = update(
            &mut state,
            Message::TrafficLights(bmol_window_shell::TrafficLightsEvent::GroupHover(false)),
        );
        assert_eq!(state.controller.traffic_lights.hover_target, 0.0);

        // 4. Focus/Unfocus handling
        let _ = update(
            &mut state,
            Message::WindowEvent((test_id, window::Event::Unfocused)),
        );
        assert!(!state.controller.is_focused);

        let _ = update(
            &mut state,
            Message::WindowEvent((test_id, window::Event::Focused)),
        );
        assert!(state.controller.is_focused);
    }

    #[test]
    fn test_toggle_actions_update_state() {
        let mut state = State::default();

        let _ = update(&mut state, Message::TriggerAction(MenuAction::ToggleLineNumbers));
        assert!(!state.show_line_numbers);

        let _ = update(&mut state, Message::TriggerAction(MenuAction::ToggleWordWrap));
        assert!(state.word_wrap);

        let _ = update(&mut state, Message::TriggerAction(MenuAction::ToggleStatusBar));
        assert!(!state.show_status_bar);
    }

    #[test]
    fn test_floating_menu_lifecycle() {
        let mut state = State::default();
        let target = Point::new(200.0, 150.0);

        let _ = update(&mut state, Message::OpenFloatingMenuAt(target));
        assert_eq!(state.floating_menu, Some(target));

        let _ = update(&mut state, Message::DismissFloatingMenu);
        assert_eq!(state.floating_menu, None);

        let _ = update(&mut state, Message::CursorMoved(Point::new(350.0, 400.0)));
        let _ = update(&mut state, Message::RightClicked);
        assert_eq!(state.floating_menu, Some(Point::new(350.0, 400.0)));
    }

    #[test]
    fn test_theme_and_wallpaper_switching() {
        let mut state = State::default();
        let orig_theme = state.controller.is_dark;

        let _ = update(&mut state, Message::ToggleColorScheme);
        assert_ne!(state.controller.is_dark, orig_theme);

        let _ = update(&mut state, Message::SetWallpaper(WallpaperStyle::TvSmpteSplit));
        assert_eq!(state.wallpaper, WallpaperStyle::TvSmpteSplit);

        let _ = update(&mut state, Message::SetBlurPreset(BlurPreset::Heavy48));
        assert_eq!(state.blur_preset, BlurPreset::Heavy48);
        assert_eq!(state.blur_preset.radius(), 48.0);
    }

    #[test]
    fn test_card_dragging_lifecycle() {
        let mut state = State::default();
        state.cursor_pos = Point::new(100.0, 80.0);

        // 1. Start dragging light card
        let _ = update(&mut state, Message::StartDragCard(DragTarget::LightCard));
        assert!(state.active_drag.is_some());
        assert_eq!(state.top_card, DragTarget::LightCard);

        // 2. Cursor moved updates position
        let _ = update(&mut state, Message::CursorMoved(Point::new(160.0, 140.0)));
        assert_eq!(state.light_pos, Point::new(130.0, 110.0));

        // 3. End drag fixes card in place
        let _ = update(&mut state, Message::EndDragCard);
        assert!(state.active_drag.is_none());
        assert_eq!(state.light_pos, Point::new(130.0, 110.0));

        // 4. Reset positions restores defaults
        let _ = update(&mut state, Message::ResetCardPositions);
        assert_eq!(state.light_pos, Point::new(70.0, 50.0));
        assert_eq!(state.dark_pos, Point::new(450.0, 50.0));
    }

    #[test]
    fn test_demo_menu_building_and_rendering() {
        let mut state = State::default();
        state.rebuild_menus();
        let menu_dock = build_demo_menu(&state);
        assert_eq!(menu_dock.len(), 5);

        let theme_dark = app_theme(&state);
        let elem_dark = menu_dock.view::<iced::Renderer>(&theme_dark);
        drop(elem_dark);

        // Switch to full showcase and verify rendering
        let _ = update(&mut state, Message::ToggleMenuPreset);
        state.rebuild_menus();
        let menu_full = build_demo_menu(&state);
        assert_eq!(menu_full.len(), 22);

        let mut light_state = State::default();
        light_state.controller.set_dark_mode(false);
        let _ = update(&mut light_state, Message::ToggleMenuPreset);
        light_state.rebuild_menus();
        let theme_light = app_theme(&light_state);
        let elem_light = menu_full.view::<iced::Renderer>(&theme_light);
        drop(elem_light);
    }

    #[test]
    fn test_measured_colorimetry_expectations() {
        let (dark_r, _, _, dark_a) = menu_metrics::DARK_MENU_BASE_RGBA_F32;
        let (light_r, _, _, light_a) = menu_metrics::LIGHT_MENU_BASE_RGBA_F32;

        // Dark on white: 86
        let dark_on_white = (dark_r * 255.0 * dark_a + 255.0 * (1.0 - dark_a)).round() as u8;
        assert_eq!(dark_on_white, 86);

        // Dark on black: 33
        let dark_on_black = (dark_r * 255.0 * dark_a).round() as u8;
        assert_eq!(dark_on_black, 33);

        // Light on white: 255
        let light_on_white = (light_r * 255.0 * light_a + 255.0 * (1.0 - light_a)).round() as u8;
        assert_eq!(light_on_white, 255);

        // Light on black: 185
        let light_on_black = (light_r * 255.0 * light_a).round() as u8;
        assert_eq!(light_on_black, 185);
    }

    #[test]
    fn test_tv_color_bars_multi_color_calibration() {
        let (dark_r, _, _, dark_a) = menu_metrics::DARK_MENU_BASE_RGBA_F32;
        let (light_r, _, _, light_a) = menu_metrics::LIGHT_MENU_BASE_RGBA_F32;

        // 8 TV Color Bars: (Name, [R, G, B])
        let tv_bars: &[(&str, [u8; 3], [u8; 3], [u8; 3])] = &[
            // (Bar, Background RGB, Expected Dark Mode RGB, Expected Light Mode RGB)
            ("Pure White", [255, 255, 255], [86, 86, 86], [255, 255, 255]),
            ("Yellow",     [255, 255,   0], [86, 86, 33], [255, 255, 185]),
            ("Cyan",       [  0, 255, 255], [33, 86, 86], [185, 255, 255]),
            ("Green",      [  0, 255,   0], [33, 86, 33], [185, 255, 185]),
            ("Magenta",    [255,   0, 255], [86, 33, 86], [255, 185, 255]),
            ("Red",        [255,   0,   0], [86, 33, 33], [255, 185, 185]),
            ("Blue",       [  0,   0, 255], [33, 33, 86], [185, 185, 255]),
            ("Pure Black", [  0,   0,   0], [33, 33, 33], [185, 185, 185]),
        ];

        for &(name, bg, exp_dark, exp_light) in tv_bars {
            let calc_dark_r = (dark_r * 255.0 * dark_a + f32::from(bg[0]) * (1.0 - dark_a)).round() as u8;
            let calc_dark_g = (dark_r * 255.0 * dark_a + f32::from(bg[1]) * (1.0 - dark_a)).round() as u8;
            let calc_dark_b = (dark_r * 255.0 * dark_a + f32::from(bg[2]) * (1.0 - dark_a)).round() as u8;
            assert_eq!([calc_dark_r, calc_dark_g, calc_dark_b], exp_dark, "Dark mode on {}", name);

            let calc_light_r = (light_r * 255.0 * light_a + f32::from(bg[0]) * (1.0 - light_a)).round() as u8;
            let calc_light_g = (light_r * 255.0 * light_a + f32::from(bg[1]) * (1.0 - light_a)).round() as u8;
            let calc_light_b = (light_r * 255.0 * light_a + f32::from(bg[2]) * (1.0 - light_a)).round() as u8;
            assert_eq!([calc_light_r, calc_light_g, calc_light_b], exp_light, "Light mode on {}", name);
        }
    }

    #[test]
    fn test_vibrancy_dual_kawase_sampling_and_grid_toggle() {
        let mut state = State::default();
        assert!(state.show_calibration_grid);

        // 1. Toggle high-frequency calibration grid
        let _ = update(&mut state, Message::ToggleCalibrationGrid);
        assert!(!state.show_calibration_grid);
        let _ = update(&mut state, Message::ToggleCalibrationGrid);
        assert!(state.show_calibration_grid);

        // 2. Vibrancy PassPlan metrics
        let blur_64 = BlurPreset::UltraHeavy64.radius();
        let plan_64 = KawasePassPlan::new(1280, 800, blur_64);
        assert!(plan_64.use_deep_blur, "64pt must trigger 1/8 deep blur tier");
        assert!(plan_64.offset >= 1.2 && plan_64.offset <= 2.5);

        // 3. Pure color consistency verification
        let white_color = sample_analytical_blurred_wallpaper(
            WallpaperStyle::PureWhite,
            100.0,
            100.0,
            Size::new(800.0, 600.0),
            blur_64,
        );
        assert_eq!(white_color, Color::WHITE);

        let black_color = sample_analytical_blurred_wallpaper(
            WallpaperStyle::PureBlack,
            100.0,
            100.0,
            Size::new(800.0, 600.0),
            blur_64,
        );
        assert_eq!(black_color, Color::BLACK);

        // 4. Color bar boundary dispersion (yellow x=100.0 to cyan x=200.0)
        // Bar 1 is yellow [1, 1, 0], Bar 2 is cyan [0, 1, 1] on an 800-wide viewport (100px per bar)
        let boundary_sample = sample_analytical_blurred_wallpaper(
            WallpaperStyle::TvColorBars,
            200.0, // Exactly at Yellow/Cyan boundary
            300.0,
            Size::new(800.0, 600.0),
            blur_64,
        );
        // At boundary, both red and blue channels are non-zero due to continuous Gaussian dispersion
        assert!(boundary_sample.r > 0.05, "Yellow's red channel dispersed across boundary");
        assert!(boundary_sample.b > 0.05, "Cyan's blue channel dispersed across boundary");
        assert!(boundary_sample.g > 0.8, "Green channel remains high for both yellow and cyan");
    }

    #[test]
    fn test_demo_menu_height_geometry_exactness() {
        // 1. Verify Dock 1:1 Reference Preset
        let state = State::default();
        let menu_dock = build_demo_menu(&state);
        let items_dock = menu_dock.items_slice();
        assert_eq!(items_dock.len(), 5, "Dock 1:1 reference menu must have exactly 5 items");

        let mut dock_actions = 0;
        let mut dock_separators = 0;
        for item in items_dock {
            match item {
                MenuItem::Separator => dock_separators += 1,
                MenuItem::Action { .. } | MenuItem::Submenu { .. } => dock_actions += 1,
                _ => {}
            }
        }
        assert_eq!(dock_actions, 4);
        assert_eq!(dock_separators, 1);

        let dock_items_height = dock_actions as f32 * menu_metrics::ITEM_HEIGHT;
        let dock_separators_height = dock_separators as f32 * (menu_metrics::SEPARATOR_HEIGHT + menu_metrics::SEPARATOR_MARGIN_V * 2.0);
        let dock_gaps = (items_dock.len() - 1) as f32 * 1.0;
        let dock_padding = menu_metrics::CONTAINER_PADDING * 2.0;
        let dock_total = dock_items_height + dock_separators_height + dock_gaps + dock_padding;
        assert_eq!(dock_total, demo_metrics::DOCK_REF_HEIGHT);

        // 2. Verify Full Showcase Preset
        let mut full_state = State::default();
        let _ = update(&mut full_state, Message::ToggleMenuPreset);
        assert_eq!(full_state.menu_preset, MenuContentPreset::FullShowcase);

        let menu_full = build_demo_menu(&full_state);
        let items_full = menu_full.items_slice();
        assert_eq!(items_full.len(), 22, "Full showcase menu must have exactly 22 items");

        let mut section_count = 0;
        let mut separator_count = 0;
        let mut action_count = 0;

        for item in items_full {
            match item {
                MenuItem::Section(_) => section_count += 1,
                MenuItem::Separator => separator_count += 1,
                MenuItem::Action { .. } | MenuItem::Checkbox { .. } | MenuItem::Submenu { .. } => {
                    action_count += 1;
                }
            }
        }

        assert_eq!(section_count, 5);
        assert_eq!(separator_count, 4);
        assert_eq!(action_count, 13);

        let expected_sections = section_count as f32 * menu_metrics::SECTION_HEADER_HEIGHT;
        let expected_items = action_count as f32 * menu_metrics::ITEM_HEIGHT;
        let expected_separators = separator_count as f32 * (menu_metrics::SEPARATOR_HEIGHT + menu_metrics::SEPARATOR_MARGIN_V * 2.0);
        let expected_gaps = (items_full.len() - 1) as f32 * 1.0;
        let container_padding = menu_metrics::CONTAINER_PADDING * 2.0;
        let total_exact = expected_sections + expected_items + expected_separators + expected_gaps + container_padding;

        assert_eq!(
            total_exact,
            demo_metrics::MENU_HEIGHT,
            "demo_metrics::MENU_HEIGHT must exactly equal total item stack height to avoid transparent occlusion bottom gaps"
        );
        assert_eq!(demo_metrics::MENU_HEIGHT, 477.0);
        assert_eq!(demo_metrics::MENU_Y_INSET, 44.0);
    }

    #[test]
    fn test_toggle_menu_preset() {
        let mut state = State::default();
        assert_eq!(state.menu_preset, MenuContentPreset::DockReference1To1);
        assert_eq!(state.popover_arrow_edge, PopoverArrowEdge::Bottom);
        assert_eq!(state.arrow_offset, demo_metrics::DOCK_REF_ARROW_OFFSET);

        // 1. Toggle to FullShowcase
        let _ = update(&mut state, Message::ToggleMenuPreset);
        assert_eq!(state.menu_preset, MenuContentPreset::FullShowcase);
        assert_eq!(state.arrow_offset, 0.5);

        // 2. Toggle back to DockReference1To1
        let _ = update(&mut state, Message::ToggleMenuPreset);
        assert_eq!(state.menu_preset, MenuContentPreset::DockReference1To1);
        assert_eq!(state.popover_arrow_edge, PopoverArrowEdge::Bottom);
        assert_eq!(state.arrow_offset, demo_metrics::DOCK_REF_ARROW_OFFSET);
    }

    #[test]
    fn test_cycle_arrow_preset_lifecycle() {
        let mut state = State::default();
        assert_eq!(state.arrow_preset, PopoverArrowPreset::MenuWide);

        // 1. MenuWide -> TooltipNarrow
        let _ = update(&mut state, Message::CycleArrowPreset);
        assert_eq!(state.arrow_preset, PopoverArrowPreset::TooltipNarrow);
        let cfg_narrow = state.current_arrow_config();
        assert_eq!(cfg_narrow.base_width, 20.0);
        assert_eq!(cfg_narrow.height, 7.0);
        assert_eq!(cfg_narrow.tip_radius, 1.0);

        // 2. TooltipNarrow -> AppKitStandard
        let _ = update(&mut state, Message::CycleArrowPreset);
        assert_eq!(state.arrow_preset, PopoverArrowPreset::AppKitStandard);
        let cfg_std = state.current_arrow_config();
        assert_eq!(cfg_std.base_width, 27.5);
        assert_eq!(cfg_std.height, 13.0);
        assert_eq!(cfg_std.tip_radius, 2.0);

        // 3. AppKitStandard -> SubtleCompact
        let _ = update(&mut state, Message::CycleArrowPreset);
        assert_eq!(state.arrow_preset, PopoverArrowPreset::SubtleCompact);
        let cfg_subtle = state.current_arrow_config();
        assert_eq!(cfg_subtle.base_width, 12.0);
        assert_eq!(cfg_subtle.height, 5.0);
        assert_eq!(cfg_subtle.tip_radius, 1.0);

        // 4. SubtleCompact -> MenuWide
        let _ = update(&mut state, Message::CycleArrowPreset);
        assert_eq!(state.arrow_preset, PopoverArrowPreset::MenuWide);
        let cfg_wide = state.current_arrow_config();
        assert_eq!(cfg_wide.base_width, 21.0);
        assert_eq!(cfg_wide.height, 9.0);
        assert_eq!(cfg_wide.tip_radius, 1.8);
    }

    #[test]
    fn test_squircle_continuous_curvature_integration() {
        let rect = Rectangle {
            x: 50.0,
            y: 50.0,
            width: demo_metrics::MENU_WIDTH,
            height: demo_metrics::MENU_HEIGHT,
        };
        let r = demo_metrics::MENU_CORNER_RADIUS;

        // 1. Verify SquircleParams generates non-empty Apple continuous commands
        let params = SquircleParams::new(rect.width, rect.height, r)
            .with_smoothing(APPLE_CORNER_SMOOTHING);
        let commands = squircle_path_commands(&params);
        assert!(!commands.is_empty(), "Squircle commands must not be empty");

        // 2. Verify Apple smoothing exponent matches tagged library default (3.32)
        let exp_n = 2.0 + (4.2 - 2.0) * APPLE_CORNER_SMOOTHING;
        assert!((exp_n - 3.32).abs() < 1e-4);

        // 3. Verify corner curvature inset smoothly vanishes towards center
        let inv_exp_n = 1.0 / exp_n;
        let r_pow_n = r.powf(exp_n);

        // At corner tip (dx = r)
        let dy_tip = (r_pow_n - r.powf(exp_n)).max(0.0).powf(inv_exp_n);
        let inset_tip = r - dy_tip;
        assert!((inset_tip - r).abs() < 1e-4, "Corner tip inset must equal full radius");

        // Midway through corner (dx = r * 0.5)
        let dx_mid = r * 0.5;
        let dy_mid = (r_pow_n - dx_mid.powf(exp_n)).max(0.0).powf(inv_exp_n);
        let inset_mid = r - dy_mid;
        // In G2 squircle, inset_mid is significantly smoother than circle
        assert!(inset_mid > 0.0 && inset_mid < r * 0.5);
    }

    #[test]
    fn test_set_popover_arrow_message_handling() {
        let mut state = State::default();
        assert_eq!(state.popover_arrow_edge, PopoverArrowEdge::Bottom);

        let edges = [
            (PopoverArrowEdge::Top, "▲ 顶部触角 (Top)"),
            (PopoverArrowEdge::Left, "◀ 左侧触角 (Left)"),
            (PopoverArrowEdge::Right, "▶ 右侧触角 (Right)"),
            (PopoverArrowEdge::None, "无触角"),
            (PopoverArrowEdge::Bottom, "▼ 底部触角 (Bottom)"),
        ];

        for (edge, expected_text) in edges {
            let _ = update(&mut state, Message::SetPopoverArrow(edge));
            assert_eq!(state.popover_arrow_edge, edge);
            assert!(
                state.last_action.as_ref().unwrap().contains(expected_text),
                "Status should describe {:?}",
                edge
            );
        }
    }

    #[test]
    fn test_popover_arrow_path_geometry() {
        let rect = Rectangle {
            x: 100.0,
            y: 100.0,
            width: demo_metrics::MENU_WIDTH,
            height: demo_metrics::MENU_HEIGHT,
        };
        let r = demo_metrics::MENU_CORNER_RADIUS;

        let all_edges = [
            PopoverArrowEdge::None,
            PopoverArrowEdge::Top,
            PopoverArrowEdge::Bottom,
            PopoverArrowEdge::Left,
            PopoverArrowEdge::Right,
        ];

        for edge in all_edges {
            let config = PopoverArrowConfig::new(edge);
            if edge == PopoverArrowEdge::None {
                assert!(!config.is_visible());
            } else {
                assert!(config.is_visible());
            }

            let path = build_popover_squircle_path(rect, r, config);
            drop(path);

            // Verify with modified shadow spread size
            let shadow_config = config.with_size(config.base_width + 10.0, config.height + 5.0);
            assert_eq!(shadow_config.base_width, config.base_width + 10.0);
            assert_eq!(shadow_config.height, config.height + 5.0);
            let shadow_path = build_popover_squircle_path(rect, r + 5.0, shadow_config);
            drop(shadow_path);
        }
    }
}
