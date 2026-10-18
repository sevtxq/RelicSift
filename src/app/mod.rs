use crate::{
    analysis::{engine, rules},
    export, import,
    models::*,
};
use eframe::egui::{self, Color32, RichText};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, mpsc},
};

const BG: Color32 = Color32::from_rgb(16, 19, 26);
const SURFACE: Color32 = Color32::from_rgb(24, 29, 39);
const MUTED: Color32 = Color32::from_rgb(145, 156, 176);
const ACCENT: Color32 = Color32::from_rgb(148, 177, 252);
fn category_color(category: Category) -> Color32 {
    match category {
        Category::Keep => Color32::from_rgb(123, 205, 174),
        Category::PotentiallyUseful => Color32::from_rgb(212, 187, 132),
        Category::SafeDiscard => Color32::from_rgb(185, 158, 172),
    }
}

enum Completed {
    Imported(Box<Imported>, PathBuf),
    Analyzed(Vec<Decision>),
    Exported(PathBuf),
}
type Job = Result<Completed, String>;

#[derive(Clone, PartialEq)]
struct Filters {
    category: Option<Category>,
    query: String,
    set: String,
    slot: String,
    mainstat: String,
    level_min: u8,
    level_max: u8,
    score_min: f64,
    protection: usize,
    sort: usize,
}
impl Default for Filters {
    fn default() -> Self {
        Self {
            category: None,
            query: String::new(),
            set: String::new(),
            slot: String::new(),
            mainstat: String::new(),
            level_min: 0,
            level_max: 15,
            score_min: 0.0,
            protection: 0,
            sort: 0,
        }
    }
}

pub struct RelicSift {
    input: Option<Arc<Imported>>,
    input_path: Option<PathBuf>,
    config: Config,
    decisions: Vec<Decision>,
    overrides: BTreeMap<String, ManualOverride>,
    session_overrides: BTreeMap<String, BTreeMap<String, ManualOverride>>,
    filters: Filters,
    filtered: Vec<usize>,
    sets: Vec<(String, String)>,
    slots: Vec<String>,
    mainstats: Vec<String>,
    dirty: bool,
    selected: Option<usize>,
    override_category: Category,
    override_reason: String,
    job: Option<mpsc::Receiver<Job>>,
    status: String,
    error: Option<String>,
    settings: bool,
    rules_open: bool,
    unsaved: bool,
}

