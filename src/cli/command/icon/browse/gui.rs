use crate::hicon::hicon_to_rgba;
use crate::string::EasyPCWSTR;
use eframe::egui;
use egui_tiles::{TileId, Tiles};
use eyre::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use windows::Win32::UI::Shell::ExtractIconExW;
use windows::Win32::UI::WindowsAndMessaging::HICON;
use windows::Win32::UI::WindowsAndMessaging::PrivateExtractIconsW;

pub fn run_icon_browser(paths: Vec<PathBuf>) -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([900.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Icon Browser",
        options,
        Box::new(|cc| Ok(Box::new(IconBrowserApp::new(cc, paths)))),
    )
    .map_err(|e| eyre::eyre!("Failed to run eframe: {}", e))
}

#[derive(Debug, Clone)]
pub struct IconEntry {
    pub dll_path: PathBuf,
    pub index: u32,
}

#[derive(Debug, Clone)]
pub struct LoadedIconInfo {
    pub texture_id: egui::TextureId,
    pub width: u32,
    pub height: u32,
}

/// Key for caching icons: (path, index, requested_size)
type IconCacheKey = (PathBuf, u32, u32);

#[derive(Debug, Clone)]
pub struct DllEntry {
    pub path: PathBuf,
    pub icon_count: u32,
    pub icons: Vec<IconEntry>,
}

pub enum Pane {
    Tree,
    Preview,
}

struct TreeBehavior {
    dll_entries: Vec<DllEntry>,
    selected_icon: Option<IconEntry>,
    textures: HashMap<IconCacheKey, Option<LoadedIconInfo>>, // None means failed to load
    texture_handles: Vec<egui::TextureHandle>, // Keep handles alive
}

impl TreeBehavior {
    fn new(paths: Vec<PathBuf>) -> Self {
        let dll_entries: Vec<DllEntry> = paths
            .into_iter()
            .map(|path| {
                let icon_count = get_icon_count(&path).unwrap_or(0);
                let icons = (0..icon_count)
                    .map(|i| IconEntry {
                        dll_path: path.clone(),
                        index: i,
                    })
                    .collect();
                DllEntry {
                    path,
                    icon_count,
                    icons,
                }
            })
            .collect();

        Self {
            dll_entries,
            selected_icon: None,
            textures: HashMap::new(),
            texture_handles: Vec::new(),
        }
    }

    fn load_icon_texture(
        &mut self,
        ctx: &egui::Context,
        dll_path: &PathBuf,
        index: u32,
        size: u32,
    ) -> Option<LoadedIconInfo> {
        let key = (dll_path.clone(), index, size);
        if let Some(info) = self.textures.get(&key) {
            return info.clone();
        }

        // Try to load the icon at the requested size
        if let Ok(rgba_image) = load_icon_from_dll_sized(dll_path, index, size) {
            let width = rgba_image.width();
            let height = rgba_image.height();
            let img_size = [width as usize, height as usize];
            let pixels = rgba_image.into_raw();
            let color_image = egui::ColorImage::from_rgba_unmultiplied(img_size, &pixels);
            let handle = ctx.load_texture(
                format!("icon_{}_{}_{}", dll_path.display(), index, size),
                color_image,
                egui::TextureOptions::default(),
            );
            let info = LoadedIconInfo {
                texture_id: handle.id(),
                width,
                height,
            };
            self.textures.insert(key, Some(info.clone()));
            self.texture_handles.push(handle);
            return Some(info);
        }

        // Mark as failed so we don't retry
        self.textures.insert(key, None);
        None
    }

    /// Load icon at default 32x32 size for tree view
    fn load_icon_texture_default(
        &mut self,
        ctx: &egui::Context,
        dll_path: &PathBuf,
        index: u32,
    ) -> Option<LoadedIconInfo> {
        self.load_icon_texture(ctx, dll_path, index, 32)
    }
}

impl egui_tiles::Behavior<Pane> for TreeBehavior {
    fn tab_title_for_pane(&mut self, pane: &Pane) -> egui::WidgetText {
        match pane {
            Pane::Tree => "Icons".into(),
            Pane::Preview => "Preview".into(),
        }
    }

    fn pane_ui(
        &mut self,
        ui: &mut egui::Ui,
        _tile_id: TileId,
        pane: &mut Pane,
    ) -> egui_tiles::UiResponse {
        match pane {
            Pane::Tree => {
                self.render_tree_pane(ui);
            }
            Pane::Preview => {
                self.render_preview_pane(ui);
            }
        }
        egui_tiles::UiResponse::None
    }

    fn simplification_options(&self) -> egui_tiles::SimplificationOptions {
        egui_tiles::SimplificationOptions {
            all_panes_must_have_tabs: true,
            ..Default::default()
        }
    }
}

