use egui::{Color32, FontData, FontDefinitions, FontFamily, FontId, Margin, Stroke, Visuals};

use crate::telemetry::guidelines::{HealthBadge, MetricLevel};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ThemeId {
    Neo,
    FrutigerAero,
    Kawaii,
    Japandi,
    Minimalist,
    Modern,
    WinLike,
    Terminal,
    Corporate,
}

impl ThemeId {
    pub const ALL: [ThemeId; 9] = [
        ThemeId::Neo,
        ThemeId::FrutigerAero,
        ThemeId::Kawaii,
        ThemeId::Japandi,
        ThemeId::Minimalist,
        ThemeId::Modern,
        ThemeId::WinLike,
        ThemeId::Terminal,
        ThemeId::Corporate,
    ];

    pub fn display_name(self) -> &'static str {
        match self {
            ThemeId::Neo => "Neo",
            ThemeId::FrutigerAero => "Frutiger Aero",
            ThemeId::Kawaii => "Kawaii",
            ThemeId::Japandi => "Japandi",
            ThemeId::Minimalist => "Minimalist",
            ThemeId::Modern => "Modern",
            ThemeId::WinLike => "Win-Like",
            ThemeId::Terminal => "Terminal",
            ThemeId::Corporate => "Corporate",
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            ThemeId::Neo => "neo",
            ThemeId::FrutigerAero => "frutiger-aero",
            ThemeId::Kawaii => "kawaii",
            ThemeId::Japandi => "japandi",
            ThemeId::Minimalist => "minimalist",
            ThemeId::Modern => "modern",
            ThemeId::WinLike => "win-like",
            ThemeId::Terminal => "terminal",
            ThemeId::Corporate => "corporate",
        }
    }

    pub fn from_str_loose(raw: &str) -> Option<Self> {
        let norm: String = raw
            .trim()
            .to_ascii_lowercase()
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect();
        match norm.as_str() {
            "neo" => Some(ThemeId::Neo),
            "frutigeraero" | "aero" | "frutiger" => Some(ThemeId::FrutigerAero),
            "kawaii" | "cute" | "pastel" => Some(ThemeId::Kawaii),
            "japandi" | "japendi" | "zen" => Some(ThemeId::Japandi),
            "minimalist" | "minimal" => Some(ThemeId::Minimalist),
            "modern" => Some(ThemeId::Modern),
            "winlike" | "win" | "windows" | "win11" => Some(ThemeId::WinLike),
            "terminal" | "tui" | "crt" | "retro" => Some(ThemeId::Terminal),
            "corporate" | "enterprise" | "corp" => Some(ThemeId::Corporate),
            _ => None,
        }
    }

    pub fn swatch_colors(self) -> [Color32; 3] {
        let spec = ThemeSpec::for_id(self);
        [spec.bg_card, spec.accent_primary, spec.accent_secondary]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackdropStyle {
    NeoCyber,
    AeroSky,
    KawaiiPastel,
    JapandiPaper,
    CleanFlat,
    ModernAmbient,
    TerminalCrt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderStyle {
    CyberTrapezoid,
    AeroGlassBar,
    KawaiiRibbon,
    TerminalBar,
    ExecutiveBar,
    CleanModern,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GaugeStyle {
    NeonBloom,
    AeroGloss,
    CuteBubble,
    CrispFlat,
    TerminalBlock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarStyle {
    SmoothGlow,
    AeroGlossy,
    CleanFlat,
    TerminalBlocks,
}

#[derive(Debug, Clone)]
pub struct ThemeSpec {
    pub id: ThemeId,
    pub is_dark: bool,

    pub bg_app: Color32,
    pub bg_app_bottom: Color32,
    pub bg_header: Color32,
    pub bg_card: Color32,
    pub bg_inner: Color32,
    pub bg_track: Color32,

    pub border_card: Color32,
    pub border_subtle: Color32,

    pub header_text_primary: Color32,
    pub header_text_secondary: Color32,
    pub header_btn_bg: Color32,
    pub header_btn_border: Color32,

    pub accent_primary: Color32,
    pub accent_secondary: Color32,
    pub accent_tertiary: Color32,

    pub status_healthy: Color32,
    pub status_healthy_bg: Color32,
    pub status_healthy_text: Color32,

    pub status_warm: Color32,
    pub status_warm_bg: Color32,
    pub status_warm_text: Color32,

    pub status_critical: Color32,
    pub status_critical_bg: Color32,
    pub status_critical_text: Color32,

    pub gauge_healthy: (Color32, Color32),
    pub gauge_warm: (Color32, Color32),
    pub gauge_critical: (Color32, Color32),

    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_muted: Color32,

    pub chart_grid: Color32,
    pub chart_temp_secondary: Color32,

    pub card_radius: u8,
    pub inner_radius: u8,
    pub badge_radius: u8,

    pub glow_intensity: f32,
    pub gloss_sheen: bool,
    pub card_shadow: bool,

    pub backdrop_style: BackdropStyle,
    pub header_style: HeaderStyle,
    pub gauge_style: GaugeStyle,
    pub bar_style: BarStyle,
}

impl ThemeSpec {
    pub fn for_id(id: ThemeId) -> Self {
        match id {
            ThemeId::Neo => Self {
                id,
                is_dark: true,
                bg_app: Color32::from_rgb(7, 11, 17),
                bg_app_bottom: Color32::from_rgb(7, 11, 17),
                bg_header: Color32::from_rgba_unmultiplied(12, 18, 27, 235),
                bg_card: Color32::from_rgb(12, 20, 30),
                bg_inner: Color32::from_rgb(8, 14, 22),
                bg_track: Color32::from_rgb(23, 34, 48),
                border_card: Color32::from_rgb(31, 46, 65),
                border_subtle: Color32::from_rgb(22, 34, 50),
                header_text_primary: Color32::from_rgb(241, 245, 249),
                header_text_secondary: Color32::from_rgb(148, 163, 184),
                header_btn_bg: Color32::from_rgb(8, 14, 22),
                header_btn_border: Color32::from_rgb(31, 46, 65),
                accent_primary: Color32::from_rgb(34, 211, 238),
                accent_secondary: Color32::from_rgb(56, 189, 248),
                accent_tertiary: Color32::from_rgb(236, 72, 153),
                status_healthy: Color32::from_rgb(34, 197, 94),
                status_healthy_bg: Color32::from_rgba_unmultiplied(22, 101, 52, 55),
                status_healthy_text: Color32::from_rgb(134, 239, 172),
                status_warm: Color32::from_rgb(245, 158, 11),
                status_warm_bg: Color32::from_rgba_unmultiplied(120, 53, 15, 65),
                status_warm_text: Color32::from_rgb(253, 211, 118),
                status_critical: Color32::from_rgb(239, 68, 68),
                status_critical_bg: Color32::from_rgba_unmultiplied(127, 29, 29, 75),
                status_critical_text: Color32::from_rgb(252, 165, 165),
                gauge_healthy: (
                    Color32::from_rgb(34, 211, 238),
                    Color32::from_rgb(56, 189, 248),
                ),
                gauge_warm: (
                    Color32::from_rgb(245, 158, 11),
                    Color32::from_rgb(251, 146, 60),
                ),
                gauge_critical: (
                    Color32::from_rgb(251, 146, 60),
                    Color32::from_rgb(239, 68, 68),
                ),
                text_primary: Color32::from_rgb(241, 245, 249),
                text_secondary: Color32::from_rgb(148, 163, 184),
                text_muted: Color32::from_rgb(100, 116, 139),
                chart_grid: Color32::from_rgba_unmultiplied(31, 46, 65, 140),
                chart_temp_secondary: Color32::from_rgba_unmultiplied(251, 146, 60, 175),
                card_radius: 12,
                inner_radius: 10,
                badge_radius: 8,
                glow_intensity: 1.0,
                gloss_sheen: false,
                card_shadow: false,
                backdrop_style: BackdropStyle::NeoCyber,
                header_style: HeaderStyle::CyberTrapezoid,
                gauge_style: GaugeStyle::NeonBloom,
                bar_style: BarStyle::SmoothGlow,
            },

            ThemeId::FrutigerAero => Self {
                id,
                is_dark: false,
                bg_app: Color32::from_rgb(12, 82, 146),
                bg_app_bottom: Color32::from_rgb(18, 152, 168),
                bg_header: Color32::from_rgba_unmultiplied(244, 250, 255, 232),
                bg_card: Color32::from_rgba_unmultiplied(246, 251, 255, 234),
                bg_inner: Color32::from_rgba_unmultiplied(233, 245, 254, 190),
                bg_track: Color32::from_rgb(204, 226, 242),
                border_card: Color32::from_rgba_unmultiplied(24, 92, 148, 85),
                border_subtle: Color32::from_rgba_unmultiplied(136, 186, 222, 130),
                header_text_primary: Color32::from_rgb(8, 30, 52),
                header_text_secondary: Color32::from_rgb(30, 68, 102),
                header_btn_bg: Color32::from_rgba_unmultiplied(255, 255, 255, 215),
                header_btn_border: Color32::from_rgba_unmultiplied(116, 174, 216, 165),
                accent_primary: Color32::from_rgb(3, 105, 161),
                accent_secondary: Color32::from_rgb(4, 120, 87),
                accent_tertiary: Color32::from_rgb(13, 148, 136),
                status_healthy: Color32::from_rgb(5, 122, 85),
                status_healthy_bg: Color32::from_rgb(13, 138, 85),
                status_healthy_text: Color32::WHITE,
                status_warm: Color32::from_rgb(180, 83, 9),
                status_warm_bg: Color32::from_rgb(194, 103, 6),
                status_warm_text: Color32::WHITE,
                status_critical: Color32::from_rgb(185, 28, 28),
                status_critical_bg: Color32::from_rgb(198, 40, 40),
                status_critical_text: Color32::WHITE,
                gauge_healthy: (
                    Color32::from_rgb(2, 132, 199),
                    Color32::from_rgb(13, 168, 128),
                ),
                gauge_warm: (
                    Color32::from_rgb(217, 119, 6),
                    Color32::from_rgb(234, 88, 12),
                ),
                gauge_critical: (
                    Color32::from_rgb(220, 38, 38),
                    Color32::from_rgb(185, 28, 28),
                ),
                text_primary: Color32::from_rgb(8, 30, 52),
                text_secondary: Color32::from_rgb(30, 68, 102),
                text_muted: Color32::from_rgb(51, 94, 130),
                chart_grid: Color32::from_rgba_unmultiplied(142, 190, 224, 125),
                chart_temp_secondary: Color32::from_rgba_unmultiplied(217, 95, 14, 215),
                card_radius: 14,
                inner_radius: 10,
                badge_radius: 10,
                glow_intensity: 0.25,
                gloss_sheen: true,
                card_shadow: true,
                backdrop_style: BackdropStyle::AeroSky,
                header_style: HeaderStyle::AeroGlassBar,
                gauge_style: GaugeStyle::AeroGloss,
                bar_style: BarStyle::AeroGlossy,
            },

            ThemeId::Kawaii => Self {
                id,
                is_dark: false,
                bg_app: Color32::from_rgb(255, 240, 247),
                bg_app_bottom: Color32::from_rgb(244, 235, 255),
                bg_header: Color32::from_rgba_unmultiplied(255, 255, 255, 235),
                bg_card: Color32::from_rgb(255, 255, 255),
                bg_inner: Color32::from_rgb(255, 246, 251),
                bg_track: Color32::from_rgb(249, 226, 238),
                border_card: Color32::from_rgb(246, 192, 220),
                border_subtle: Color32::from_rgb(249, 214, 233),
                header_text_primary: Color32::from_rgb(74, 29, 61),
                header_text_secondary: Color32::from_rgb(138, 72, 115),
                header_btn_bg: Color32::from_rgb(255, 245, 250),
                header_btn_border: Color32::from_rgb(243, 182, 214),
                accent_primary: Color32::from_rgb(244, 82, 146),
                accent_secondary: Color32::from_rgb(168, 114, 247),
                accent_tertiary: Color32::from_rgb(52, 211, 153),
                status_healthy: Color32::from_rgb(16, 185, 129),
                status_healthy_bg: Color32::from_rgb(220, 252, 238),
                status_healthy_text: Color32::from_rgb(6, 95, 70),
                status_warm: Color32::from_rgb(245, 142, 48),
                status_warm_bg: Color32::from_rgb(255, 241, 214),
                status_warm_text: Color32::from_rgb(154, 52, 18),
                status_critical: Color32::from_rgb(225, 29, 72),
                status_critical_bg: Color32::from_rgb(255, 228, 234),
                status_critical_text: Color32::from_rgb(159, 18, 57),
                gauge_healthy: (
                    Color32::from_rgb(255, 107, 169),
                    Color32::from_rgb(181, 123, 255),
                ),
                gauge_warm: (
                    Color32::from_rgb(251, 176, 59),
                    Color32::from_rgb(251, 113, 133),
                ),
                gauge_critical: (
                    Color32::from_rgb(244, 63, 94),
                    Color32::from_rgb(225, 29, 72),
                ),
                text_primary: Color32::from_rgb(74, 29, 61),
                text_secondary: Color32::from_rgb(125, 74, 107),
                text_muted: Color32::from_rgb(164, 114, 146),
                chart_grid: Color32::from_rgba_unmultiplied(238, 198, 220, 140),
                chart_temp_secondary: Color32::from_rgba_unmultiplied(168, 85, 247, 165),
                card_radius: 20,
                inner_radius: 16,
                badge_radius: 14,
                glow_intensity: 0.35,
                gloss_sheen: false,
                card_shadow: true,
                backdrop_style: BackdropStyle::KawaiiPastel,
                header_style: HeaderStyle::KawaiiRibbon,
                gauge_style: GaugeStyle::CuteBubble,
                bar_style: BarStyle::SmoothGlow,
            },

            ThemeId::Japandi => Self {
                id,
                is_dark: false,
                bg_app: Color32::from_rgb(243, 238, 229),
                bg_app_bottom: Color32::from_rgb(243, 238, 229),
                bg_header: Color32::from_rgb(250, 247, 242),
                bg_card: Color32::from_rgb(250, 247, 242),
                bg_inner: Color32::from_rgb(239, 233, 222),
                bg_track: Color32::from_rgb(224, 216, 201),
                border_card: Color32::from_rgb(214, 204, 187),
                border_subtle: Color32::from_rgb(225, 217, 202),
                header_text_primary: Color32::from_rgb(43, 38, 33),
                header_text_secondary: Color32::from_rgb(98, 88, 77),
                header_btn_bg: Color32::from_rgb(239, 233, 222),
                header_btn_border: Color32::from_rgb(206, 194, 174),
                accent_primary: Color32::from_rgb(79, 111, 88),
                accent_secondary: Color32::from_rgb(112, 138, 118),
                accent_tertiary: Color32::from_rgb(184, 115, 75),
                status_healthy: Color32::from_rgb(79, 111, 88),
                status_healthy_bg: Color32::from_rgb(227, 235, 227),
                status_healthy_text: Color32::from_rgb(46, 72, 54),
                status_warm: Color32::from_rgb(184, 115, 75),
                status_warm_bg: Color32::from_rgb(246, 230, 216),
                status_warm_text: Color32::from_rgb(138, 74, 38),
                status_critical: Color32::from_rgb(168, 62, 50),
                status_critical_bg: Color32::from_rgb(245, 220, 216),
                status_critical_text: Color32::from_rgb(128, 39, 30),
                gauge_healthy: (
                    Color32::from_rgb(79, 111, 88),
                    Color32::from_rgb(106, 142, 115),
                ),
                gauge_warm: (
                    Color32::from_rgb(196, 128, 72),
                    Color32::from_rgb(184, 96, 62),
                ),
                gauge_critical: (
                    Color32::from_rgb(184, 96, 62),
                    Color32::from_rgb(168, 62, 50),
                ),
                text_primary: Color32::from_rgb(43, 38, 33),
                text_secondary: Color32::from_rgb(98, 88, 77),
                text_muted: Color32::from_rgb(142, 130, 116),
                chart_grid: Color32::from_rgba_unmultiplied(208, 198, 180, 145),
                chart_temp_secondary: Color32::from_rgba_unmultiplied(184, 115, 75, 190),
                card_radius: 8,
                inner_radius: 6,
                badge_radius: 6,
                glow_intensity: 0.0,
                gloss_sheen: false,
                card_shadow: false,
                backdrop_style: BackdropStyle::JapandiPaper,
                header_style: HeaderStyle::CleanModern,
                gauge_style: GaugeStyle::CrispFlat,
                bar_style: BarStyle::CleanFlat,
            },

            ThemeId::Minimalist => Self {
                id,
                is_dark: false,
                bg_app: Color32::from_rgb(248, 249, 250),
                bg_app_bottom: Color32::from_rgb(248, 249, 250),
                bg_header: Color32::from_rgb(255, 255, 255),
                bg_card: Color32::from_rgb(255, 255, 255),
                bg_inner: Color32::from_rgb(248, 250, 252),
                bg_track: Color32::from_rgb(226, 232, 240),
                border_card: Color32::from_rgb(226, 232, 240),
                border_subtle: Color32::from_rgb(236, 240, 244),
                header_text_primary: Color32::from_rgb(15, 23, 42),
                header_text_secondary: Color32::from_rgb(82, 82, 91),
                header_btn_bg: Color32::from_rgb(248, 250, 252),
                header_btn_border: Color32::from_rgb(212, 212, 216),
                accent_primary: Color32::from_rgb(24, 24, 27),
                accent_secondary: Color32::from_rgb(82, 82, 91),
                accent_tertiary: Color32::from_rgb(113, 113, 122),
                status_healthy: Color32::from_rgb(22, 163, 74),
                status_healthy_bg: Color32::from_rgb(240, 253, 244),
                status_healthy_text: Color32::from_rgb(21, 128, 61),
                status_warm: Color32::from_rgb(217, 119, 6),
                status_warm_bg: Color32::from_rgb(255, 251, 235),
                status_warm_text: Color32::from_rgb(180, 83, 9),
                status_critical: Color32::from_rgb(220, 38, 38),
                status_critical_bg: Color32::from_rgb(254, 242, 242),
                status_critical_text: Color32::from_rgb(185, 28, 28),
                gauge_healthy: (
                    Color32::from_rgb(24, 24, 27),
                    Color32::from_rgb(63, 63, 70),
                ),
                gauge_warm: (
                    Color32::from_rgb(217, 119, 6),
                    Color32::from_rgb(234, 88, 12),
                ),
                gauge_critical: (
                    Color32::from_rgb(239, 68, 68),
                    Color32::from_rgb(185, 28, 28),
                ),
                text_primary: Color32::from_rgb(9, 9, 11),
                text_secondary: Color32::from_rgb(82, 82, 91),
                text_muted: Color32::from_rgb(113, 113, 122),
                chart_grid: Color32::from_rgba_unmultiplied(226, 232, 240, 180),
                chart_temp_secondary: Color32::from_rgba_unmultiplied(217, 119, 6, 175),
                card_radius: 4,
                inner_radius: 3,
                badge_radius: 3,
                glow_intensity: 0.0,
                gloss_sheen: false,
                card_shadow: true,
                backdrop_style: BackdropStyle::CleanFlat,
                header_style: HeaderStyle::CleanModern,
                gauge_style: GaugeStyle::CrispFlat,
                bar_style: BarStyle::CleanFlat,
            },

            ThemeId::Modern => Self {
                id,
                is_dark: true,
                bg_app: Color32::from_rgb(10, 11, 15),
                bg_app_bottom: Color32::from_rgb(10, 11, 15),
                bg_header: Color32::from_rgb(17, 19, 27),
                bg_card: Color32::from_rgb(19, 21, 28),
                bg_inner: Color32::from_rgb(13, 15, 21),
                bg_track: Color32::from_rgb(31, 35, 48),
                border_card: Color32::from_rgb(39, 43, 58),
                border_subtle: Color32::from_rgb(30, 34, 46),
                header_text_primary: Color32::from_rgb(248, 250, 252),
                header_text_secondary: Color32::from_rgb(148, 163, 184),
                header_btn_bg: Color32::from_rgb(26, 29, 39),
                header_btn_border: Color32::from_rgb(48, 53, 71),
                accent_primary: Color32::from_rgb(99, 102, 241),
                accent_secondary: Color32::from_rgb(56, 189, 248),
                accent_tertiary: Color32::from_rgb(168, 85, 247),
                status_healthy: Color32::from_rgb(34, 197, 94),
                status_healthy_bg: Color32::from_rgba_unmultiplied(22, 101, 52, 50),
                status_healthy_text: Color32::from_rgb(134, 239, 172),
                status_warm: Color32::from_rgb(245, 158, 11),
                status_warm_bg: Color32::from_rgba_unmultiplied(120, 53, 15, 55),
                status_warm_text: Color32::from_rgb(253, 224, 71),
                status_critical: Color32::from_rgb(244, 63, 94),
                status_critical_bg: Color32::from_rgba_unmultiplied(136, 19, 55, 65),
                status_critical_text: Color32::from_rgb(253, 164, 175),
                gauge_healthy: (
                    Color32::from_rgb(99, 102, 241),
                    Color32::from_rgb(56, 189, 248),
                ),
                gauge_warm: (
                    Color32::from_rgb(245, 158, 11),
                    Color32::from_rgb(251, 146, 60),
                ),
                gauge_critical: (
                    Color32::from_rgb(251, 113, 133),
                    Color32::from_rgb(244, 63, 94),
                ),
                text_primary: Color32::from_rgb(248, 250, 252),
                text_secondary: Color32::from_rgb(148, 163, 184),
                text_muted: Color32::from_rgb(100, 116, 139),
                chart_grid: Color32::from_rgba_unmultiplied(40, 45, 62, 150),
                chart_temp_secondary: Color32::from_rgba_unmultiplied(251, 146, 60, 175),
                card_radius: 12,
                inner_radius: 9,
                badge_radius: 8,
                glow_intensity: 0.32,
                gloss_sheen: false,
                card_shadow: false,
                backdrop_style: BackdropStyle::ModernAmbient,
                header_style: HeaderStyle::CleanModern,
                gauge_style: GaugeStyle::NeonBloom,
                bar_style: BarStyle::SmoothGlow,
            },

            ThemeId::WinLike => Self {
                id,
                is_dark: true,
                bg_app: Color32::from_rgb(28, 28, 28),
                bg_app_bottom: Color32::from_rgb(28, 28, 28),
                bg_header: Color32::from_rgb(39, 39, 39),
                bg_card: Color32::from_rgb(43, 43, 43),
                bg_inner: Color32::from_rgb(35, 35, 35),
                bg_track: Color32::from_rgb(58, 58, 58),
                border_card: Color32::from_rgb(63, 63, 63),
                border_subtle: Color32::from_rgb(52, 52, 52),
                header_text_primary: Color32::from_rgb(255, 255, 255),
                header_text_secondary: Color32::from_rgb(204, 204, 204),
                header_btn_bg: Color32::from_rgb(50, 50, 50),
                header_btn_border: Color32::from_rgb(72, 72, 72),
                accent_primary: Color32::from_rgb(96, 205, 255),
                accent_secondary: Color32::from_rgb(76, 194, 255),
                accent_tertiary: Color32::from_rgb(108, 203, 95),
                status_healthy: Color32::from_rgb(108, 203, 95),
                status_healthy_bg: Color32::from_rgba_unmultiplied(45, 92, 40, 120),
                status_healthy_text: Color32::from_rgb(154, 230, 144),
                status_warm: Color32::from_rgb(252, 225, 0),
                status_warm_bg: Color32::from_rgba_unmultiplied(105, 86, 12, 125),
                status_warm_text: Color32::from_rgb(253, 232, 102),
                status_critical: Color32::from_rgb(255, 153, 164),
                status_critical_bg: Color32::from_rgba_unmultiplied(120, 34, 42, 130),
                status_critical_text: Color32::from_rgb(255, 196, 202),
                gauge_healthy: (
                    Color32::from_rgb(76, 194, 255),
                    Color32::from_rgb(96, 205, 255),
                ),
                gauge_warm: (
                    Color32::from_rgb(252, 225, 0),
                    Color32::from_rgb(245, 158, 11),
                ),
                gauge_critical: (
                    Color32::from_rgb(255, 153, 164),
                    Color32::from_rgb(239, 68, 68),
                ),
                text_primary: Color32::from_rgb(255, 255, 255),
                text_secondary: Color32::from_rgb(204, 204, 204),
                text_muted: Color32::from_rgb(154, 154, 154),
                chart_grid: Color32::from_rgba_unmultiplied(68, 68, 68, 170),
                chart_temp_secondary: Color32::from_rgba_unmultiplied(252, 225, 0, 175),
                card_radius: 6,
                inner_radius: 4,
                badge_radius: 4,
                glow_intensity: 0.0,
                gloss_sheen: false,
                card_shadow: false,
                backdrop_style: BackdropStyle::CleanFlat,
                header_style: HeaderStyle::CleanModern,
                gauge_style: GaugeStyle::CrispFlat,
                bar_style: BarStyle::CleanFlat,
            },

            ThemeId::Terminal => Self {
                id,
                is_dark: true,
                bg_app: Color32::from_rgb(4, 8, 4),
                bg_app_bottom: Color32::from_rgb(4, 8, 4),
                bg_header: Color32::from_rgb(7, 16, 7),
                bg_card: Color32::from_rgb(7, 15, 7),
                bg_inner: Color32::from_rgb(4, 10, 4),
                bg_track: Color32::from_rgb(13, 34, 16),
                border_card: Color32::from_rgb(29, 110, 46),
                border_subtle: Color32::from_rgb(20, 74, 32),
                header_text_primary: Color32::from_rgb(51, 255, 102),
                header_text_secondary: Color32::from_rgb(36, 198, 76),
                header_btn_bg: Color32::from_rgb(6, 14, 6),
                header_btn_border: Color32::from_rgb(29, 110, 46),
                accent_primary: Color32::from_rgb(51, 255, 102),
                accent_secondary: Color32::from_rgb(34, 211, 102),
                accent_tertiary: Color32::from_rgb(255, 176, 0),
                status_healthy: Color32::from_rgb(51, 255, 102),
                status_healthy_bg: Color32::from_rgb(8, 34, 14),
                status_healthy_text: Color32::from_rgb(51, 255, 102),
                status_warm: Color32::from_rgb(255, 176, 0),
                status_warm_bg: Color32::from_rgb(38, 26, 4),
                status_warm_text: Color32::from_rgb(255, 198, 64),
                status_critical: Color32::from_rgb(255, 51, 51),
                status_critical_bg: Color32::from_rgb(42, 8, 8),
                status_critical_text: Color32::from_rgb(255, 110, 110),
                gauge_healthy: (
                    Color32::from_rgb(34, 211, 90),
                    Color32::from_rgb(51, 255, 102),
                ),
                gauge_warm: (
                    Color32::from_rgb(245, 158, 11),
                    Color32::from_rgb(255, 176, 0),
                ),
                gauge_critical: (
                    Color32::from_rgb(255, 92, 51),
                    Color32::from_rgb(255, 51, 51),
                ),
                text_primary: Color32::from_rgb(168, 255, 186),
                text_secondary: Color32::from_rgb(92, 214, 120),
                text_muted: Color32::from_rgb(52, 145, 74),
                chart_grid: Color32::from_rgba_unmultiplied(24, 92, 38, 150),
                chart_temp_secondary: Color32::from_rgba_unmultiplied(255, 176, 0, 190),
                card_radius: 0,
                inner_radius: 0,
                badge_radius: 0,
                glow_intensity: 0.25,
                gloss_sheen: false,
                card_shadow: false,
                backdrop_style: BackdropStyle::TerminalCrt,
                header_style: HeaderStyle::TerminalBar,
                gauge_style: GaugeStyle::TerminalBlock,
                bar_style: BarStyle::TerminalBlocks,
            },

            ThemeId::Corporate => Self {
                id,
                is_dark: false,
                bg_app: Color32::from_rgb(237, 241, 245),
                bg_app_bottom: Color32::from_rgb(237, 241, 245),
                bg_header: Color32::from_rgb(15, 39, 68),
                bg_card: Color32::from_rgb(255, 255, 255),
                bg_inner: Color32::from_rgb(248, 250, 252),
                bg_track: Color32::from_rgb(218, 226, 236),
                border_card: Color32::from_rgb(203, 213, 225),
                border_subtle: Color32::from_rgb(226, 232, 240),
                header_text_primary: Color32::from_rgb(255, 255, 255),
                header_text_secondary: Color32::from_rgb(203, 213, 225),
                header_btn_bg: Color32::from_rgb(26, 54, 93),
                header_btn_border: Color32::from_rgb(55, 89, 134),
                accent_primary: Color32::from_rgb(29, 78, 216),
                accent_secondary: Color32::from_rgb(2, 132, 199),
                accent_tertiary: Color32::from_rgb(212, 166, 62),
                status_healthy: Color32::from_rgb(21, 128, 61),
                status_healthy_bg: Color32::from_rgb(236, 253, 245),
                status_healthy_text: Color32::from_rgb(22, 101, 52),
                status_warm: Color32::from_rgb(180, 83, 9),
                status_warm_bg: Color32::from_rgb(255, 247, 237),
                status_warm_text: Color32::from_rgb(154, 52, 18),
                status_critical: Color32::from_rgb(185, 28, 28),
                status_critical_bg: Color32::from_rgb(254, 242, 242),
                status_critical_text: Color32::from_rgb(153, 27, 27),
                gauge_healthy: (
                    Color32::from_rgb(29, 78, 216),
                    Color32::from_rgb(2, 132, 199),
                ),
                gauge_warm: (
                    Color32::from_rgb(217, 119, 6),
                    Color32::from_rgb(234, 88, 12),
                ),
                gauge_critical: (
                    Color32::from_rgb(220, 38, 38),
                    Color32::from_rgb(185, 28, 28),
                ),
                text_primary: Color32::from_rgb(15, 23, 42),
                text_secondary: Color32::from_rgb(71, 85, 105),
                text_muted: Color32::from_rgb(100, 116, 139),
                chart_grid: Color32::from_rgba_unmultiplied(203, 213, 225, 175),
                chart_temp_secondary: Color32::from_rgba_unmultiplied(180, 83, 9, 175),
                card_radius: 6,
                inner_radius: 4,
                badge_radius: 4,
                glow_intensity: 0.0,
                gloss_sheen: false,
                card_shadow: true,
                backdrop_style: BackdropStyle::CleanFlat,
                header_style: HeaderStyle::ExecutiveBar,
                gauge_style: GaugeStyle::CrispFlat,
                bar_style: BarStyle::CleanFlat,
            },
        }
    }

    pub fn font_title(&self, size: f32) -> FontId {
        font_title(size)
    }

    pub fn font_title_semi(&self, size: f32) -> FontId {
        font_title_semi(size)
    }

    pub fn font_body(&self, size: f32) -> FontId {
        font_body(size)
    }

    pub fn font_ui(&self, size: f32) -> FontId {
        font_ui(size)
    }

    pub fn font_value(&self, size: f32) -> FontId {
        font_value(size)
    }

    pub fn font_mono(&self, size: f32) -> FontId {
        font_mono(size)
    }

    pub fn font_mono_bold(&self, size: f32) -> FontId {
        font_mono_bold(size)
    }

    pub fn temp_colors(&self, level: MetricLevel) -> (Color32, Color32) {
        match level {
            MetricLevel::Healthy => self.gauge_healthy,
            MetricLevel::Warm => self.gauge_warm,
            MetricLevel::Critical => self.gauge_critical,
        }
    }

    pub fn badge_colors(&self, badge: HealthBadge) -> (Color32, Color32, Color32) {
        match badge {
            HealthBadge::Healthy => (
                self.status_healthy,
                self.status_healthy_bg,
                self.status_healthy_text,
            ),
            HealthBadge::Warm => (self.status_warm, self.status_warm_bg, self.status_warm_text),
            HealthBadge::Critical | HealthBadge::Throttling => (
                self.status_critical,
                self.status_critical_bg,
                self.status_critical_text,
            ),
        }
    }
}

pub fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    fonts.font_data.insert(
        "inter_regular".to_owned(),
        FontData::from_static(include_bytes!("../../assets/fonts/Inter-Regular.ttf")).into(),
    );
    fonts.font_data.insert(
        "inter_medium".to_owned(),
        FontData::from_static(include_bytes!("../../assets/fonts/Inter-Medium.ttf")).into(),
    );
    fonts.font_data.insert(
        "inter_semibold".to_owned(),
        FontData::from_static(include_bytes!("../../assets/fonts/Inter-SemiBold.ttf")).into(),
    );
    fonts.font_data.insert(
        "inter_bold".to_owned(),
        FontData::from_static(include_bytes!("../../assets/fonts/Inter-Bold.ttf")).into(),
    );
    fonts.font_data.insert(
        "jb_mono".to_owned(),
        FontData::from_static(include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf"))
            .into(),
    );
    fonts.font_data.insert(
        "jb_mono_bold".to_owned(),
        FontData::from_static(include_bytes!("../../assets/fonts/JetBrainsMono-Bold.ttf")).into(),
    );

    if let Some(prop) = fonts.families.get_mut(&FontFamily::Proportional) {
        prop.insert(0, "inter_regular".to_owned());
    }
    if let Some(mono) = fonts.families.get_mut(&FontFamily::Monospace) {
        mono.insert(0, "jb_mono".to_owned());
        mono.insert(1, "inter_regular".to_owned());
    }

    fonts.families.insert(
        FontFamily::Name("ui_medium".into()),
        vec!["inter_medium".to_owned(), "inter_regular".to_owned()],
    );
    fonts.families.insert(
        FontFamily::Name("ui_semibold".into()),
        vec![
            "inter_semibold".to_owned(),
            "inter_medium".to_owned(),
            "inter_regular".to_owned(),
        ],
    );
    fonts.families.insert(
        FontFamily::Name("title_semi".into()),
        vec![
            "inter_semibold".to_owned(),
            "inter_bold".to_owned(),
            "inter_regular".to_owned(),
        ],
    );
    fonts.families.insert(
        FontFamily::Name("title".into()),
        vec![
            "inter_bold".to_owned(),
            "inter_semibold".to_owned(),
            "inter_regular".to_owned(),
        ],
    );
    fonts.families.insert(
        FontFamily::Name("mono_bold".into()),
        vec![
            "jb_mono_bold".to_owned(),
            "jb_mono".to_owned(),
            "inter_semibold".to_owned(),
        ],
    );

    ctx.set_fonts(fonts);
}

pub fn apply_theme_visuals(ctx: &egui::Context, theme: &ThemeSpec) {
    let mut visuals = if theme.is_dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };

    visuals.panel_fill = theme.bg_app;
    visuals.window_fill = theme.bg_card;
    visuals.extreme_bg_color = theme.bg_inner;
    visuals.faint_bg_color = theme.bg_inner;
    visuals.window_stroke = Stroke::new(1.0_f32, theme.border_card);
    visuals.override_text_color = Some(theme.text_primary);

    visuals.widgets.noninteractive.bg_fill = theme.bg_card;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, theme.text_primary);
    visuals.widgets.inactive.bg_fill = theme.bg_track;
    visuals.widgets.inactive.weak_bg_fill = theme.bg_inner;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, theme.text_secondary);

    let hover_bg = if theme.is_dark {
        lerp_color(theme.bg_track, Color32::WHITE, 0.08)
    } else {
        lerp_color(theme.bg_track, Color32::BLACK, 0.04)
    };
    visuals.widgets.hovered.bg_fill = hover_bg;
    visuals.widgets.hovered.weak_bg_fill = hover_bg;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, theme.text_primary);

    visuals.widgets.active.bg_fill = with_alpha(theme.accent_primary, 75);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, theme.text_primary);

    visuals.selection.bg_fill = with_alpha(theme.accent_primary, 55);
    visuals.selection.stroke = Stroke::new(1.0_f32, theme.accent_primary);

    ctx.set_visuals(visuals);

    ctx.style_mut(|s| {
        s.spacing.window_margin = Margin::same(14);
        s.spacing.item_spacing = egui::vec2(10.0, 8.0);
    });
}