impl RelicSift {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let ctx = &cc.egui_ctx;
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = BG;
        visuals.window_fill = SURFACE;
        visuals.extreme_bg_color = Color32::from_rgb(12, 15, 22);
        visuals.selection.bg_fill = Color32::from_rgb(51, 69, 108);
        visuals.widgets.inactive.bg_fill = SURFACE;
        visuals.widgets.inactive.weak_bg_fill = SURFACE;
        visuals.widgets.inactive.bg_stroke =
            egui::Stroke::new(1.0_f32, Color32::from_rgb(42, 50, 65));
        ctx.set_visuals(visuals);
        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(10.0, 10.0);
        style.spacing.button_padding = egui::vec2(13.0, 9.0);
        style
            .text_styles
            .insert(egui::TextStyle::Body, egui::FontId::proportional(14.0));
        style
            .text_styles
            .insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
        style
            .text_styles
            .insert(egui::TextStyle::Heading, egui::FontId::proportional(25.0));
        ctx.set_style(style);
        Self {
            input: None,
            input_path: None,
            config: Config::default(),
            decisions: Vec::new(),
            overrides: BTreeMap::new(),
            session_overrides: BTreeMap::new(),
            filters: Filters::default(),
            filtered: Vec::new(),
            sets: Vec::new(),
            slots: Vec::new(),
            mainstats: Vec::new(),
            dirty: true,
            selected: None,
            override_category: Category::PotentiallyUseful,
            override_reason: String::new(),
            job: None,
            status: "Local. Offline. Explainable.".into(),
            error: None,
            settings: false,
            rules_open: false,
            unsaved: false,
        }
    }
    fn spawn(&mut self, ctx: &egui::Context, f: impl FnOnce() -> Job + Send + 'static) {
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_or_else(|_| {
                    Err("The operation stopped unexpectedly; existing data is preserved.".into())
                });
            let _ = tx.send(result);
            ctx.request_repaint();
        });
        self.job = Some(rx);
        self.error = None;
    }
    fn load(&mut self, path: PathBuf, ctx: &egui::Context) {
        self.status = "Checking inventory …".into();
        self.spawn(ctx, move || {
            import::read(&path)
                .map(|i| Completed::Imported(Box::new(i), path))
                .map_err(|e| e.to_string())
        });
    }
    fn choose_file(&mut self, ctx: &egui::Context) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Reliquary / RelicSift", &["json"])
            .pick_file()
        {
            self.load(path, ctx);
        }
    }
    fn analyze(&mut self, ctx: &egui::Context) {
        if let Some(input) = self.input.clone() {
            let config = self.config.clone();
            let overrides = self.overrides.clone();
            self.status =
                "Evaluating archetypes, upgrade potential and inventory comparisons …".into();
            self.spawn(ctx, move || {
                engine::analyze(&input, &config, &overrides).map(Completed::Analyzed)
            });
        }
    }
    fn poll(&mut self) {
        let Some(rx) = &self.job else {
            return;
        };
        let received = match rx.try_recv() {
            Ok(job) => job,
            Err(mpsc::TryRecvError::Empty) => return,
            Err(mpsc::TryRecvError::Disconnected) => Err("The background operation ended.".into()),
        };
        self.job = None;
        match received {
            Ok(Completed::Imported(input, path)) => {
                if let Some(old) = &self.input {
                    self.session_overrides
                        .insert(old.fingerprint.clone(), self.overrides.clone());
                }
                self.config = input.saved_config.clone().unwrap_or_default();
                self.overrides = if input.saved_config.is_some() {
                    input.overrides.clone()
                } else {
                    self.session_overrides
                        .get(&input.fingerprint)
                        .cloned()
                        .unwrap_or_default()
                };
                self.sets = input
                    .archive
                    .relics
                    .iter()
                    .map(|r| (r.set_id.clone(), r.name.clone()))
                    .collect::<BTreeMap<_, _>>()
                    .into_iter()
                    .collect();
                self.slots = input
                    .archive
                    .relics
                    .iter()
                    .map(|r| r.slot.clone())
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .collect();
                self.mainstats = input
                    .archive
                    .relics
                    .iter()
                    .map(|r| r.mainstat.clone())
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .collect();
                self.status = format!(
                    "{} relics imported in full. Ready to analyze.",
                    input.archive.relics.len()
                );
                self.input = Some(Arc::new(*input));
                self.input_path = Some(path);
                self.decisions.clear();
                self.selected = None;
                self.filters = Filters::default();
                self.dirty = true;
                self.unsaved = false;
            }
            Ok(Completed::Analyzed(decisions)) => {
                self.status = format!(
                    "{} relics analyzed. Every recommendation can be explained.",
                    decisions.len()
                );
                self.decisions = decisions;
                self.dirty = true;
                self.unsaved = true;
            }
            Ok(Completed::Exported(path)) => {
                self.status = format!("Saved: {}", path.display());
                self.unsaved = false;
            }
            Err(error) => {
                self.status = "Operation canceled. Existing data is preserved.".into();
                self.error = Some(error);
            }
        }
    }
    fn save(&mut self, ctx: &egui::Context) {
        let Some(input) = self.input.clone() else {
            return;
        };
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("RelicSift result", &["json"])
            .set_file_name("inventory.relicsift.json")
            .save_file()
        {
            let decisions = self.decisions.clone();
            let config = self.config.clone();
            self.status = "Writing the complete result export …".into();
            self.spawn(ctx, move || {
                let report = export::report(&input, config, decisions)?;
                export::write_new(&path, &report)?;
                Ok(Completed::Exported(path))
            });
        }
    }
    fn refresh_filter(&mut self) {
        if !self.dirty {
            return;
        }
        self.dirty = false;
        let Some(input) = &self.input else {
            return;
        };
        let f = &self.filters;
        let query = f.query.to_lowercase();
        self.filtered = input
            .archive
            .relics
            .iter()
            .enumerate()
            .filter(|(i, r)| {
                let d = self.decisions.get(*i);
                (f.category.is_none() || d.is_some_and(|d| Some(d.category) == f.category))
                    && (f.set.is_empty() || f.set == r.set_id)
                    && (f.slot.is_empty() || f.slot == r.slot)
                    && (f.mainstat.is_empty() || f.mainstat == r.mainstat)
                    && (f.level_min..=f.level_max).contains(&r.level)
                    && d.is_none_or(|d| d.score >= f.score_min)
                    && (query.is_empty()
                        || r.name.to_lowercase().contains(&query)
                        || r.uid.to_lowercase().contains(&query))
                    && match f.protection {
                        1 => r.protected(),
                        2 => !r.protected(),
                        3 => r.lock,
                        4 => r.equipped(),
                        _ => true,
                    }
            })
            .map(|(i, _)| i)
            .collect();
        self.filtered.sort_by(|&a, &b| {
            let ra = &input.archive.relics[a];
            let rb = &input.archive.relics[b];
            match f.sort {
                1 => rb.level.cmp(&ra.level),
                2 => ra.name.cmp(&rb.name),
                3 => ra.uid.cmp(&rb.uid),
                _ => self
                    .decisions
                    .get(b)
                    .map_or(0.0, |d| d.score)
                    .total_cmp(&self.decisions.get(a).map_or(0.0, |d| d.score)),
            }
            .then(ra.uid.cmp(&rb.uid))
        });
    }
    fn select(&mut self, index: usize) {
        self.selected = Some(index);
        let over = self
            .input
            .as_ref()
            .and_then(|i| self.overrides.get(&i.archive.relics[index].uid));
        self.override_category = over.map_or_else(
            || {
                self.decisions
                    .get(index)
                    .map_or(Category::PotentiallyUseful, |d| d.category)
            },
            |o| o.category,
        );
        self.override_reason = over.map_or(String::new(), |o| o.reason.clone());
    }
    fn apply_override(&mut self, remove: bool) {
        let Some(index) = self.selected else {
            return;
        };
        let Some(decision) = self.decisions.get_mut(index) else {
            return;
        };
        if !remove {
            let over = ManualOverride {
                category: self.override_category,
                reason: self.override_reason.clone(),
            };
            if let Err(error) = rules::validate_override(&decision.relic, &self.config, &over) {
                self.error = Some(error);
                return;
            }
        }
        self.error = None;
        decision.rule_ids.retain(|id| id != "M001");
        let registry = rules::registry();
        let explanation = decision
            .rule_ids
            .first()
            .and_then(|id| registry.iter().find(|r| r.id == id))
            .map(|r| format!("{}: {}", r.name, r.description))
            .unwrap_or_default();
        if remove {
            self.overrides.remove(&decision.relic.uid);
            decision.manual_override = None;
            decision.category = decision.automatic_category;
            decision.explanation = explanation;
        } else {
            let over = ManualOverride {
                category: self.override_category,
                reason: self.override_reason.trim().to_owned(),
            };
            self.overrides
                .insert(decision.relic.uid.clone(), over.clone());
            decision.category = over.category;
            decision.manual_override = Some(over.clone());
            decision.rule_ids.insert(0, "M001".into());
            decision.explanation = format!(
                "Manual: {}. Automatic: {}. {}",
                over.reason,
                decision.automatic_category.label(),
                explanation
            );
        }
        self.unsaved = true;
        self.dirty = true;
    }
    fn header(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("header")
            .frame(egui::Frame::new().fill(BG).inner_margin(22))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("RelicSift").size(27.0).strong());
                        ui.label(
                            RichText::new("Your inventory. With a future.")
                                .color(MUTED)
                                .size(13.0),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.input.is_some() {
                            if ui
                                .add_enabled(
                                    self.job.is_none() && !self.decisions.is_empty(),
                                    egui::Button::new("Export results"),
                                )
                                .clicked()
                            {
                                self.save(ctx);
                            }
                            if ui
                                .add_enabled(
                                    self.job.is_none(),
                                    egui::Button::new(RichText::new("Analyze").color(ACCENT)),
                                )
                                .clicked()
                            {
                                self.analyze(ctx);
                            }
                            if ui
                                .add_enabled(self.job.is_none(), egui::Button::new("Open file"))
                                .clicked()
                            {
                                self.choose_file(ctx);
                            }
                        }
                        if ui.small_button("Rules").clicked() {
                            self.rules_open = true;
                        }
                        if ui.small_button("Settings").clicked() {
                            self.settings = !self.settings;
                        }
                    });
                });
            });
    }
    fn empty(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.add_space(65.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new("Room for new possibilities.")
                    .size(32.0)
                    .strong(),
            );
            ui.add_space(10.0);
            ui.label(
                RichText::new("Preserve valuable relics. Make uncertainty visible.")
                    .color(MUTED)
                    .size(16.0),
            );
            ui.add_space(36.0);
            egui::Frame::new()
                .fill(SURFACE)
                .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(57, 71, 94)))
                .corner_radius(16)
                .inner_margin(48)
                .show(ui, |ui| {
                    ui.set_min_size(egui::vec2(550.0, 160.0));
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("Drop a JSON file here").size(22.0));
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new("Reliquary Archiver or a saved RelicSift result")
                                .color(MUTED),
                        );
                        ui.add_space(20.0);
                        if ui
                            .add_enabled(
                                self.job.is_none(),
                                egui::Button::new("Choose file").min_size(egui::vec2(185.0, 42.0)),
                            )
                            .clicked()
                        {
                            self.choose_file(ctx);
                        }
                    });
                });
            ui.add_space(24.0);
            ui.label(
                RichText::new("Fully offline  ·  No game access  ·  Original file preserved")
                    .color(MUTED)
                    .size(12.0),
            );
        });
    }
    fn dashboard(&mut self, ui: &mut egui::Ui) {
        let Some(input) = &self.input else {
            return;
        };
        let total = input.archive.relics.len();
        let protected = input
            .archive
            .relics
            .iter()
            .filter(|r| r.protected())
            .count();
        let mut categories = [0; 3];
        for d in &self.decisions {
            categories[d.category as usize] += 1;
        }
        ui.horizontal_wrapped(|ui| {
            for (category, label, count) in [
                (None, "All", total),
                (Some(Category::Keep), "Keep", categories[0]),
                (
                    Some(Category::PotentiallyUseful),
                    "Potentially Useful",
                    categories[1],
                ),
                (Some(Category::SafeDiscard), "Safe Discard", categories[2]),
            ] {
                let selected = self.filters.category == category;
                let color = category.map_or(ACCENT, category_color);
                let value = if category.is_some() && self.decisions.is_empty() {
                    "—".into()
                } else {
                    count.to_string()
                };
                let text = RichText::new(format!("{value}   {label}"))
                    .size(16.0)
                    .color(if selected { color } else { MUTED });
                if ui
                    .add(
                        egui::Button::new(text)
                            .selected(selected)
                            .min_size(egui::vec2(135.0, 64.0)),
                    )
                    .clicked()
                {
                    self.filters.category = category;
                    self.dirty = true;
                }
            }
        });
        ui.add_space(5.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(format!("{protected} locked or equipped")).color(MUTED));
            if let Some(path) = &self.input_path {
                ui.label(
                    RichText::new(format!(
                        "· {}",
                        path.file_name().unwrap_or_default().to_string_lossy()
                    ))
                    .color(MUTED),
                );
            }
            if !self.overrides.is_empty() {
                ui.label(
                    RichText::new(format!("· {} manual decisions", self.overrides.len()))
                        .color(ACCENT),
                );
            }
            if self.unsaved {
                ui.label(RichText::new("· not exported yet").color(MUTED));
            }
        });
        ui.add_space(12.0);
        let before = self.filters.clone();
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.filters.query)
                    .hint_text("Search name or ID …")
                    .desired_width(210.0),
            );
            egui::ComboBox::from_id_salt("set")
                .selected_text(if self.filters.set.is_empty() {
                    "All sets".into()
                } else {
                    self.sets
                        .iter()
                        .find(|(id, _)| id == &self.filters.set)
                        .map_or(self.filters.set.clone(), |(_, n)| n.clone())
                })
                .width(210.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.filters.set, String::new(), "All sets");
                    for (id, name) in &self.sets {
                        ui.selectable_value(&mut self.filters.set, id.clone(), name);
                    }
                });
            string_combo(ui, "slot", "All slots", &mut self.filters.slot, &self.slots);
            string_combo(
                ui,
                "main",
                "All main stats",
                &mut self.filters.mainstat,
                &self.mainstats,
            );
            egui::ComboBox::from_id_salt("protection")
                .selected_text(
                    [
                        "Any status",
                        "Protected",
                        "Unprotected",
                        "Locked",
                        "Equipped",
                    ][self.filters.protection],
                )
                .show_ui(ui, |ui| {
                    for (index, label) in [
                        "Any status",
                        "Protected",
                        "Unprotected",
                        "Locked",
                        "Equipped",
                    ]
                    .iter()
                    .enumerate()
                    {
                        ui.selectable_value(&mut self.filters.protection, index, *label);
                    }
                });
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Level").color(MUTED));
            ui.add(
                egui::DragValue::new(&mut self.filters.level_min)
                    .range(0..=15)
                    .prefix("from +"),
            );
            ui.add(
                egui::DragValue::new(&mut self.filters.level_max)
                    .range(0..=15)
                    .prefix("to +"),
            );
            ui.label(RichText::new("Min. score").color(MUTED));
            ui.add(
                egui::DragValue::new(&mut self.filters.score_min)
                    .range(0.0..=20.0)
                    .speed(0.1)
                    .fixed_decimals(1),
            );
            egui::ComboBox::from_id_salt("sort")
                .selected_text(["Score ↓", "Level ↓", "Set A–Z", "ID"][self.filters.sort])
                .show_ui(ui, |ui| {
                    for (i, label) in ["Score ↓", "Level ↓", "Set A–Z", "ID"].iter().enumerate()
                    {
                        ui.selectable_value(&mut self.filters.sort, i, *label);
                    }
                });
            if ui.small_button("Reset filters").clicked() {
                self.filters = Filters::default();
            }
        });
        if before != self.filters {
            self.dirty = true;
        }
        ui.add_space(10.0);
    }
    fn list(&mut self, ui: &mut egui::Ui) {
        self.refresh_filter();
        let Some(input) = &self.input else {
            return;
        };
        ui.label(
            RichText::new(format!("{} relics in this view", self.filtered.len()))
                .color(MUTED)
                .size(12.0),
        );
        ui.add_space(4.0);
        let mut select = None;
        egui::ScrollArea::vertical()
            .id_salt("relic_list")
            .auto_shrink([false, false])
            .show_rows(ui, 136.0, self.filtered.len(), |ui, range| {
                for row in range {
                    let index = self.filtered[row];
                    let relic = &input.archive.relics[index];
                    let decision = self.decisions.get(index);
                    let selected = self.selected == Some(index);
                    let response = egui::Frame::new()
                        .fill(if selected {
                            Color32::from_rgb(32, 43, 63)
                        } else {
                            SURFACE
                        })
                        .corner_radius(10)
                        .inner_margin(12)
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());
                            ui.set_min_height(112.0);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(&relic.name).strong().size(15.0));
                                ui.label(RichText::new(format!("+{}", relic.level)).color(ACCENT));
                                if relic.lock {
                                    ui.label(RichText::new("Locked").color(MUTED).size(11.0));
                                }
                                if relic.equipped() {
                                    ui.label(RichText::new("Equipped").color(MUTED).size(11.0));
                                }
                                if let Some(d) = decision {
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                RichText::new(format!(
                                                    "{}{}",
                                                    d.category.label(),
                                                    if d.manual_override.is_some() {
                                                        " · manual"
                                                    } else {
                                                        ""
                                                    }
                                                ))
                                                .color(category_color(d.category))
                                                .size(12.0),
                                            );
                                            ui.label(
                                                RichText::new(format!("{:.1}", d.score))
                                                    .color(MUTED),
                                            );
                                        },
                                    );
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "{} · {} · {}★",
                                        relic.slot, relic.mainstat, relic.rarity
                                    ))
                                    .color(MUTED)
                                    .size(12.0),
                                );
                                ui.label(
                                    RichText::new(format!("#{}", relic.uid))
                                        .color(MUTED)
                                        .size(11.0),
                                );
                            });
                            ui.horizontal_wrapped(|ui| {
                                for stat in &relic.substats {
                                    ui.label(RichText::new(stat_text(stat)).size(12.0));
                                }
                            });
                            if let Some(d) = decision {
                                let summary = if d.manual_override.is_some() {
                                    "Manual decision".to_owned()
                                } else {
                                    d.rule_ids
                                        .first()
                                        .and_then(|id| {
                                            rules::registry().into_iter().find(|r| r.id == id)
                                        })
                                        .map_or(String::new(), |r| r.name.to_owned())
                                };
                                ui.label(RichText::new(summary).color(MUTED).size(11.0));
                            }
                        })
                        .response;
                    if ui
                        .interact(
                            response.rect,
                            ui.id().with(("row", index)),
                            egui::Sense::click(),
                        )
                        .clicked()
                    {
                        select = Some(index);
                    }
                }
            });
        if let Some(index) = select {
            self.select(index);
        }
    }
    fn detail(&mut self, ui: &mut egui::Ui) {
        let Some(index) = self.selected else {
            ui.label(RichText::new("Select a relic").size(20.0));
            ui.label(
                RichText::new("Stats, potential and decision evidence at a glance.").color(MUTED),
            );
            return;
        };
        let Some(input) = &self.input else {
            return;
        };
        let relic = &input.archive.relics[index];
        let decision = self.decisions.get(index);
        let mut apply = false;
        let mut remove = false;
        let mut jump_uid = None;
        egui::ScrollArea::vertical().id_salt("detail").show(ui, |ui| {
            ui.label(RichText::new(&relic.name).size(23.0).strong());
            ui.label(RichText::new(format!("{} · +{} · #{}", relic.slot, relic.level, relic.uid)).color(MUTED));
            ui.add_space(8.0);
            ui.label(RichText::new(&relic.mainstat).color(ACCENT).size(19.0));
            ui.label(RichText::new("Main stat type · The export does not include its numeric value").size(11.0).color(MUTED));
            ui.collapsing("Set effect and coverage", |ui| {
                if let Some(set) = rules::set(&relic.set_id) {
                    ui.label(set.effect);
                    ui.label(RichText::new("Activation conditions are not assumed to be satisfied.").size(11.0).color(MUTED));
                } else {
                    ui.label("This set has not been fully reviewed. Automatic discard is blocked.");
                }
            });
            for s in &relic.substats { stat_line(ui, s); }
            if let Some(preview) = &relic.preview_substats && !preview.is_empty() { ui.add_space(8.0); ui.label(RichText::new("Known additional stats · not active yet").color(MUTED)); for s in preview { stat_line(ui, s); } }
            if let Some(reroll) = &relic.reroll_substats && !reroll.is_empty() { ui.collapsing("Reroll alternative (separate)", |ui| { for s in reroll { stat_line(ui, s); } }); }
            ui.separator();
            if let Some(d) = decision {
                ui.label(RichText::new(d.category.label()).color(category_color(d.category)).size(21.0));
                ui.label(&d.explanation);
                ui.label(RichText::new(format!("Best current score: {:.2} roll equivalents", d.score)).color(MUTED));
                ui.add_space(6.0);
                if !d.synergies.is_empty() { ui.label(RichText::new("Detected synergies").strong()); for s in &d.synergies { ui.label(s); } }
                ui.collapsing("Archetypes and upgrade potential", |ui| {
                    for a in &d.archetypes {
                        if !a.mainstat_compatible { continue; }
                        ui.collapsing(format!("{}{} · {:.1}", a.name, if a.set_affinity { " · set affinity" } else { "" }, a.projection.current), |ui| {
                            ui.label(format!("Current {:.2} · {} primary target stats in the known pool", a.projection.current, a.relevant_stats));
                            ui.label(format!("Remaining events: {} (including {} unlock events)", a.projection.remaining_events, a.projection.unlock_events));
                            if let (Some(expected),Some(p90),Some(maximum),Some(probability)) = (a.projection.expected,a.projection.good_p90,a.projection.theoretical_max,a.projection.probability_good) {
                                ui.label(format!("Expected {expected:.2} · good outcome (P90) {p90:.2}"));
                                ui.label(format!("Theoretical maximum {maximum:.2}"));
                                ui.label(format!("Conditional chance of ≥{:.1}: {:.1}%", rules::GOOD_SCORE, probability * 100.0));
                            }
                            ui.label(RichText::new(&a.projection.assumption).size(11.0).color(MUTED));
                        });
                    }
                });
                ui.collapsing("Matched rules", |ui| {
                    for id in &d.rule_ids {
                        if id == "M001" { ui.label("M001 · Manual decision"); continue; }
                        if let Some(rule) = rules::registry().into_iter().find(|r| r.id == id) { ui.collapsing(format!("{} · {}", id, rule.name), |ui| { ui.label(rule.description); ui.label(format!("Priority {} · {:?}", rule.priority, rule.condition)); }); }
                    }
                });
                ui.collapsing(format!("Better comparison relics ({})", d.better_relics.len()), |ui| {
                    if d.better_relics.is_empty() { ui.label("No strictly dominating alternative with identical comparison dimensions was found."); }
                    for uid in &d.better_relics { if ui.link(format!("Relic #{uid}")).clicked() { jump_uid = Some(uid.clone()); } }
                });
                ui.collapsing("Uncertainty and evidence", |ui| { ui.label(&d.evidence); for u in &d.uncertainty { ui.label(format!("• {u}")); } });
                ui.separator();
                ui.label(RichText::new("Your decision").strong());
                ui.label(RichText::new("Preserved when you analyze again. Export the results to save permanently.").color(MUTED).size(12.0));
                egui::ComboBox::from_id_salt("override").selected_text(self.override_category.label()).show_ui(ui, |ui| { for c in Category::ALL { ui.selectable_value(&mut self.override_category, c, c.label()); } });
                ui.add(egui::TextEdit::multiline(&mut self.override_reason).hint_text("Brief reason …").desired_rows(2));
                ui.horizontal(|ui| {
                    apply = ui.add_enabled(self.job.is_none(), egui::Button::new("Apply")).clicked();
                    remove = ui.add_enabled(self.job.is_none() && d.manual_override.is_some(), egui::Button::new("Reset")).clicked();
                });
            } else { ui.label(RichText::new("Analyze to see potential and a recommendation.").color(MUTED)); }
        });
        if apply {
            self.apply_override(false);
        }
        if remove {
            self.apply_override(true);
        }
        if let Some(uid) = jump_uid
            && let Some(index) = self
                .input
                .as_ref()
                .and_then(|i| i.archive.relics.iter().position(|r| r.uid == uid))
        {
            self.select(index);
        }
    }
    fn settings(&mut self, ctx: &egui::Context) {
        let mut open = self.settings;
        let before = self.config.clone();
        let mut analyze_requested = false;
        egui::Window::new("Settings").open(&mut open).default_width(430.0).show(ctx, |ui| {
            if self.job.is_some() { ui.disable(); }
            ui.label("Every result export includes its configuration.");
            ui.add_space(8.0);
            ui.horizontal(|ui| { ui.label("Analysis date"); ui.text_edit_singleline(&mut self.config.analysis_date); });
            ui.horizontal(|ui| { ui.label("Protect new sets (days)"); ui.add(egui::DragValue::new(&mut self.config.new_set_protection_days).range(0..=3650)); });
            ui.label(RichText::new("Time-based protection only uses explicitly supplied release dates. Unreviewed sets remain protected independently.").color(MUTED));
            ui.label(format!("{} verified release dates · {} future profiles · {} protected IDs", self.config.verified_release_dates.len(), self.config.future_profiles.len(), self.config.protected_uids.len()));
            if ui.add_enabled(self.job.is_none(), egui::Button::new("Load configuration JSON")).clicked()
                && let Some(path) = rfd::FileDialog::new().add_filter("Configuration", &["json"]).pick_file() {
                    match std::fs::read(path).map_err(|e| e.to_string()).and_then(|bytes| serde_json::from_slice::<Config>(&bytes).map_err(|e| e.to_string())).and_then(|c| rules::validate_config(&c).map(|()| c)) {
                        Ok(c) => { self.config = c; },
                        Err(e) => self.error = Some(e),
                    }
                }
            if ui.add_enabled(self.job.is_none() && self.input.is_some(), egui::Button::new("Analyze with these settings")).clicked() { analyze_requested = true; }
            ui.separator();
            ui.label(RichText::new("V1 · No game automation or network connection. Result files include private original data; keep them local.").color(MUTED).size(12.0));
        });
        if before != self.config {
            let conflict = self.input.as_ref().and_then(|input| {
                self.overrides.iter().find_map(|(uid, over)| {
                    input
                        .archive
                        .relics
                        .iter()
                        .find(|r| &r.uid == uid)
                        .and_then(|relic| rules::validate_override(relic, &self.config, over).err())
                })
            });
            if let Some(error) = conflict {
                self.config = before;
                self.error = Some(format!(
                    "Configuration not applied: {error} Reset the affected manual decision first."
                ));
                analyze_requested = false;
            } else {
                self.decisions.clear();
                self.filters.category = None;
                self.dirty = true;
                self.unsaved = true;
                self.status = "Configuration changed. Analyze again.".into();
            }
        }
        if analyze_requested {
            self.analyze(ctx);
        }
        self.settings = open;
    }
}

