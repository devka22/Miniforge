//! Shared editor UI services: icons, fuzzy search, desktop integration and previews.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crossbeam_channel::{Receiver, unbounded};
use image::RgbaImage;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use nucleo_matcher::pattern::{AtomKind, CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher};
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditorIcon {
    Save,
    Open,
    NewEntity,
    Component,
    Camera,
    Audio,
    Script,
    Prefab,
    Folder,
    Error,
    Warning,
    Search,
    Settings,
    Delete,
    Copy,
    Paste,
    ExternalLink,
    Scene,
    Play,
    Stop,
    Refresh,
    Validate,
    Graph,
    Image,
}

impl EditorIcon {
    pub fn glyph(self) -> &'static str {
        use egui_phosphor::regular as icon;
        match self {
            Self::Save => icon::FLOPPY_DISK,
            Self::Open => icon::FOLDER_OPEN,
            Self::NewEntity => icon::CUBE,
            Self::Component => icon::PUZZLE_PIECE,
            Self::Camera => icon::CAMERA,
            Self::Audio => icon::SPEAKER_HIGH,
            Self::Script => icon::FILE_CODE,
            Self::Prefab => icon::PACKAGE,
            Self::Folder => icon::FOLDER,
            Self::Error => icon::X_CIRCLE,
            Self::Warning => icon::WARNING,
            Self::Search => icon::MAGNIFYING_GLASS,
            Self::Settings => icon::GEAR,
            Self::Delete => icon::TRASH,
            Self::Copy => icon::COPY,
            Self::Paste => icon::CLIPBOARD_TEXT,
            Self::ExternalLink => icon::ARROW_SQUARE_OUT,
            Self::Scene => icon::LAYOUT,
            Self::Play => icon::PLAY,
            Self::Stop => icon::STOP,
            Self::Refresh => icon::ARROWS_CLOCKWISE,
            Self::Validate => icon::CHECK_CIRCLE,
            Self::Graph => icon::GRAPH,
            Self::Image => icon::IMAGE,
        }
    }

    pub fn label(self, text: &str) -> String {
        format!("{}  {text}", self.glyph())
    }
}

pub fn install_phosphor_fonts(ctx: &egui::Context) {
    let installed = egui::Id::new("miniforge.phosphor-font-installed");
    if ctx.data(|data| data.get_temp::<bool>(installed).unwrap_or(false)) {
        return;
    }
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "phosphor".into(),
        egui::FontData::from_static(egui_phosphor::Variant::Regular.font_bytes()).into(),
    );
    if let Some(font_keys) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
        font_keys.insert(1, "phosphor".into());
    }
    ctx.set_fonts(fonts);
    ctx.data_mut(|data| data.insert_temp(installed, true));
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuzzySearchResult {
    pub index: usize,
    pub score: u32,
}

#[derive(Clone)]
struct IndexedCandidate<'a> {
    index: usize,
    searchable: &'a str,
}

impl AsRef<str> for IndexedCandidate<'_> {
    fn as_ref(&self) -> &str {
        self.searchable
    }
}

/// Ranks a small UI list using nucleo's Unicode-aware fuzzy matcher.
pub fn fuzzy_rank(query: &str, candidates: &[String], limit: usize) -> Vec<FuzzySearchResult> {
    let query = query.trim();
    if candidates.is_empty() || limit == 0 {
        return Vec::new();
    }
    if query.is_empty() {
        return candidates
            .iter()
            .enumerate()
            .take(limit)
            .map(|(index, _)| FuzzySearchResult { index, score: 0 })
            .collect();
    }

    let pattern = Pattern::new(
        query,
        CaseMatching::Smart,
        Normalization::Smart,
        AtomKind::Fuzzy,
    );
    let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
    pattern
        .match_list(
            candidates
                .iter()
                .enumerate()
                .map(|(index, searchable)| IndexedCandidate { index, searchable }),
            &mut matcher,
        )
        .into_iter()
        .take(limit)
        .map(|(candidate, score)| FuzzySearchResult {
            index: candidate.index,
            score,
        })
        .collect()
}