pub fn font_title(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("title".into()))
}

pub fn font_title_semi(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("title_semi".into()))
}

pub fn font_body(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

pub fn font_ui(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("ui_medium".into()))
}

pub fn font_value(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("ui_semibold".into()))
}

pub fn font_mono(size: f32) -> FontId {
    FontId::new(size, FontFamily::Monospace)
}

pub fn font_mono_bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("mono_bold".into()))
}

pub fn with_alpha(c: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), alpha)
}

pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let r = a.r() as f32 + (b.r() as f32 - a.r() as f32) * t;
    let g = a.g() as f32 + (b.g() as f32 - a.g() as f32) * t;
    let bl = a.b() as f32 + (b.b() as f32 - a.b() as f32) * t;
    let al = a.a() as f32 + (b.a() as f32 - a.a() as f32) * t;
    Color32::from_rgba_unmultiplied(
        r.round() as u8,
        g.round() as u8,
        bl.round() as u8,
        al.round() as u8,
    )
}

pub fn display_temp(temp_c: f32, use_fahrenheit: bool) -> f32 {
    if use_fahrenheit {
        temp_c * 9.0 / 5.0 + 32.0
    } else {
        temp_c
    }
}

pub fn temp_unit(use_fahrenheit: bool) -> &'static str {
    if use_fahrenheit {
        "°F"
    } else {
        "°C"
    }
}

pub fn format_temp(temp_c: f32, use_fahrenheit: bool) -> String {
    format!(
        "{:.0}{}",
        display_temp(temp_c, use_fahrenheit),
        temp_unit(use_fahrenheit)
    )
}
