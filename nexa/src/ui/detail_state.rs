use crate::core::models::{FileDetailInfo, MediaItem, StagingItem};
use egui::TextureHandle;
use std::sync::Arc;

pub struct LibraryDetailState {
    pub selected_item: Option<Arc<MediaItem>>,
    pub preview_texture: Option<TextureHandle>,
    pub preview_rx: Option<crossbeam_channel::Receiver<Option<image::RgbaImage>>>,
    pub info: Option<FileDetailInfo>,
    pub info_rx: Option<crossbeam_channel::Receiver<FileDetailInfo>>,
    pub selected_path: String,
}

impl LibraryDetailState {
    pub fn new() -> Self {
        Self {
            selected_item: None,
            preview_texture: None,
            preview_rx: None,
            info: None,
            info_rx: None,
            selected_path: String::new(),
        }
    }

    pub fn reset_async(&mut self) {
        self.preview_texture = None;
        self.preview_rx = None;
        self.info = None;
        self.info_rx = None;
        self.selected_path = String::new();
    }

    pub fn is_selected(&self, path: &str) -> bool {
        self.selected_item
            .as_ref()
            .map(|i| i.path == path)
            .unwrap_or(false)
    }
}

pub struct StagingDetailState {
    pub selected_item: Option<Arc<StagingItem>>,
    pub preview_texture: Option<TextureHandle>,
    pub preview_rx: Option<crossbeam_channel::Receiver<Option<image::RgbaImage>>>,
    pub info: Option<FileDetailInfo>,
    pub info_rx: Option<crossbeam_channel::Receiver<FileDetailInfo>>,
    pub selected_path: String,
}

impl StagingDetailState {
    pub fn new() -> Self {
        Self {
            selected_item: None,
            preview_texture: None,
            preview_rx: None,
            info: None,
            info_rx: None,
            selected_path: String::new(),
        }
    }

    pub fn reset_async(&mut self) {
        self.preview_texture = None;
        self.preview_rx = None;
        self.info = None;
        self.info_rx = None;
        self.selected_path = String::new();
    }

    pub fn is_selected(&self, path: &str) -> bool {
        self.selected_item
            .as_ref()
            .map(|i| i.path == path)
            .unwrap_or(false)
    }
}
