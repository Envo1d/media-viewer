use egui::{Context, TextureHandle};
use std::collections::HashMap;
use usvg::Options;

pub struct IconRegistry {
    icons: HashMap<&'static str, TextureHandle>,
}

impl IconRegistry {
    pub fn new(ctx: &Context) -> Self {
        let mut this = Self {
            icons: HashMap::new(),
        };

        this.load(
            ctx,
            "folder",
            include_bytes!("../../assets/icons/folder.svg"),
        );
        this.load(
            ctx,
            "folder_open",
            include_bytes!("../../assets/icons/folder_open.svg"),
        );
        this.load(
            ctx,
            "search",
            include_bytes!("../../assets/icons/search.svg"),
        );
        this.load(
            ctx,
            "lightning",
            include_bytes!("../../assets/icons/lightning.svg"),
        );
        this.load(
            ctx,
            "layers",
            include_bytes!("../../assets/icons/layers.svg"),
        );
        this.load(ctx, "trash", include_bytes!("../../assets/icons/trash.svg"));
        this.load(ctx, "close", include_bytes!("../../assets/icons/close.svg"));
        this.load(
            ctx,
            "maximize",
            include_bytes!("../../assets/icons/maximize.svg"),
        );
        this.load(
            ctx,
            "minimize",
            include_bytes!("../../assets/icons/minimize.svg"),
        );
        this.load(
            ctx,
            "restore",
            include_bytes!("../../assets/icons/restore.svg"),
        );
        this.load(
            ctx,
            "settings",
            include_bytes!("../../assets/icons/settings.svg"),
        );

        this
    }

    fn load(&mut self, ctx: &Context, name: &'static str, bytes: &[u8]) {
        let mut opt = Options::default();

        opt.dpi = 96.0 * ctx.pixels_per_point();

        let image = egui_extras::image::load_svg_bytes(bytes, &opt).expect("Invalid SVG");

        let tex = ctx.load_texture(name, image, Default::default());
        self.icons.insert(name, tex);
    }

    pub fn get(&self, name: &str) -> &TextureHandle {
        self.icons.get(name).expect("Icon not found")
    }
}
