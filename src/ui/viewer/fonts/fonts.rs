/// Названия шрифтовых гарнитур
#[derive(Clone, Copy)]
pub enum FontFace {
    Regular,
    Medium,
    Bold,
    Italic,
}

impl FontFace {
    pub const fn key(self) -> &'static str {
        match self {
            FontFace::Regular => "JBMono-Regular",
            FontFace::Medium => "JBMono-Medium",
            FontFace::Bold => "JBMono-Bold",
            FontFace::Italic => "JBMono-Italic",
        }
    }

    pub const fn cjk_key(self) -> &'static str {
        match self {
            FontFace::Regular => "LXGW-Regular",
            FontFace::Medium => "LXGW-Medium", 
            FontFace::Bold => "LXGW-Bold",
            FontFace::Italic => "LXGW-Italic",
        }
    }

    pub const fn cjk_tc_key(self) -> &'static str {
        match self {
            FontFace::Regular => "LXGW-TC-Regular",
            FontFace::Medium => "LXGW-TC-Medium",
            FontFace::Bold => "LXGW-TC-Bold", 
            FontFace::Italic => "LXGW-TC-Italic",
        }
    }
}

/// Вшитые TTF-байты
pub const FONT_BYTES: &[(FontFace, &[u8])] = &[
    (FontFace::Regular, include_bytes!("ttf/JetBrainsMono-Regular.ttf")), //
    (FontFace::Medium, include_bytes!("ttf/JetBrainsMono-Medium.ttf")),
    (FontFace::Bold, include_bytes!("ttf/JetBrainsMono-Bold.ttf")),
    (FontFace::Italic, include_bytes!("ttf/JetBrainsMono-Italic.ttf")),
];

/// Вшитые CJK TTF-байты
pub const CJK_FONT_BYTES: &[(FontFace, &[u8])] = &[
    (FontFace::Regular, include_bytes!("ttf/LXGWWenKaiMono-Regular.ttf")),
    (FontFace::Medium, include_bytes!("ttf/LXGWWenKaiMono-Medium.ttf")),
    (FontFace::Bold, include_bytes!("ttf/LXGWWenKaiMono-Medium.ttf")), // Bold резерв
    (FontFace::Italic, include_bytes!("ttf/LXGWWenKaiMono-Light.ttf")), // Light как Italic
];

/// Вшитые CJK TC TTF-байты
pub const CJK_TC_FONT_BYTES: &[(FontFace, &[u8])] = &[
    (FontFace::Regular, include_bytes!("ttf/LXGWWenKaiMonoTC-Regular.ttf")),
    (FontFace::Medium, include_bytes!("ttf/LXGWWenKaiMonoTC-Medium.ttf")),
    (FontFace::Bold, include_bytes!("ttf/LXGWWenKaiMonoTC-Medium.ttf")), // Bold резерв
    (FontFace::Italic, include_bytes!("ttf/LXGWWenKaiMonoTC-Light.ttf")), // Light как Italic
];