pub fn fuzzy_rank_strs(query: &str, candidates: &[&str], limit: usize) -> Vec<FuzzySearchResult> {
    let owned = candidates
        .iter()
        .map(|candidate| (*candidate).to_string())
        .collect::<Vec<_>>();
    fuzzy_rank(query, &owned, limit)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightSpan {
    pub text: String,
    pub rgba: [u8; 4],
    pub bold: bool,
    pub italic: bool,
}

pub struct EditorSyntaxHighlighter {
    syntaxes: SyntaxSet,
    themes: ThemeSet,
    theme: String,
}

impl Default for EditorSyntaxHighlighter {
    fn default() -> Self {
        Self {
            syntaxes: SyntaxSet::load_defaults_newlines(),
            themes: ThemeSet::load_defaults(),
            theme: "base16-ocean.dark".to_string(),
        }
    }
}

impl EditorSyntaxHighlighter {
    pub fn highlight_line(&self, line: &str, extension: &str) -> Vec<HighlightSpan> {
        let syntax_extension = match extension.to_ascii_lowercase().as_str() {
            "luau" => "lua",
            "mfgraph" | "scene" | "prefab" => "json",
            _ => extension,
        };
        let syntax = self
            .syntaxes
            .find_syntax_by_extension(syntax_extension)
            .or_else(|| self.syntaxes.find_syntax_by_extension("rs"))
            .unwrap_or_else(|| self.syntaxes.find_syntax_plain_text());
        let Some(theme) = self
            .themes
            .themes
            .get(&self.theme)
            .or_else(|| self.themes.themes.values().next())
        else {
            return vec![HighlightSpan {
                text: line.to_string(),
                rgba: [220, 225, 235, 255],
                bold: false,
                italic: false,
            }];
        };
        let mut highlighter = HighlightLines::new(syntax, theme);
        highlighter
            .highlight_line(line, &self.syntaxes)
            .unwrap_or_default()
            .into_iter()
            .map(|(style, text)| HighlightSpan {
                text: text.to_string(),
                rgba: [
                    style.foreground.r,
                    style.foreground.g,
                    style.foreground.b,
                    style.foreground.a,
                ],
                bold: style
                    .font_style
                    .contains(syntect::highlighting::FontStyle::BOLD),
                italic: style
                    .font_style
                    .contains(syntect::highlighting::FontStyle::ITALIC),
            })
            .collect()
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct EditorClipboard;

impl EditorClipboard {
    pub fn copy_text(text: impl Into<String>) -> Result<(), String> {
        let mut clipboard = arboard::Clipboard::new().map_err(|error| error.to_string())?;
        clipboard
            .set_text(text.into())
            .map_err(|error| error.to_string())
    }

    pub fn paste_text() -> Result<String, String> {
        let mut clipboard = arboard::Clipboard::new().map_err(|error| error.to_string())?;
        clipboard.get_text().map_err(|error| error.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorFileChange {
    pub paths: Vec<PathBuf>,
    pub kind: String,
}

pub struct EditorFileWatcher {
    _watcher: RecommendedWatcher,
    receiver: Receiver<notify::Result<Event>>,
}

impl EditorFileWatcher {
    pub fn watch(path: impl AsRef<Path>) -> notify::Result<Self> {
        let (sender, receiver) = unbounded();
        let mut watcher = notify::recommended_watcher(move |event| {
            let _ = sender.send(event);
        })?;
        watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;
        Ok(Self {
            _watcher: watcher,
            receiver,
        })
    }

    pub fn drain(&self) -> Vec<EditorFileChange> {
        let mut changes = Vec::new();
        let mut seen = BTreeSet::new();
        while let Ok(event) = self.receiver.try_recv() {
            let Ok(event) = event else {
                continue;
            };
            let paths = event
                .paths
                .into_iter()
                .filter(|path| !ignored_editor_change(path))
                .filter(|path| seen.insert(path.clone()))
                .collect::<Vec<_>>();
            if !paths.is_empty() {
                changes.push(EditorFileChange {
                    paths,
                    kind: format!("{:?}", event.kind),
                });
            }
        }
        changes
    }
}

pub fn open_in_default_application(path: impl AsRef<Path>) -> Result<(), String> {
    open::that(path.as_ref()).map_err(|error| error.to_string())
}

pub fn move_to_trash(path: impl AsRef<Path>) -> Result<(), String> {
    trash::delete(path.as_ref()).map_err(|error| error.to_string())
}

pub fn rasterize_svg(
    svg: &[u8],
    target_width: u32,
    target_height: u32,
) -> Result<RgbaImage, String> {
    if target_width == 0 || target_height == 0 {
        return Err("SVG target dimensions must be greater than zero".to_string());
    }
    let tree =
        usvg::Tree::from_data(svg, &usvg::Options::default()).map_err(|error| error.to_string())?;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(target_width, target_height)
        .ok_or_else(|| "could not allocate SVG preview".to_string())?;
    let source_size = tree.size();
    let scale = (target_width as f32 / source_size.width())
        .min(target_height as f32 / source_size.height());
    let offset_x = (target_width as f32 - source_size.width() * scale) * 0.5;
    let offset_y = (target_height as f32 - source_size.height() * scale) * 0.5;
    let transform =
        resvg::tiny_skia::Transform::from_translate(offset_x, offset_y).post_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    RgbaImage::from_raw(target_width, target_height, pixmap.take())
        .ok_or_else(|| "invalid SVG preview pixel buffer".to_string())
}

fn ignored_editor_change(path: &Path) -> bool {
    let text = path.to_string_lossy();
    text.contains("/target/")
        || text.contains("/build/")
        || text.ends_with("asset_metadata.json")
        || text.ends_with(".DS_Store")
        || text.ends_with(".bak")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuzzy_search_handles_non_contiguous_queries() {
        let candidates = vec![
            "Save current scene".to_string(),
            "Create audio component".to_string(),
            "Open prefab editor".to_string(),
        ];
        let matches = fuzzy_rank("opref", &candidates, 3);
        assert_eq!(matches.first().map(|item| item.index), Some(2));
    }

    #[test]
    fn icons_and_syntax_are_ready_for_editor_surfaces() {
        assert!(!EditorIcon::Save.glyph().is_empty());
        assert!(EditorIcon::Warning.label("Warnings").contains("Warnings"));
        let spans = EditorSyntaxHighlighter::default().highlight_line("fn update() {}", "rs");
        assert!(!spans.is_empty());
        assert_eq!(
            spans
                .iter()
                .map(|span| span.text.as_str())
                .collect::<String>(),
            "fn update() {}"
        );
    }

    #[test]
    fn svg_preview_rasterizes_to_requested_size() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><rect width="16" height="16" fill="#ff0000"/></svg>"##;
        let image = rasterize_svg(svg, 32, 24).unwrap();
        assert_eq!(image.dimensions(), (32, 24));
        assert!(image.pixels().any(|pixel| pixel.0[0] > 0));
    }
}
