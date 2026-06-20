use ratatui::style::Color;

pub mod catppuccin_mocha {
    use super::Color;
    #[allow(dead_code)]
    pub const BASE: Color = Color::Rgb(30, 30, 46);
    pub const TEXT: Color = Color::Rgb(205, 214, 244);
    pub const SUBTEXT0: Color = Color::Rgb(166, 173, 200);
    pub const OVERLAY0: Color = Color::Rgb(108, 112, 134);
    #[allow(dead_code)]
    pub const SURFACE0: Color = Color::Rgb(49, 50, 68);
    pub const SURFACE1: Color = Color::Rgb(69, 71, 90);

    pub const MAUVE: Color = Color::Rgb(203, 166, 247);
    pub const RED: Color = Color::Rgb(243, 139, 168);
    pub const PEACH: Color = Color::Rgb(250, 179, 135);
    pub const YELLOW: Color = Color::Rgb(249, 226, 175);
    pub const GREEN: Color = Color::Rgb(166, 227, 161);
    pub const TEAL: Color = Color::Rgb(148, 226, 213);
    #[allow(dead_code)]
    pub const SAPPHIRE: Color = Color::Rgb(116, 199, 236);
    #[allow(dead_code)]
    pub const BLUE: Color = Color::Rgb(137, 180, 250);
    #[allow(dead_code)]
    pub const LAVENDER: Color = Color::Rgb(180, 190, 254);
}

pub mod tokyo_night {
    use super::Color;
    #[allow(dead_code)]
    pub const BASE: Color = Color::Rgb(26, 27, 38);
    pub const TEXT: Color = Color::Rgb(192, 202, 245);
    pub const SUBTEXT0: Color = Color::Rgb(169, 177, 214);
    pub const OVERLAY0: Color = Color::Rgb(154, 165, 206);
    #[allow(dead_code)]
    pub const SURFACE0: Color = Color::Rgb(36, 40, 59);
    pub const SURFACE1: Color = Color::Rgb(65, 72, 104);

    pub const MAUVE: Color = Color::Rgb(187, 154, 247);
    pub const RED: Color = Color::Rgb(247, 118, 142);
    pub const PEACH: Color = Color::Rgb(255, 158, 100);
    pub const YELLOW: Color = Color::Rgb(224, 175, 104);
    pub const GREEN: Color = Color::Rgb(158, 206, 106);
    pub const TEAL: Color = Color::Rgb(42, 195, 222);
    #[allow(dead_code)]
    pub const SAPPHIRE: Color = Color::Rgb(122, 162, 247);
    #[allow(dead_code)]
    pub const BLUE: Color = Color::Rgb(122, 162, 247);
    #[allow(dead_code)]
    pub const LAVENDER: Color = Color::Rgb(180, 249, 248);
}

// Set active theme
pub use tokyo_night as active_theme;
