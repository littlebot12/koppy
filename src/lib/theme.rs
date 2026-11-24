use gpui::WindowAppearance;

pub struct Theme {
    pub dark: bool,
    pub bg: u32,
    pub titlebar: u32,
    pub card: u32,
    pub card_hover: u32,
    pub card_pinned: u32,
    pub border: u32,
    pub border_pinned: u32,
    pub text: u32,
    pub muted: u32,
    pub btn: u32,
    pub btn_hover: u32,
    pub solid: u32,
    pub on_solid: u32,
}

impl Theme {
    fn dark() -> Self {
        Self {
            dark: true,
            bg: 0x0A0A0A,
            titlebar: 0x000000,
            card: 0x141414,
            card_hover: 0x202020,
            card_pinned: 0x1F1F1F,
            border: 0x262626,
            border_pinned: 0xFFFFFF,
            text: 0xFAFAFA,
            muted: 0xA3A3A3,
            btn: 0x1C1C1C,
            btn_hover: 0x2A2A2A,
            solid: 0xFFFFFF,
            on_solid: 0x000000,
        }
    }

    fn light() -> Self {
        Self {
            dark: false,
            bg: 0xFAFAFA,
            titlebar: 0xF4F4F4,
            card: 0xFFFFFF,
            card_hover: 0xF0F0F0,
            card_pinned: 0xEAEAEA,
            border: 0xD4D4D4,
            border_pinned: 0x000000,
            text: 0x0A0A0A,
            muted: 0x525252,
            btn: 0xE5E5E5,
            btn_hover: 0xD4D4D4,
            solid: 0x000000,
            on_solid: 0xFFFFFF,
        }
    }

    pub fn from_appearance(appearance: WindowAppearance) -> Self {
        match appearance {
            WindowAppearance::Dark | WindowAppearance::VibrantDark => Self::dark(),
            WindowAppearance::Light | WindowAppearance::VibrantLight => Self::light(),
        }
    }
}