impl eframe::App for RelicSift {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll();
        let dropped: Vec<_> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        if self.job.is_none() && !dropped.is_empty() {
            if dropped.len() == 1 {
                self.load(dropped[0].clone(), ctx);
            } else {
                self.error = Some("Import exactly one inventory or result file at a time.".into());
            }
        }
        self.header(ctx);
        egui::TopBottomPanel::bottom("status")
            .frame(
                egui::Frame::new()
                    .fill(BG)
                    .inner_margin(egui::Margin::symmetric(22, 12)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if self.job.is_some() {
                        ui.spinner();
                    }
                    ui.label(RichText::new(&self.status).color(MUTED).size(12.0));
                });
            });
        if self.input.is_some() {
            egui::SidePanel::right("details")
                .default_width(365.0)
                .min_width(300.0)
                .max_width(600.0)
                .resizable(true)
                .frame(egui::Frame::new().fill(SURFACE).inner_margin(20))
                .show(ctx, |ui| self.detail(ui));
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(BG).inner_margin(22))
            .show(ctx, |ui| {
                if let Some(error) = &self.error {
                    egui::Frame::new()
                        .fill(Color32::from_rgb(56, 35, 43))
                        .corner_radius(8)
                        .inner_margin(12)
                        .show(ui, |ui| {
                            ui.label(error);
                        });
                    ui.add_space(8.0);
                }
                if self.input.is_none() {
                    self.empty(ui, ctx);
                } else {
                    self.dashboard(ui);
                    if let Some(input) = &self.input {
                        ui.collapsing(format!("Import notes ({})", input.warnings.len()), |ui| {
                            for warning in &input.warnings {
                                ui.label(RichText::new(warning).color(MUTED));
                            }
                        });
                    }
                    ui.add_space(6.0);
                    self.list(ui);
                }
            });
        if self.settings {
            self.settings(ctx);
        }
        if self.rules_open {
            egui::Window::new("Scoring rules · the same definitions used by the engine")
                .open(&mut self.rules_open)
                .default_size([760.0, 620.0])
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for rule in rules::registry() {
                            ui.collapsing(
                                format!("{} · {} · {}", rule.id, rule.name, rule.category.label()),
                                |ui| {
                                    ui.label(rule.description);
                                    ui.label(rule.scope);
                                    ui.label(format!(
                                        "Priority {} · {:?}",
                                        rule.priority, rule.condition
                                    ));
                                    ui.label(format!("Example: {}", rule.example));
                                    ui.label(format!("Exceptions: {}", rule.exceptions));
                                },
                            );
                        }
                    });
                });
        }
    }
}

fn stat_text(s: &Substat) -> String {
    format!(
        "{} {:.2}{}",
        s.key.trim_end_matches('_'),
        s.value,
        if s.key.ends_with('_') { "%" } else { "" }
    )
}
fn stat_line(ui: &mut egui::Ui, s: &Substat) {
    ui.horizontal(|ui| {
        ui.label(stat_text(s));
        ui.label(
            RichText::new(format!(
                "{} contributions · quality step {}",
                s.count.map_or("?".into(), |v| v.to_string()),
                s.step.map_or("?".into(), |v| v.to_string())
            ))
            .color(MUTED)
            .size(11.0),
        );
    });
}
fn string_combo(
    ui: &mut egui::Ui,
    id: &str,
    empty: &str,
    selected: &mut String,
    options: &[String],
) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(if selected.is_empty() {
            empty
        } else {
            selected.as_str()
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(selected, String::new(), empty);
            for value in options {
                ui.selectable_value(selected, value.clone(), value);
            }
        });
}