impl TreeBehavior {
    fn render_tree_pane(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            let dll_entries = self.dll_entries.clone();
            for dll_entry in dll_entries.iter() {
                let header_text = format!(
                    "{} ({} icons)",
                    dll_entry
                        .path
                        .file_name()
                        .map(|s| s.to_string_lossy())
                        .unwrap_or_else(|| dll_entry.path.to_string_lossy()),
                    dll_entry.icon_count
                );

                egui::CollapsingHeader::new(header_text)
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            for icon in &dll_entry.icons {
                                let loaded_info =
                                    self.load_icon_texture_default(ui.ctx(), &icon.dll_path, icon.index);

                                let response = if let Some(ref info) = loaded_info {
                                    ui.add(
                                        egui::ImageButton::new(egui::Image::new((
                                            info.texture_id,
                                            egui::vec2(32.0, 32.0),
                                        )))
                                        .frame(true),
                                    )
                                } else {
                                    ui.add(egui::Button::new(format!("#{}", icon.index)))
                                };

                                if response.clicked() {
                                    self.selected_icon = Some(icon.clone());
                                }

                                let hover_text = if let Some(ref info) = loaded_info {
                                    format!(
                                        "{},-{}\nSize: {}x{}",
                                        dll_entry.path.display(),
                                        icon.index,
                                        info.width,
                                        info.height
                                    )
                                } else {
                                    format!("{},-{}", dll_entry.path.display(), icon.index)
                                };
                                response.on_hover_text(hover_text);
                            }
                        });
                    });
            }
        });
    }

    fn render_preview_pane(&mut self, ui: &mut egui::Ui) {
        let selected_icon = self.selected_icon.clone();
        if let Some(icon) = selected_icon {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Selected Icon");
                    ui.separator();

                    ui.label(format!("DLL: {}", icon.dll_path.display()));
                    ui.label(format!("Index: {}", icon.index));
                    ui.label(format!("Path: {},-{}", icon.dll_path.display(), icon.index));

                    ui.separator();
                    ui.heading("Available Sizes");
                    ui.label("Each size is extracted separately from the icon resource:");
                    
                    // Try to load at different sizes
                    let sizes = [16, 24, 32, 48, 64, 96, 128, 256];
                    
                    ui.horizontal_wrapped(|ui| {
                        for &size in &sizes {
                            ui.vertical(|ui| {
                                ui.label(format!("{}x{}", size, size));
                                if let Some(info) = self.load_icon_texture(ui.ctx(), &icon.dll_path, icon.index, size) {
                                    ui.image((info.texture_id, egui::vec2(size as f32, size as f32)));
                                    if info.width != size || info.height != size {
                                        ui.small(format!("(actual: {}x{})", info.width, info.height));
                                    }
                                } else {
                                    ui.label("N/A");
                                }
                            });
                        }
                    });

                    ui.separator();

                    if ui.button("Copy path to clipboard").clicked() {
                        ui.output_mut(|o| {
                            o.copied_text = format!("{},-{}", icon.dll_path.display(), icon.index);
                        });
                    }
                });
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("Select an icon from the tree to preview");
            });
        }
    }
}

struct IconBrowserApp {
    tree: egui_tiles::Tree<Pane>,
    behavior: TreeBehavior,
}

impl IconBrowserApp {
    fn new(_cc: &eframe::CreationContext<'_>, paths: Vec<PathBuf>) -> Self {
        let mut tiles = Tiles::default();

        let tree_pane = tiles.insert_pane(Pane::Tree);
        let preview_pane = tiles.insert_pane(Pane::Preview);

        let root = tiles.insert_horizontal_tile(vec![tree_pane, preview_pane]);

        let tree = egui_tiles::Tree::new("icon_browser", root, tiles);
        let behavior = TreeBehavior::new(paths);

        Self { tree, behavior }
    }
}

impl eframe::App for IconBrowserApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            self.tree.ui(&mut self.behavior, ui);
        });
    }
}

fn get_icon_count(path: &PathBuf) -> Result<u32> {
    let path_str = path.to_string_lossy();
    let pcwstr = path_str.as_ref().easy_pcwstr()?;

    // Pass -1 as nIconIndex and NULL for both icon arrays to get the count
    let count = unsafe { ExtractIconExW(pcwstr.as_ref(), -1, None, None, 0) };

    Ok(count)
}

fn load_icon_from_dll_sized(path: &PathBuf, index: u32, size: u32) -> Result<image::RgbaImage> {
    let path_str = path.to_string_lossy();
    
    // PrivateExtractIconsW requires a fixed-size buffer of 260 u16s
    let mut filename_buf: [u16; 260] = [0; 260];
    for (i, c) in path_str.encode_utf16().take(259).enumerate() {
        filename_buf[i] = c;
    }

    let mut icons: [HICON; 1] = [HICON::default()];
    let mut icon_id: u32 = 0;

    // Use PrivateExtractIconsW to extract icon at specific size
    let extracted = unsafe {
        PrivateExtractIconsW(
            &filename_buf,
            index as i32,
            size as i32,
            size as i32,
            Some(&mut icons),
            Some(&raw mut icon_id),
            1,
        )
    };

    if extracted == 0 || icons[0].is_invalid() {
        eyre::bail!("Failed to extract icon at index {} with size {}", index, size);
    }

    // The icon handle needs to be destroyed after use
    let result = unsafe { hicon_to_rgba(icons[0]) };

    // Destroy the icon handle
    unsafe {
        _ = windows::Win32::UI::WindowsAndMessaging::DestroyIcon(icons[0]);
    }

    result
}
