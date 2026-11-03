//! eqLinux main application window.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use eframe::egui::{self, Align, Align2, Layout, RichText, Vec2};

use crate::audio::{AudioParams, EngineHandle};
use crate::autoeq;
use crate::config::{Accent, Config, SuperPreset};
use crate::dsp::{Channel, EqMode, FilterType, SpatialMode, MAX_GAIN, MIN_GAIN};
use crate::i18n::Language;
use crate::mixer;
use crate::presets::{self, AdvancedPreset, BasicPreset, ExpertPreset};
use crate::render;
use crate::routing::{self, Routing, SinkInfo};
use crate::theme::{self, PANEL_DARK, TEXT, TEXT_DIM, WARNING};
use crate::tray::{TrayAction, TrayEvent};
use crate::widgets::{self, Icon};

const FREQUENCY_LABELS: [&str; 10] = [
    "32", "64", "125", "250", "500", "1K", "2K", "4K", "8K", "16K",
];
const PAYPAL_URL: &str = "https://www.paypal.com/paypalme/etovivePr";

struct SaveDialog {
    name: String,
    mode: EqMode,
}

pub struct EqLinuxApp {
    config: Config,
    engine: Option<EngineHandle>,
    routing: Option<Routing>,
    sinks: Vec<SinkInfo>,
    selected_sink: Option<String>,
    status: String,
    show_settings: bool,
    show_help: bool,
    save_dialog: Option<SaveDialog>,
    dirty_at: Option<Instant>,
    last_sink_refresh: Instant,
    spectrum: [f32; 32],
    toast: Option<(String, Instant)>,
    quit_requested: bool,
    no_route: bool,
    started_at: Instant,
    quit_after: Option<Duration>,
    last_content_height: f32,
    app_mixer: Vec<mixer::SinkInput>,
    last_mixer_refresh: Instant,
    autoeq_path: String,
    render_input: String,
    render_output: String,
    recording_path: Option<PathBuf>,
    tray_events: Receiver<TrayEvent>,
}

impl EqLinuxApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        no_route: bool,
        quit_after: Option<Duration>,
        tray_events: Receiver<TrayEvent>,
    ) -> Self {
        let config = Config::load();
        theme::set_accent(config.accent.rgb());
        theme::configure(&cc.egui_ctx);

        let mut app = Self {
            config,
            engine: None,
            routing: None,
            sinks: Vec::new(),
            selected_sink: None,
            status: "Starting…".into(),
            show_settings: std::env::var_os("EQLINUX_OPEN_SETTINGS").is_some(),
            show_help: false,
            save_dialog: None,
            dirty_at: None,
            last_sink_refresh: Instant::now(),
            spectrum: [0.0; 32],
            toast: None,
            quit_requested: false,
            no_route,
            started_at: Instant::now(),
            quit_after,
            last_content_height: 0.0,
            app_mixer: Vec::new(),
            last_mixer_refresh: Instant::now(),
            autoeq_path: String::new(),
            render_input: String::new(),
            render_output: String::new(),
            recording_path: None,
            tray_events,
        };

        app.refresh_sinks();
        app.pick_initial_output();

        if app.config.routing_enabled && !app.no_route {
            match Routing::start(&app.config.virtual_sink) {
                Ok(routing) => app.routing = Some(routing),
                Err(err) => {
                    app.status = format!("Routing error: {err}");
                }
            }
        } else {
            app.status = "System routing disabled".into();
        }

        app.start_engine();
        app
    }

    fn pick_initial_output(&mut self) {
        let configured = self
            .config
            .output_sink
            .clone()
            .filter(|name| self.is_real_sink(name));

        self.selected_sink = configured
            .or_else(|| routing::default_sink().filter(|name| self.is_real_sink(name)))
            .or_else(|| {
                self.sinks
                    .iter()
                    .find(|s| s.name != self.config.virtual_sink)
                    .map(|s| s.name.clone())
            });

        if let Some(sink) = &self.selected_sink {
            self.config.output_sink = Some(sink.clone());
            let sink = sink.clone();
            self.apply_super_preset(&sink);
        }
    }

    fn is_real_sink(&self, name: &str) -> bool {
        name != self.config.virtual_sink
            && !routing::is_virtual_sink(name)
            && self.sinks.iter().any(|s| s.name == name)
    }

    fn refresh_sinks(&mut self) {
        match routing::list_sinks() {
            Ok(sinks) => {
                self.sinks = sinks
                    .into_iter()
                    .filter(|sink| !routing::is_virtual_sink(&sink.name))
                    .collect();
                if self
                    .selected_sink
                    .as_ref()
                    .is_none_or(|name| !self.is_real_sink(name))
                {
                    let fallback = routing::default_sink()
                        .filter(|name| self.is_real_sink(name))
                        .or_else(|| {
                            self.sinks
                                .iter()
                                .find(|s| s.name != self.config.virtual_sink)
                                .map(|s| s.name.clone())
                        });

                    if fallback != self.selected_sink {
                        self.selected_sink = fallback.clone();
                        if let Some(name) = fallback {
                            self.config.output_sink = Some(name);
                            self.dirty_at = Some(Instant::now());
                        }
                    }
                }
            }
            Err(err) => self.status = format!("Could not list outputs: {err}"),
        }
    }

    fn tr(&self, english: &'static str, spanish: &'static str) -> &'static str {
        self.config.language.t(english, spanish)
    }

    fn spanish(&self) -> bool {
        self.config.language == Language::Spanish
    }

    fn status_display(&self) -> String {
        if !self.spanish() {
            return self.status.clone();
        }
        let status = self.status.as_str();
        let replacements = [
            ("Starting…", "Iniciando…"),
            (
                "System routing disabled by EQLINUX_NO_ROUTE",
                "Enrutado global desactivado por EQLINUX_NO_ROUTE",
            ),
            ("System routing disabled", "Enrutado global desactivado"),
            ("No output device available", "No hay salida disponible"),
            (
                "Could not list outputs:",
                "No se pudieron listar las salidas:",
            ),
            ("Routing error:", "Error de enrutado:"),
            ("App mixer error:", "Error del mezclador:"),
            ("Recording error:", "Error de grabación:"),
            ("Running · eqLinux →", "Activo · eqLinux →"),
            ("Running ·", "Activo ·"),
        ];
        for (english, spanish) in replacements {
            if let Some(rest) = status.strip_prefix(english) {
                return format!("{spanish}{rest}");
            }
        }
        status.to_string()
    }

    fn build_params(&self) -> AudioParams {
        AudioParams {
            enabled: self.config.app_enabled,
            eq_enabled: self.config.eq_enabled,
            eq: self.config.eq_params(),
            volume: self.config.effective_volume(),
            balance: self.config.balance,
            muted: self.config.muted,
        }
    }

    fn publish(&mut self) {
        let params = self.build_params();
        if let Some(engine) = &self.engine {
            engine.update(params);
        }
        self.dirty_at = Some(Instant::now());
    }

    fn mark_dirty(&mut self) {
        self.dirty_at = Some(Instant::now());
    }

    fn set_start_minimized_on_startup(&mut self, enabled: bool) -> Result<(), String> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| "Could not locate the user config directory".to_string())?;
        let autostart_path = config_dir.join("autostart/eqlinux.desktop");

        if enabled {
            let executable = std::env::current_exe()
                .map_err(|error| format!("Could not locate eqLinux: {error}"))?;
            let desktop_entry = format!(
                "[Desktop Entry]\nType=Application\nName=eqLinux\nComment=Audio equalizer\nExec={} --minimized\nTerminal=false\n",
                desktop_exec_argument(&executable)
            );
            let parent = autostart_path
                .parent()
                .ok_or_else(|| "Could not locate the autostart directory".to_string())?;
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            fs::write(&autostart_path, desktop_entry).map_err(|error| error.to_string())?;
        } else {
            match fs::remove_file(autostart_path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.to_string()),
            }
        }

        self.config.start_minimized_on_startup = enabled;
        self.mark_dirty();
        Ok(())
    }

    fn start_engine(&mut self) {
        self.engine.take();

        let Some(routing) = &self.routing else {
            self.status = "System routing disabled".into();
            return;
        };
        let Some(sink) = self.selected_sink.clone() else {
            self.status = "No output device available".into();
            return;
        };

        let monitor = routing.monitor_source().to_string();
        let params = self.build_params();
        self.engine = Some(EngineHandle::start(monitor, sink.clone(), params));
        self.status = format!("Running · eqLinux → {sink}");
    }

    fn restart_engine(&mut self) {
        self.start_engine();
    }

    fn set_output(&mut self, name: String) {
        self.selected_sink = Some(name.clone());
        self.config.output_sink = Some(name.clone());
        self.mark_dirty();
        self.apply_super_preset(&name);
        self.restart_engine();
    }

    fn apply_super_preset(&mut self, output: &str) {
        if !self.config.super_presets_enabled {
            return;
        }
        let Some(rule) = self
            .config
            .super_presets
            .iter()
            .find(|rule| rule.output_sink == output)
            .cloned()
        else {
            return;
        };
        self.config.eq_mode = rule.mode;
        self.select_preset(&rule.preset_id);
    }

    fn set_routing(&mut self, enabled: bool) {
        if self.no_route {
            self.status = "System routing disabled by EQLINUX_NO_ROUTE".into();
            return;
        }
        self.config.routing_enabled = enabled;
        self.mark_dirty();

        if enabled {
            if self.routing.is_none() {
                match Routing::start(&self.config.virtual_sink) {
                    Ok(routing) => {
                        self.routing = Some(routing);
                        self.start_engine();
                    }
                    Err(err) => self.status = format!("Routing error: {err}"),
                }
            }
        } else {
            self.engine.take();
            if let Some(mut routing) = self.routing.take() {
                routing.stop();
            }
            self.status = "System routing disabled".into();
        }
    }

    fn shutdown(&mut self) {
        self.engine.take();
        if let Some(mut routing) = self.routing.take() {
            routing.stop();
        }
        let _ = self.config.save();
    }

    fn basic_presets(&self) -> Vec<BasicPreset> {
        let mut list = presets::ordered_basic(&self.config.user_basic_presets);
        if let Some(manual) = list.iter_mut().find(|p| p.id == "manual") {
            manual.bass = self.config.basic.bass;
            manual.mid = self.config.basic.mid;
            manual.treble = self.config.basic.treble;
            manual.peak_limiter = self.config.basic.peak_limiter;
        }
        list
    }

    fn advanced_presets(&self) -> Vec<AdvancedPreset> {
        let mut list = presets::ordered_advanced(
            &self.config.user_advanced_presets,
            self.config.advanced.show_default_presets,
        );
        if let Some(manual) = list.iter_mut().find(|p| p.id == "manual") {
            manual.global = self.config.advanced.global;
            manual.bands = self.config.advanced.bands;
        }
        list
    }

    fn expert_presets(&self) -> Vec<ExpertPreset> {
        let mut list = presets::ordered_expert(
            &self.config.user_expert_presets,
            self.config.expert.show_default_presets,
        );
        if let Some(manual) = list.iter_mut().find(|p| p.id == "manual") {
            manual.global = self.config.expert.global;
            manual.bands = self.config.expert.bands.clone();
            manual.peak_limiter = self.config.expert.peak_limiter;
        }
        list
    }

    fn selected_basic_name(&self) -> String {
        self.basic_presets()
            .iter()
            .find(|p| p.id == self.config.basic.selected_preset)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "Manual".into())
    }

    fn selected_advanced_name(&self) -> String {
        self.advanced_presets()
            .iter()
            .find(|p| p.id == self.config.advanced.selected_preset)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "Manual".into())
    }

    fn selected_expert_name(&self) -> String {
        self.expert_presets()
            .iter()
            .find(|p| p.id == self.config.expert.selected_preset)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "Manual".into())
    }

    fn preset_display_name(&self, mode: EqMode, id: &str) -> String {
        match mode {
            EqMode::Basic => self
                .basic_presets()
                .iter()
                .find(|preset| preset.id == id)
                .map(|preset| preset.name.clone()),
            EqMode::Advanced => self
                .advanced_presets()
                .iter()
                .find(|preset| preset.id == id)
                .map(|preset| preset.name.clone()),
            EqMode::Expert => self
                .expert_presets()
                .iter()
                .find(|preset| preset.id == id)
                .map(|preset| preset.name.clone()),
        }
        .unwrap_or_else(|| "Manual".into())
    }

    fn select_preset(&mut self, id: &str) {
        match self.config.eq_mode {
            EqMode::Basic => {
                let list = self.basic_presets();
                if let Some(preset) = list.iter().find(|p| p.id == id) {
                    if preset.id != "manual" {
                        self.config.basic.bass = preset.bass;
                        self.config.basic.mid = preset.mid;
                        self.config.basic.treble = preset.treble;
                        self.config.basic.peak_limiter = preset.peak_limiter;
                    }
                    self.config.basic.selected_preset = preset.id.clone();
                }
            }
            EqMode::Advanced => {
                let list = self.advanced_presets();
                if let Some(preset) = list.iter().find(|p| p.id == id) {
                    if preset.id != "manual" {
                        self.config.advanced.global = preset.global;
                        self.config.advanced.bands = preset.bands;
                    }
                    self.config.advanced.selected_preset = preset.id.clone();
                }
            }
            EqMode::Expert => {
                let list = self.expert_presets();
                if let Some(preset) = list.iter().find(|p| p.id == id) {
                    if preset.id != "manual" {
                        self.config.expert.global = preset.global;
                        self.config.expert.bands = preset.bands.clone();
                        self.config.expert.peak_limiter = preset.peak_limiter;
                    }
                    self.config.expert.selected_preset = preset.id.clone();
                }
            }
        }
        self.publish();
    }

    fn apply_hs55_profile(&mut self) {
        let Some(preset) = presets::advanced_defaults()
            .into_iter()
            .find(|preset| preset.name == "Corsair HS55 (starting point)")
        else {
            self.status = "Built-in Corsair HS55 profile is unavailable".into();
            return;
        };

        self.config.eq_mode = EqMode::Advanced;
        self.config.eq_enabled = true;
        self.config.advanced.global = preset.global;
        self.config.advanced.bands = preset.bands;
        self.config.advanced.selected_preset = preset.id;
        self.publish();
        self.mark_dirty();
        self.toast = Some((
            self.config
                .language
                .t(
                    "Corsair HS55 starting profile applied",
                    "Perfil inicial Corsair HS55 aplicado",
                )
                .into(),
            Instant::now(),
        ));
    }

    fn delete_preset(&mut self) {
        match self.config.eq_mode {
            EqMode::Basic => {
                let id = self.config.basic.selected_preset.clone();
                let before = self.config.user_basic_presets.len();
                self.config.user_basic_presets.retain(|p| p.id != id);
                if self.config.user_basic_presets.len() != before {
                    self.select_preset("flat");
                }
            }
            EqMode::Advanced => {
                let id = self.config.advanced.selected_preset.clone();
                let before = self.config.user_advanced_presets.len();
                self.config.user_advanced_presets.retain(|p| p.id != id);
                if self.config.user_advanced_presets.len() != before {
                    self.select_preset("flat");
                }
            }
            EqMode::Expert => {
                let id = self.config.expert.selected_preset.clone();
                let before = self.config.user_expert_presets.len();
                self.config.user_expert_presets.retain(|p| p.id != id);
                if self.config.user_expert_presets.len() != before {
                    self.select_preset("flat");
                }
            }
        }
    }

    fn is_selected_user_preset(&self) -> bool {
        match self.config.eq_mode {
            EqMode::Basic => self
                .config
                .user_basic_presets
                .iter()
                .any(|p| p.id == self.config.basic.selected_preset),
            EqMode::Advanced => self
                .config
                .user_advanced_presets
                .iter()
                .any(|p| p.id == self.config.advanced.selected_preset),
            EqMode::Expert => self
                .config
                .user_expert_presets
                .iter()
                .any(|p| p.id == self.config.expert.selected_preset),
        }
    }

    fn save_preset(&mut self, name: &str, mode: EqMode) {
        let id = format!(
            "user-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        );

        match mode {
            EqMode::Basic => {
                let preset = BasicPreset {
                    id: id.clone(),
                    name: name.to_string(),
                    is_default: false,
                    peak_limiter: self.config.basic.peak_limiter,
                    bass: self.config.basic.bass,
                    mid: self.config.basic.mid,
                    treble: self.config.basic.treble,
                };

                if let Some(existing) = self
                    .config
                    .user_basic_presets
                    .iter_mut()
                    .find(|p| p.name.eq_ignore_ascii_case(name))
                {
                    let existing_id = existing.id.clone();
                    *existing = BasicPreset {
                        id: existing_id.clone(),
                        ..preset
                    };
                    self.config.basic.selected_preset = existing_id;
                } else {
                    self.config.user_basic_presets.push(preset);
                    self.config.basic.selected_preset = id;
                }
            }
            EqMode::Advanced => {
                let preset = AdvancedPreset {
                    id: id.clone(),
                    name: name.to_string(),
                    is_default: false,
                    global: self.config.advanced.global,
                    bands: self.config.advanced.bands,
                };

                if let Some(existing) = self
                    .config
                    .user_advanced_presets
                    .iter_mut()
                    .find(|p| p.name.eq_ignore_ascii_case(name))
                {
                    let existing_id = existing.id.clone();
                    *existing = AdvancedPreset {
                        id: existing_id.clone(),
                        ..preset
                    };
                    self.config.advanced.selected_preset = existing_id;
                } else {
                    self.config.user_advanced_presets.push(preset);
                    self.config.advanced.selected_preset = id;
                }
            }
            EqMode::Expert => {
                let preset = ExpertPreset {
                    id: id.clone(),
                    name: name.to_string(),
                    global: self.config.expert.global,
                    bands: self.config.expert.bands.clone(),
                    peak_limiter: self.config.expert.peak_limiter,
                };

                if let Some(existing) = self
                    .config
                    .user_expert_presets
                    .iter_mut()
                    .find(|p| p.name.eq_ignore_ascii_case(name))
                {
                    let existing_id = existing.id.clone();
                    *existing = ExpertPreset {
                        id: existing_id.clone(),
                        ..preset
                    };
                    self.config.expert.selected_preset = existing_id;
                } else {
                    self.config.user_expert_presets.push(preset);
                    self.config.expert.selected_preset = id;
                }
            }
        }

        self.publish();
        self.toast = Some(("Preset saved".into(), Instant::now()));
    }

    // ---------------------------------------------------------------- Interfaz

    fn header(&mut self, ui: &mut egui::Ui) {
        let language = self.config.language;
        let tr = |english: &'static str, spanish: &'static str| language.t(english, spanish);
        let mut header_rect =
            egui::Rect::from_min_size(ui.cursor().min, Vec2::new(ui.available_width(), 30.0));
        header_rect.min.x -= 12.0;
        header_rect.max.x += 12.0;
        header_rect.min.y -= 6.0;
        header_rect.max.y -= 6.0;
        ui.painter().rect_filled(header_rect, 0.0, theme::HEADER_BG);
        let drag = ui.interact(
            header_rect,
            egui::Id::unique("eqlinux-header-drag"),
            egui::Sense::drag(),
        );
        if drag.drag_started() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }

        let mut toggle_recording = false;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;

            if widgets::header_button(ui, Icon::Close, true, 12.0)
                .on_hover_text(tr("Close", "Cerrar"))
                .clicked()
            {
                self.quit_requested = true;
            }
            if widgets::header_button(ui, Icon::Minus, true, 12.0)
                .on_hover_text(tr("Minimize", "Minimizar"))
                .clicked()
            {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Visible(false));
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }

            ui.add_space(2.0);
            let mut enabled = self.config.app_enabled;
            if widgets::toggle(ui, &mut enabled, true)
                .on_hover_text(tr("eqLinux Bypass", "Anular eqLinux"))
                .changed()
            {
                self.config.app_enabled = enabled;
                self.publish();
            }

            let center = ui.max_rect().center().x;
            let cursor = ui.cursor().min.x;
            let title_width = 78.0;
            let space = (center - title_width * 0.5 - cursor).max(0.0);
            ui.add_space(space);
            ui.label(RichText::new("eqLinux").size(12.0).strong().color(TEXT));
            egui::Frame::new()
                .fill(PANEL_DARK)
                .corner_radius(4.0)
                .inner_margin(egui::Margin::symmetric(4, 1))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("ALfa")
                            .size(10.0)
                            .strong()
                            .color(theme::accent()),
                    );
                });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if widgets::header_button(ui, Icon::Power, true, 16.0)
                    .on_hover_text(tr("Quit", "Salir"))
                    .clicked()
                {
                    self.quit_requested = true;
                }
                if widgets::header_button(ui, Icon::Gear, true, 16.0)
                    .on_hover_text(tr("Settings", "Ajustes"))
                    .clicked()
                {
                    self.show_settings = !self.show_settings;
                }
                if widgets::header_button(ui, Icon::Record, self.engine.is_some(), 16.0)
                    .on_hover_text(self.tr("Record processed audio", "Grabar audio procesado"))
                    .clicked()
                {
                    toggle_recording = true;
                }
                if widgets::header_button(ui, Icon::Help, true, 16.0)
                    .on_hover_text(self.tr("About", "Acerca de"))
                    .clicked()
                {
                    self.show_help = !self.show_help;
                }
            });
        });

        if toggle_recording {
            if let Some(engine) = &self.engine {
                if engine.is_recording() {
                    if let Some(path) = engine.stop_recording() {
                        self.recording_path = Some(path.clone());
                        self.toast = Some((
                            format!(
                                "{}: {}",
                                self.tr("Recording saved", "Grabación guardada"),
                                path.display()
                            ),
                            Instant::now(),
                        ));
                    }
                } else {
                    match engine.start_recording() {
                        Ok(path) => {
                            self.recording_path = Some(path.clone());
                            self.toast = Some((
                                format!("{}: {}", self.tr("Recording", "Grabando"), path.display()),
                                Instant::now(),
                            ));
                        }
                        Err(err) => {
                            self.status =
                                format!("{}: {err}", tr("Recording error", "Error de grabación"));
                        }
                    }
                }
            }
        }

        ui.add_space(6.0);
    }

    fn volume_section(&mut self, ui: &mut egui::Ui) {
        let show_volume = self.config.ui.show_volume;
        let show_balance = self.config.ui.show_balance;
        let enabled = self.config.app_enabled;
        // Volume and balance use knobs in every equalizer mode.
        let use_sliders = false;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let total = ui.available_width();
            let columns = show_volume as usize + show_balance as usize;
            let dividers = columns.saturating_sub(1) as f32;
            let divider_spacing = ui.spacing().item_spacing.x * 2.0;
            let column_width = (total - dividers * (7.0 + divider_spacing)) / columns.max(1) as f32;
            let divider_x = ui.max_rect().center().x;

            if show_volume {
                ui.allocate_ui_with_layout(
                    Vec2::new(column_width, 82.0),
                    Layout::top_down(Align::Center),
                    |ui| self.volume_controls(ui, enabled, use_sliders),
                );
            }
            if show_volume && show_balance {
                widgets::vertical_divider(ui, 82.0, divider_x);
            }
            if show_balance {
                ui.allocate_ui_with_layout(
                    Vec2::new(column_width, 82.0),
                    Layout::top_down(Align::Center),
                    |ui| self.balance_controls(ui, enabled, use_sliders),
                );
            }
        });
    }

    fn volume_controls(&mut self, ui: &mut egui::Ui, enabled: bool, use_sliders: bool) {
        let language = self.config.language;
        let tr = |english: &'static str, spanish: &'static str| language.t(english, spanish);
        let mut volume = self.config.volume as f64;
        let mut boost = self.config.boost_enabled;
        let max = if boost { 2.0 } else { 1.0 };
        let mut changed = false;

        if use_sliders {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    theme::small_label(ui, tr("Volume", "Volumen"));
                    widgets::value_screen(
                        ui,
                        &format!("{:.0}%", (volume.min(1.0) * 100.0).round()),
                        enabled,
                        44.0,
                    );
                });
                if widgets::flat_slider(ui, &mut volume, 0.0, max, 76.0, 1.0, boost, enabled)
                    .changed()
                {
                    changed = true;
                }
                ui.vertical(|ui| {
                    if widgets::check_box(ui, &mut boost, enabled).changed() {
                        if !boost && volume > 1.0 {
                            volume = 1.0;
                        }
                        changed = true;
                    }
                    theme::small_label(ui, tr("Boost", "Refuerzo"));
                    widgets::value_screen(
                        ui,
                        &format!("{:.0}%", (((volume - 1.0).max(0.0)) * 100.0).round()),
                        boost && enabled,
                        44.0,
                    );
                });
            });
        } else {
            let group_width = 44.0 * 2.0 + 42.0 + ui.spacing().item_spacing.x * 2.0;
            ui.allocate_ui_with_layout(
                Vec2::new(group_width, ui.available_height()),
                Layout::left_to_right(Align::Center),
                |ui| {
                    ui.vertical(|ui| {
                        theme::small_label(ui, tr("Volume", "Volumen"));
                        widgets::value_screen(
                            ui,
                            &format!("{:.0}%", (volume.min(1.0) * 100.0).round()),
                            enabled,
                            44.0,
                        );
                    });
                    if widgets::knob(ui, &mut volume, 0.0, max, 42.0, 1.0, enabled).changed() {
                        changed = true;
                    }
                    ui.vertical(|ui| {
                        if widgets::check_box(ui, &mut boost, enabled).changed() {
                            if !boost && volume > 1.0 {
                                volume = 1.0;
                            }
                            changed = true;
                        }
                        theme::small_label(ui, tr("Boost", "Refuerzo"));
                        widgets::value_screen(
                            ui,
                            &format!("{:.0}%", (((volume - 1.0).max(0.0)) * 100.0).round()),
                            boost && enabled,
                            44.0,
                        );
                    });
                },
            );
        }

        if changed {
            self.config.volume = volume as f32;
            self.config.boost_enabled = boost;
            self.publish();
        }
    }

    fn balance_controls(&mut self, ui: &mut egui::Ui, enabled: bool, use_sliders: bool) {
        let language = self.config.language;
        let tr = |english: &'static str, spanish: &'static str| language.t(english, spanish);
        let mut balance = self.config.balance as f64;
        let mut changed = false;

        let left = if balance > 0.0 {
            (1.0 - balance) * 100.0
        } else {
            100.0
        };
        let right = if balance < 0.0 {
            (1.0 + balance) * 100.0
        } else {
            100.0
        };

        if use_sliders {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    theme::small_label(ui, tr("Left", "Izq."));
                    widgets::value_screen(ui, &format!("{left:.0}%"), enabled, 44.0);
                });
                if widgets::flat_slider(ui, &mut balance, -1.0, 1.0, 76.0, 0.0, true, enabled)
                    .changed()
                {
                    changed = true;
                }
                ui.vertical(|ui| {
                    theme::small_label(ui, tr("Right", "Der."));
                    widgets::value_screen(ui, &format!("{right:.0}%"), enabled, 44.0);
                });
            });
        } else {
            let group_width = 44.0 * 2.0 + 42.0 + ui.spacing().item_spacing.x * 2.0;
            let section_rect = ui.max_rect();
            let group_rect = egui::Rect::from_center_size(
                section_rect.center() + Vec2::new(15.0, 0.0),
                Vec2::new(group_width, section_rect.height()),
            );
            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(group_rect)
                    .layout(Layout::left_to_right(Align::Center)),
                |ui| {
                    ui.vertical(|ui| {
                        theme::small_label(ui, tr("Left", "Izq."));
                        widgets::value_screen(ui, &format!("{left:.0}%"), enabled, 44.0);
                    });
                    if widgets::knob(ui, &mut balance, -1.0, 1.0, 42.0, 0.0, enabled).changed() {
                        changed = true;
                    }
                    ui.vertical(|ui| {
                        theme::small_label(ui, tr("Right", "Der."));
                        widgets::value_screen(ui, &format!("{right:.0}%"), enabled, 44.0);
                    });
                },
            );
        }

        if changed {
            self.config.balance = balance as f32;
            self.publish();
        }
    }

    fn equalizer_section(&mut self, ui: &mut egui::Ui) {
        let language = self.config.language;
        let tr = |english: &'static str, spanish: &'static str| language.t(english, spanish);
        let app_enabled = self.config.app_enabled;
        let mut eq_enabled = self.config.eq_enabled;
        let mut mode = self.config.eq_mode;
        let mut collapsed = self.config.ui.equalizer_collapsed;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;

            if widgets::toggle(ui, &mut eq_enabled, app_enabled)
                .on_hover_text(tr("Enable equalizer", "Activar ecualizador"))
                .changed()
            {
                self.config.eq_enabled = eq_enabled;
                self.publish();
            }

            theme::small_label(ui, tr("Equalizer:", "Ecualizador:"));

            if widgets::tab(
                ui,
                tr("Basic", "Básico"),
                mode == EqMode::Basic,
                app_enabled,
            )
            .clicked()
                && mode != EqMode::Basic
            {
                mode = EqMode::Basic;
            }
            if widgets::tab(
                ui,
                tr("Advanced", "Avanzado"),
                mode == EqMode::Advanced,
                app_enabled,
            )
            .clicked()
                && mode != EqMode::Advanced
            {
                mode = EqMode::Advanced;
            }
            if widgets::tab(
                ui,
                tr("Expert", "Experto"),
                mode == EqMode::Expert,
                app_enabled,
            )
            .clicked()
                && mode != EqMode::Expert
            {
                mode = EqMode::Expert;
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let icon = if collapsed { Icon::Eye } else { Icon::EyeOff };
                if widgets::icon_button(ui, icon, true)
                    .on_hover_text(if collapsed {
                        tr("Show", "Mostrar")
                    } else {
                        tr("Hide", "Ocultar")
                    })
                    .clicked()
                {
                    collapsed = !collapsed;
                    self.config.ui.equalizer_collapsed = collapsed;
                    self.mark_dirty();
                }
            });
        });

        if mode != self.config.eq_mode {
            self.config.eq_mode = mode;
            self.publish();
        }

        if collapsed {
            return;
        }

        self.preset_row(ui);

        match self.config.eq_mode {
            EqMode::Basic => self.basic_equalizer(ui),
            EqMode::Advanced => self.advanced_equalizer(ui),
            EqMode::Expert => self.expert_equalizer(ui),
        }
    }

    fn preset_row(&mut self, ui: &mut egui::Ui) {
        let language = self.config.language;
        let tr = |english: &'static str, spanish: &'static str| language.t(english, spanish);
        let mode = self.config.eq_mode;
        let is_user = self.is_selected_user_preset();
        let enabled = self.config.app_enabled && self.config.eq_enabled;
        let mut chosen: Option<String> = None;
        let mut open_save = false;
        let mut apply_headphones = false;
        let mut toggle_spatial = false;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;

            if widgets::square_button(ui, Icon::Minus, enabled && is_user)
                .on_hover_text(tr("Remove Preset", "Eliminar preset"))
                .clicked()
            {
                self.delete_preset();
            }

            match mode {
                EqMode::Basic => {
                    let list = self.basic_presets();
                    let selected = self.selected_basic_name();
                    egui::ComboBox::from_id_salt("basic-preset")
                        .width(214.0)
                        .selected_text(presets::display_name(
                            &selected,
                            language == Language::Spanish,
                        ))
                        .show_ui(ui, |ui| {
                            for preset in &list {
                                if ui
                                    .selectable_label(
                                        preset.name == selected,
                                        presets::display_name(
                                            &preset.name,
                                            language == Language::Spanish,
                                        ),
                                    )
                                    .clicked()
                                {
                                    chosen = Some(preset.id.clone());
                                }
                            }
                        });
                }
                EqMode::Advanced => {
                    let list = self.advanced_presets();
                    let selected = self.selected_advanced_name();
                    egui::ComboBox::from_id_salt("advanced-preset")
                        .width(214.0)
                        .selected_text(presets::display_name(
                            &selected,
                            language == Language::Spanish,
                        ))
                        .show_ui(ui, |ui| {
                            for preset in &list {
                                if ui
                                    .selectable_label(
                                        preset.name == selected,
                                        presets::display_name(
                                            &preset.name,
                                            language == Language::Spanish,
                                        ),
                                    )
                                    .clicked()
                                {
                                    chosen = Some(preset.id.clone());
                                }
                            }
                        });
                }
                EqMode::Expert => {
                    let list = self.expert_presets();
                    let selected = self.selected_expert_name();
                    egui::ComboBox::from_id_salt("expert-preset")
                        .width(214.0)
                        .selected_text(presets::display_name(
                            &selected,
                            language == Language::Spanish,
                        ))
                        .show_ui(ui, |ui| {
                            for preset in &list {
                                if ui
                                    .selectable_label(
                                        preset.name == selected,
                                        presets::display_name(
                                            &preset.name,
                                            language == Language::Spanish,
                                        ),
                                    )
                                    .clicked()
                                {
                                    chosen = Some(preset.id.clone());
                                }
                            }
                        });
                }
            }

            if widgets::square_button(ui, Icon::Plus, enabled)
                .on_hover_text(tr("Save Preset", "Guardar preset"))
                .clicked()
            {
                open_save = true;
            }
            if ui
                .button("HS55")
                .on_hover_text(tr(
                    "Apply the Corsair HS55 headphone profile",
                    "Aplicar el perfil de auriculares Corsair HS55",
                ))
                .clicked()
            {
                apply_headphones = true;
            }
            if ui
                .selectable_label(
                    self.config.spatial == SpatialMode::Headphones3D,
                    tr("3D", "3D"),
                )
                .on_hover_text(tr(
                    "Toggle headphone spatial audio",
                    "Activar o desactivar audio espacial para auriculares",
                ))
                .clicked()
            {
                toggle_spatial = true;
            }
        });

        if let Some(id) = chosen {
            self.select_preset(&id);
        }
        if apply_headphones {
            self.apply_hs55_profile();
        }
        if toggle_spatial {
            if self.config.spatial == SpatialMode::Headphones3D {
                self.config.spatial = SpatialMode::Off;
            } else {
                self.config.spatial = SpatialMode::Headphones3D;
                self.config.spatial_mix = 1.0;
            }
            self.publish();
            self.mark_dirty();
        }
        if open_save {
            let name = match mode {
                EqMode::Basic => self.selected_basic_name(),
                EqMode::Advanced => self.selected_advanced_name(),
                EqMode::Expert => self.selected_expert_name(),
            };
            self.save_dialog = Some(SaveDialog { name, mode });
        }

        ui.add_space(2.0);
    }

    fn basic_equalizer(&mut self, ui: &mut egui::Ui) {
        let language = self.config.language;
        let tr = |english: &'static str, spanish: &'static str| language.t(english, spanish);
        let enabled = self.config.app_enabled && self.config.eq_enabled;
        let use_sliders = self.config.ui.use_sliders;
        let mut peak_limiter = self.config.basic.peak_limiter;
        let mut bass = self.config.basic.bass;
        let mut mid = self.config.basic.mid;
        let mut treble = self.config.basic.treble;
        let mut reverb = if matches!(
            self.config.spatial,
            SpatialMode::Off | SpatialMode::Headphones3D
        ) {
            0.0
        } else {
            self.config.spatial_mix as f64
        };
        let mut changed = false;
        let mut reverb_changed = false;

        ui.horizontal(|ui| {
            let available = ui.available_width();
            ui.add_space(((available - 110.0) * 0.5).max(0.0));
            if widgets::labeled_checkbox(ui, &mut peak_limiter, tr("Peak Limiter", "Limitador"), enabled).changed()
            {
                changed = true;
            }
            widgets::help_badge(
                ui,
                tr("Enable if you hear distortion when increasing Gain.\nThis decreases the Global Gain so the sound never clips.\nThen increase the overall volume to compensate.", "Actívalo si oyes distorsión al subir la ganancia.\nReduce la ganancia global para que el sonido no sature.\nDespués sube el volumen general para compensar."),
            );
        });

        if use_sliders {
            ui.add_space(4.0);
            changed |= basic_slider_row(ui, tr("BASS", "GRAVES"), &mut bass, enabled);
            changed |= basic_slider_row(ui, tr("MID", "MEDIOS"), &mut mid, enabled);
            changed |= basic_slider_row(ui, tr("TREBLE", "AGUDOS"), &mut treble, enabled);
            reverb_changed |=
                basic_amount_slider_row(ui, tr("REVERB", "REVERB"), &mut reverb, enabled);
        } else {
            ui.add_space(2.0);
            ui.columns(4, |columns| {
                changed |= knob_column(&mut columns[0], tr("BASS", "GRAVES"), &mut bass, enabled);
                changed |= knob_column(&mut columns[1], tr("MID", "MEDIOS"), &mut mid, enabled);
                changed |= knob_column(
                    &mut columns[2],
                    tr("TREBLE", "AGUDOS"),
                    &mut treble,
                    enabled,
                );
                reverb_changed |= amount_knob_column(
                    &mut columns[3],
                    tr("REVERB", "REVERB"),
                    &mut reverb,
                    enabled,
                );
            });
        }

        if reverb_changed {
            if matches!(
                self.config.spatial,
                SpatialMode::Off | SpatialMode::Headphones3D
            ) {
                self.config.spatial = SpatialMode::LargeHall;
            }
            self.config.spatial_mix = reverb as f32;
            self.publish();
        }

        if changed {
            self.config.basic.bass = bass;
            self.config.basic.mid = mid;
            self.config.basic.treble = treble;
            self.config.basic.peak_limiter = peak_limiter;
            self.config.basic.selected_preset = "manual".into();
            self.publish();
        }
    }

    fn advanced_equalizer(&mut self, ui: &mut egui::Ui) {
        let language = self.config.language;
        let tr = |english: &'static str, spanish: &'static str| language.t(english, spanish);
        let enabled = self.config.app_enabled && self.config.eq_enabled;
        let mut global = self.config.advanced.global;
        let mut bands = self.config.advanced.bands;
        let mut changed = false;

        ui.horizontal(|ui| {
            widgets::value_screen(ui, tr("Global:", "Global:"), enabled, 54.0);
            if widgets::flat_slider(
                ui,
                &mut global,
                MIN_GAIN,
                MAX_GAIN,
                250.0,
                0.0,
                true,
                enabled,
            )
            .changed()
            {
                changed = true;
            }
            widgets::value_screen(ui, &format_gain(global), enabled, 54.0);
        });

        if self.config.ui.spectrum {
            ui.add_space(2.0);
            if widgets::spectrum(
                ui,
                &self.spectrum,
                enabled,
                &mut self.config.ui.spectrum_style,
                language == Language::Spanish,
            ) {
                self.mark_dirty();
            }
        }

        ui.add_space(4.0);
        let band_width = 33.0;
        let gap = 2.0;
        let annotation_width = 18.0;
        let total_width = annotation_width + 10.0 * band_width + 10.0 * gap;
        let available = ui.available_width();
        let mut reset_bands = false;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            ui.add_space(((available - total_width) * 0.5).max(0.0));
            ui.allocate_ui_with_layout(
                Vec2::new(annotation_width, 154.0),
                Layout::top_down(Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    theme::small_label(ui, "Hz");
                    if widgets::reset_triangle(ui, enabled).clicked() {
                        reset_bands = true;
                    }
                    theme::small_label(ui, tr("Gain", "Ganancia"));
                },
            );
            for i in 0..10 {
                ui.allocate_ui_with_layout(
                    Vec2::new(band_width, 154.0),
                    Layout::top_down(Align::Center),
                    |ui| {
                        ui.spacing_mut().item_spacing.y = 4.0;
                        theme::small_label(ui, FREQUENCY_LABELS[i]);
                        if widgets::fader(ui, &mut bands[i], MIN_GAIN, MAX_GAIN, 118.0, enabled)
                            .changed()
                        {
                            changed = true;
                        }
                        widgets::value_screen(ui, &format_gain(bands[i]), enabled, 34.0);
                    },
                );
            }
        });

        if reset_bands {
            bands = [0.0; 10];
            changed = true;
        }

        if changed {
            self.config.advanced.global = global;
            self.config.advanced.bands = bands;
            self.config.advanced.selected_preset = "manual".into();
            self.publish();
        }
    }

    fn expert_equalizer(&mut self, ui: &mut egui::Ui) {
        let enabled = self.config.app_enabled && self.config.eq_enabled;
        let spanish = self.config.language == crate::i18n::Language::Spanish;
        let mut global = self.config.expert.global;
        let mut peak_limiter = self.config.expert.peak_limiter;
        let mut bands = self.config.expert.bands.clone();
        let mut changed = false;
        let mut add_band = false;

        ui.horizontal(|ui| {
            theme::small_label(ui, "Global:");
            if widgets::flat_slider(
                ui,
                &mut global,
                MIN_GAIN,
                MAX_GAIN,
                220.0,
                0.0,
                true,
                enabled,
            )
            .changed()
            {
                changed = true;
            }
            widgets::value_screen(ui, &format_gain(global), enabled, 54.0);
        });

        ui.horizontal(|ui| {
            let available = ui.available_width();
            ui.add_space(((available - 180.0) * 0.5).max(0.0));
            if widgets::labeled_checkbox(
                ui,
                &mut peak_limiter,
                if spanish { "Limitador" } else { "Peak Limiter" },
                enabled,
            )
            .changed()
            {
                changed = true;
            }
            widgets::help_badge(
                ui,
                if spanish {
                    "Baja el nivel global según la ganancia positiva máxima para evitar saturación."
                } else {
                    "Lowers the global gain by the highest positive band gain to avoid clipping."
                },
            );
        });

        if self.config.ui.spectrum {
            ui.add_space(2.0);
            if widgets::spectrum(
                ui,
                &self.spectrum,
                enabled,
                &mut self.config.ui.spectrum_style,
                spanish,
            ) {
                self.mark_dirty();
            }
        }

        ui.add_space(4.0);
        changed |= widgets::expert_graph(ui, &mut bands, enabled);

        ui.add_space(4.0);
        let mut remove: Option<usize> = None;
        egui::ScrollArea::vertical()
            .max_height(210.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                if bands.is_empty() {
                    ui.label(
                        RichText::new(if spanish {
                            "Sin bandas. Pulsa + para añadir."
                        } else {
                            "No bands. Press + to add one."
                        })
                        .size(10.0)
                        .color(TEXT_DIM),
                    );
                }
                for (index, band) in bands.iter_mut().enumerate() {
                    let band_col = widgets::band_color(band.frequency);
                    let badge_id = ui.scope_id().with(("band-badge", index));
                    egui::Frame::new()
                        .fill(PANEL_DARK)
                        .corner_radius(4.0)
                        .inner_margin(egui::Margin::symmetric(6, 4))
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing = egui::Vec2::new(4.0, 2.0);

                            // ── Row 1: [✓] [freq badge] [G:] [slider] [value] [-] ──
                            ui.horizontal(|ui| {
                                // Checkbox
                                if widgets::check_box(ui, &mut band.enabled, enabled).changed() {
                                    changed = true;
                                }

                                // Colored frequency badge with popup for filter/channel
                                let freq_text = if band.frequency >= 1000.0 {
                                    format!("{:.1}k", band.frequency / 1000.0)
                                } else {
                                    format!("{:.0}Hz", band.frequency)
                                };
                                let badge_btn = egui::Button::new(
                                    RichText::new(&freq_text)
                                        .size(10.0)
                                        .color(egui::Color32::WHITE)
                                        .strong(),
                                )
                                .fill(band_col.gamma_multiply(0.85))
                                .corner_radius(4.0)
                                .min_size(egui::Vec2::new(52.0, 18.0));
                                let badge_resp = ui.add(badge_btn);
                                if badge_resp.clicked() {
                                    ui.data_mut(|d| {
                                        let open: bool = d.get_temp(badge_id).unwrap_or(false);
                                        d.insert_temp(badge_id, !open);
                                    });
                                }

                                // G label
                                ui.label(RichText::new("G:").size(10.0).color(TEXT_DIM));
                                // Gain slider
                                if widgets::flat_slider(
                                    ui,
                                    &mut band.gain,
                                    MIN_GAIN,
                                    MAX_GAIN,
                                    160.0,
                                    0.0,
                                    true,
                                    enabled,
                                )
                                .changed()
                                {
                                    changed = true;
                                }
                                // Gain value screen
                                widgets::value_screen(ui, &format_gain(band.gain), enabled, 52.0);

                                // Remove button (right-aligned)
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if widgets::icon_button(ui, Icon::Minus, enabled).clicked() {
                                        remove = Some(index);
                                    }
                                });
                            });

                            // ── Row 2: (indent) [Q:] [slider] [value] ──
                            ui.horizontal(|ui| {
                                // Indent to align Q slider with G slider
                                ui.add_space(18.0 + 52.0 + 4.0); // checkbox + badge + gap

                                ui.label(RichText::new("Q:").size(10.0).color(TEXT_DIM));
                                if widgets::flat_slider(
                                    ui,
                                    &mut band.q,
                                    0.1,
                                    20.0,
                                    160.0,
                                    0.0,
                                    false,
                                    enabled,
                                )
                                .changed()
                                {
                                    changed = true;
                                }
                                widgets::value_screen(ui, &format!("{:.2}", band.q), enabled, 52.0);
                            });

                            // ── Popup: filter type + channel (shown when badge clicked) ──
                            let popup_open: bool =
                                ui.data(|d| d.get_temp(badge_id).unwrap_or(false));
                            if popup_open {
                                ui.separator();
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 4.0;
                                    egui::ComboBox::from_id_salt(("expert-filter", index))
                                        .width(94.0)
                                        .selected_text(band.filter_type.label(spanish))
                                        .show_ui(ui, |ui| {
                                            for filter in [
                                                FilterType::Peaking,
                                                FilterType::LowShelf,
                                                FilterType::HighShelf,
                                                FilterType::LowPass,
                                                FilterType::HighPass,
                                                FilterType::BandPass,
                                                FilterType::Notch,
                                            ] {
                                                if ui
                                                    .selectable_label(
                                                        band.filter_type == filter,
                                                        filter.label(spanish),
                                                    )
                                                    .clicked()
                                                {
                                                    band.filter_type = filter;
                                                    changed = true;
                                                }
                                            }
                                        });
                                    egui::ComboBox::from_id_salt(("expert-channel", index))
                                        .width(62.0)
                                        .selected_text(band.channel.label(spanish))
                                        .show_ui(ui, |ui| {
                                            for channel in
                                                [Channel::Both, Channel::Left, Channel::Right]
                                            {
                                                if ui
                                                    .selectable_label(
                                                        band.channel == channel,
                                                        channel.label(spanish),
                                                    )
                                                    .clicked()
                                                {
                                                    band.channel = channel;
                                                    changed = true;
                                                }
                                            }
                                        });
                                });
                            }
                        });
                    ui.add_space(2.0);
                }
            });

        // ── "+ Add band" button ───────────────────────────────────────────
        ui.add_space(4.0);
        let add_btn = ui.add_sized(
            egui::Vec2::new(ui.available_width(), 26.0),
            egui::Button::new(
                RichText::new(if spanish {
                    "+ Añadir banda"
                } else {
                    "+ Add band"
                })
                .color(theme::accent())
                .size(11.0),
            )
            .fill(PANEL_DARK),
        );
        if add_btn.clicked() {
            add_band = true;
        }

        if let Some(index) = remove {
            bands.remove(index);
            changed = true;
        }
        if add_band {
            bands.push(crate::dsp::ExpertBand::default());
            changed = true;
        }

        if changed {
            self.config.expert.global = global;
            self.config.expert.peak_limiter = peak_limiter;
            self.config.expert.bands = bands;
            self.config.expert.selected_preset = "manual".into();
            self.publish();
        }
    }

    fn outputs_section(&mut self, ui: &mut egui::Ui) {
        let language = self.config.language;
        let tr = |english: &'static str, spanish: &'static str| language.t(english, spanish);
        let sinks: Vec<SinkInfo> = self
            .sinks
            .iter()
            .filter(|s| s.name != self.config.virtual_sink)
            .cloned()
            .collect();

        let selected_text = self
            .selected_sink
            .as_ref()
            .and_then(|name| sinks.iter().find(|s| &s.name == name))
            .map(|s| s.description.clone())
            .unwrap_or_else(|| tr("Select Output", "Seleccionar salida").into());

        let mut chosen: Option<String> = None;
        let mut reset = false;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if widgets::icon_button(ui, Icon::Reset, true)
                .on_hover_text(tr("Reset playback", "Reiniciar reproducción"))
                .clicked()
            {
                reset = true;
            }

            egui::ComboBox::from_id_salt("output-sink")
                .width(268.0)
                .selected_text(selected_text)
                .show_ui(ui, |ui| {
                    if sinks.is_empty() {
                        ui.label(tr("No outputs", "Sin salidas"));
                    }
                    for sink in &sinks {
                        let selected = self.selected_sink.as_deref() == Some(&sink.name);
                        if ui.selectable_label(selected, &sink.description).clicked() {
                            chosen = Some(sink.name.clone());
                        }
                    }
                });

            widgets::vent(ui);
        });

        if reset {
            self.restart_engine();
        }
        if let Some(name) = chosen {
            self.set_output(name);
        }
    }

    fn app_mixer_section(&mut self, ui: &mut egui::Ui) {
        let enabled = self.config.app_enabled;
        if self.last_mixer_refresh.elapsed() > Duration::from_secs(1) {
            self.app_mixer = mixer::list().unwrap_or_default();
            self.last_mixer_refresh = Instant::now();
        }

        ui.horizontal(|ui| {
            theme::small_label(ui, self.tr("App Mixer", "Mezclador de apps"));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if widgets::icon_button(ui, Icon::Reset, enabled)
                    .on_hover_text(self.tr("Refresh", "Actualizar"))
                    .clicked()
                {
                    self.app_mixer = mixer::list().unwrap_or_default();
                    self.last_mixer_refresh = Instant::now();
                }
            });
        });

        if self.app_mixer.is_empty() {
            ui.label(
                RichText::new(self.tr(
                    "No applications are playing audio.",
                    "No hay aplicaciones reproduciendo audio.",
                ))
                .size(10.0)
                .color(TEXT_DIM),
            );
            return;
        }

        let mut volume_changes = Vec::new();
        let mut mute_changes = Vec::new();
        for input in &self.app_mixer {
            ui.horizontal(|ui| {
                let mut volume = input.volume as f64;
                let label = ui.add_sized(
                    [118.0, 18.0],
                    egui::Label::new(RichText::new(&input.app).size(10.0).color(TEXT)).truncate(),
                );
                if !input.media.is_empty() {
                    label.on_hover_text(&input.media);
                }
                if widgets::flat_slider(ui, &mut volume, 0.0, 1.5, 96.0, 1.0, false, enabled)
                    .changed()
                {
                    volume_changes.push((input.index, volume as f32));
                }
                widgets::value_screen(ui, &format!("{:.0}%", volume * 100.0), enabled, 44.0);
                let mut muted = input.muted;
                if widgets::check_box(ui, &mut muted, enabled).changed() {
                    mute_changes.push((input.index, muted));
                }
            });
        }

        for (index, volume) in volume_changes {
            if let Err(err) = mixer::set_volume(index, volume) {
                self.status = format!("App mixer error: {err}");
            }
        }
        for (index, muted) in mute_changes {
            if let Err(err) = mixer::set_mute(index, muted) {
                self.status = format!("App mixer error: {err}");
            }
        }
    }

    fn footer(&mut self, ui: &mut egui::Ui) {
        let is_error = self.status.contains("error")
            || self.status.contains("Error")
            || self.status.contains("Could not")
            || self.status.contains("System tray")
            || self.status.contains("No output")
            || self.status.contains("Capture")
            || self.status.contains("Playback");

        if is_error {
            ui.add_space(4.0);
            ui.label(
                RichText::new(self.status_display())
                    .size(9.0)
                    .color(WARNING),
            );
        }
    }

    fn settings_window(&mut self, ctx: &egui::Context) {
        if !self.show_settings {
            return;
        }

        let language = self.config.language;
        let tr = |english: &'static str, spanish: &'static str| language.t(english, spanish);
        let mut open = true;
        egui::Window::new(tr("Settings", "Ajustes"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::RIGHT_TOP, [-6.0, 34.0])
            .show(ctx, |ui| {
                ui.set_width(310.0);
                let language = self.config.language;
                let tr =
                    |english: &'static str, spanish: &'static str| language.t(english, spanish);
                egui::ScrollArea::vertical()
                    .max_height(360.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        theme::heading(ui, tr("Language", "Idioma"));
                        let mut language = self.config.language;
                        egui::ComboBox::from_id_salt("language")
                            .width(180.0)
                            .selected_text(language.label())
                            .show_ui(ui, |ui| {
                                for option in [Language::English, Language::Spanish] {
                                    if ui
                                        .selectable_label(language == option, option.label())
                                        .clicked()
                                    {
                                        language = option;
                                    }
                                }
                            });
                        if language != self.config.language {
                            self.config.language = language;
                            self.mark_dirty();
                        }

                        ui.separator();
                        theme::heading(ui, tr("Features", "Funciones"));
                        if ui
                            .checkbox(&mut self.config.ui.show_volume, tr("Volume", "Volumen"))
                            .changed()
                            || ui
                                .checkbox(
                                    &mut self.config.ui.show_balance,
                                    tr("Balance", "Balance"),
                                )
                                .changed()
                            || ui
                                .checkbox(
                                    &mut self.config.ui.show_equalizer,
                                    tr("Equalizer", "Ecualizador"),
                                )
                                .changed()
                            || ui
                                .checkbox(
                                    &mut self.config.ui.show_outputs,
                                    tr("Outputs", "Salidas"),
                                )
                                .changed()
                            || ui
                                .checkbox(
                                    &mut self.config.ui.show_app_mixer,
                                    tr("App Mixer", "Mezclador de apps"),
                                )
                                .changed()
                        {
                            self.mark_dirty();
                        }

                        ui.separator();
                        theme::heading(ui, tr("Interface", "Interfaz"));
                        if ui
                            .checkbox(
                                &mut self.config.ui.use_sliders,
                                tr(
                                    "Use sliders instead of knobs",
                                    "Usar deslizadores en vez de mandos",
                                ),
                            )
                            .changed()
                            || ui
                                .checkbox(
                                    &mut self.config.ui.spectrum,
                                    tr("Spectrum analyzer", "Analizador de espectro"),
                                )
                                .changed()
                        {
                            self.mark_dirty();
                        }

                        let mut start_minimized = self.config.start_minimized_on_startup;
                        if ui
                            .checkbox(
                                &mut start_minimized,
                                tr(
                                    "Start minimized when Linux starts",
                                    "Iniciar minimizada al iniciar Linux",
                                ),
                            )
                            .changed()
                        {
                            let enabled = start_minimized;
                            if let Err(error) = self.set_start_minimized_on_startup(enabled) {
                                self.toast = Some((
                                    format!("Could not update autostart: {error}"),
                                    Instant::now(),
                                ));
                            }
                        }

                        let mut accent = self.config.accent;
                        ui.horizontal(|ui| {
                            theme::small_label(ui, tr("Accent", "Acento"));
                            egui::ComboBox::from_id_salt("accent")
                                .width(150.0)
                                .selected_text(accent.label(language == Language::Spanish))
                                .show_ui(ui, |ui| {
                                    for option in [
                                        Accent::Green,
                                        Accent::Blue,
                                        Accent::Purple,
                                        Accent::Orange,
                                        Accent::Pink,
                                        Accent::Graphite,
                                    ] {
                                        if ui
                                            .selectable_label(
                                                accent == option,
                                                option.label(language == Language::Spanish),
                                            )
                                            .clicked()
                                        {
                                            accent = option;
                                        }
                                    }
                                });
                        });
                        if accent != self.config.accent {
                            self.config.accent = accent;
                            theme::set_accent(accent.rgb());
                            theme::configure(ctx);
                            self.mark_dirty();
                        }

                        ui.separator();
                        theme::heading(ui, tr("ALfa effects", "Efectos ALfa"));
                        let mut spatial = self.config.spatial;
                        ui.horizontal(|ui| {
                            theme::small_label(ui, tr("Spatial audio", "Audio espacial"));
                            egui::ComboBox::from_id_salt("spatial")
                                .width(132.0)
                                .selected_text(spatial.label(language == Language::Spanish))
                                .show_ui(ui, |ui| {
                                    for option in [
                                        SpatialMode::Off,
                                        SpatialMode::SmallRoom,
                                        SpatialMode::LargeHall,
                                        SpatialMode::Concert,
                                        SpatialMode::Headphones3D,
                                    ] {
                                        if ui
                                            .selectable_label(
                                                spatial == option,
                                                option.label(language == Language::Spanish),
                                            )
                                            .clicked()
                                        {
                                            spatial = option;
                                        }
                                    }
                                });
                        });
                        if spatial != self.config.spatial {
                            self.config.spatial = spatial;
                            if spatial == SpatialMode::Headphones3D {
                                self.config.spatial_mix = 1.0;
                            }
                            self.publish();
                            self.mark_dirty();
                        }
                        if self.config.spatial != SpatialMode::Off {
                            let mut mix = self.config.spatial_mix as f64;
                            ui.horizontal(|ui| {
                                theme::small_label(ui, tr("Mix", "Mezcla"));
                                if widgets::flat_slider(
                                    ui, &mut mix, 0.0, 1.0, 140.0, 0.5, false, true,
                                )
                                .changed()
                                {
                                    self.config.spatial_mix = mix as f32;
                                    self.publish();
                                    self.mark_dirty();
                                }
                                widgets::value_screen(
                                    ui,
                                    &format!("{:.0}%", mix * 100.0),
                                    true,
                                    42.0,
                                );
                            });
                            if self.config.spatial == SpatialMode::Headphones3D {
                                ui.label(
                                    RichText::new(tr(
                                        "For headphones; increase Mix for a stronger effect.",
                                        "Para auriculares; sube la mezcla para un efecto más intenso.",
                                    ))
                                    .size(9.0)
                                    .color(TEXT_DIM),
                                );
                            }
                        }

                        let mut left_trim = self.config.left_trim;
                        let mut right_trim = self.config.right_trim;
                        ui.horizontal(|ui| {
                            theme::small_label(ui, tr("L trim", "Aten. izq."));
                            let left_changed = ui
                                .add(
                                    egui::DragValue::new(&mut left_trim)
                                        .range(-12.0..=12.0)
                                        .speed(0.1)
                                        .suffix(" dB"),
                                )
                                .changed();
                            theme::small_label(ui, tr("R trim", "Aten. der."));
                            let right_changed = ui
                                .add(
                                    egui::DragValue::new(&mut right_trim)
                                        .range(-12.0..=12.0)
                                        .speed(0.1)
                                        .suffix(" dB"),
                                )
                                .changed();
                            if left_changed || right_changed {
                                self.config.left_trim = left_trim;
                                self.config.right_trim = right_trim;
                                self.publish();
                                self.mark_dirty();
                            }
                        });

                        ui.separator();
                        theme::heading(ui, tr("Headphone presets", "Presets de auriculares"));
                        if ui
                            .button(tr(
                                "Apply Corsair HS55 profile",
                                "Aplicar perfil Corsair HS55",
                            ))
                            .clicked()
                        {
                            self.apply_hs55_profile();
                        }
                        ui.label(
                            RichText::new(tr(
                                "Personalized starting point, not a measured AutoEQ calibration.",
                                "Ajuste inicial; no es una calibración AutoEQ medida.",
                            ))
                            .size(9.0)
                            .color(TEXT_DIM),
                        );
                        ui.label(
                            RichText::new(tr(
                                "Import an AutoEQ ParametricEQ.txt or CSV profile.",
                                "Importa un perfil AutoEQ ParametricEQ.txt o CSV.",
                            ))
                            .size(9.0)
                            .color(TEXT_DIM),
                        );
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.autoeq_path)
                                    .desired_width(164.0)
                                    .hint_text(tr("Profile path", "Ruta del perfil")),
                            );
                            if ui.button(tr("Import", "Importar")).clicked() {
                                match fs::read_to_string(&self.autoeq_path)
                                    .map_err(|err| err.to_string())
                                    .and_then(|text| autoeq::parse(&text))
                                {
                                    Ok(bands) => {
                                        self.config.expert.bands = bands;
                                        self.config.eq_mode = EqMode::Expert;
                                        self.config.expert.selected_preset = "manual".into();
                                        self.publish();
                                        self.toast = Some((
                                            tr(
                                                "AutoEQ profile imported",
                                                "Perfil AutoEQ importado",
                                            )
                                            .into(),
                                            Instant::now(),
                                        ));
                                    }
                                    Err(err) => self.status = format!("AutoEQ: {err}"),
                                }
                            }
                        });

                        ui.separator();
                        theme::heading(ui, tr("Recording & rendering", "Grabación y renderizado"));
                        if let Some(engine) = &self.engine {
                            let recording = engine.is_recording();
                            if ui
                                .button(if recording {
                                    tr("Stop recording", "Detener grabación")
                                } else {
                                    tr("Start recording", "Iniciar grabación")
                                })
                                .clicked()
                            {
                                if recording {
                                    if let Some(path) = engine.stop_recording() {
                                        self.recording_path = Some(path.clone());
                                        self.toast = Some((
                                            format!(
                                                "{}: {}",
                                                tr("Recording saved", "Grabación guardada"),
                                                path.display()
                                            ),
                                            Instant::now(),
                                        ));
                                    }
                                } else {
                                    match engine.start_recording() {
                                        Ok(path) => {
                                            self.recording_path = Some(path);
                                        }
                                        Err(err) => self.status = format!("Recording error: {err}"),
                                    }
                                }
                            }
                        } else {
                            ui.label(
                                RichText::new(tr(
                                    "Enable system-wide routing to record.",
                                    "Activa el enrutado global para grabar.",
                                ))
                                .size(9.0)
                                .color(TEXT_DIM),
                            );
                        }
                        if let Some(path) = &self.recording_path {
                            ui.label(
                                RichText::new(path.display().to_string())
                                    .size(8.0)
                                    .color(TEXT_DIM),
                            );
                        }

                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.render_input)
                                    .desired_width(124.0)
                                    .hint_text(tr("Input WAV", "WAV entrada")),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.render_output)
                                    .desired_width(124.0)
                                    .hint_text(tr("Output WAV", "WAV salida")),
                            );
                            if ui.button(tr("Render", "Renderizar")).clicked() {
                                if self.render_input.is_empty() || self.render_output.is_empty() {
                                    self.status = self
                                        .tr(
                                            "Select input and output WAV files",
                                            "Elige archivos WAV",
                                        )
                                        .into();
                                } else {
                                    let params = self.config.eq_params();
                                    match render::render_file(
                                        std::path::Path::new(&self.render_input),
                                        std::path::Path::new(&self.render_output),
                                        &params,
                                        self.config.effective_volume(),
                                        self.config.balance,
                                    ) {
                                        Ok(()) => {
                                            self.toast = Some((
                                                tr("File rendered", "Archivo renderizado").into(),
                                                Instant::now(),
                                            ));
                                        }
                                        Err(err) => self.status = format!("Render: {err}"),
                                    }
                                }
                            }
                        });

                        ui.separator();
                        theme::heading(ui, tr("Super Presets", "Superpresets"));
                        if ui
                            .checkbox(
                                &mut self.config.super_presets_enabled,
                                tr(
                                    "Switch presets by output device",
                                    "Cambiar presets según la salida",
                                ),
                            )
                            .changed()
                        {
                            self.mark_dirty();
                        }
                        let selected_output = self.selected_sink.clone();
                        let can_add = selected_output.is_some();
                        if ui
                            .add_enabled(
                                can_add,
                                egui::Button::new(tr(
                                    "Add current preset for this output",
                                    "Añadir preset actual para esta salida",
                                )),
                            )
                            .clicked()
                        {
                            if let Some(output) = selected_output.clone() {
                                let id = format!(
                                    "super-{}",
                                    SystemTime::now()
                                        .duration_since(UNIX_EPOCH)
                                        .map(|d| d.as_millis())
                                        .unwrap_or(0)
                                );
                                let preset_id = match self.config.eq_mode {
                                    EqMode::Basic => self.config.basic.selected_preset.clone(),
                                    EqMode::Advanced => {
                                        self.config.advanced.selected_preset.clone()
                                    }
                                    EqMode::Expert => self.config.expert.selected_preset.clone(),
                                };
                                let name = self
                                    .sinks
                                    .iter()
                                    .find(|sink| sink.name == output)
                                    .map(|sink| sink.description.clone())
                                    .unwrap_or_else(|| output.clone());
                                self.config.super_presets.push(SuperPreset {
                                    id,
                                    name,
                                    output_sink: output,
                                    mode: self.config.eq_mode,
                                    preset_id,
                                });
                                self.mark_dirty();
                            }
                        }

                        let mut remove_rule: Option<usize> = None;
                        for (index, rule) in self.config.super_presets.iter().enumerate() {
                            let output_name = self
                                .sinks
                                .iter()
                                .find(|sink| sink.name == rule.output_sink)
                                .map(|sink| sink.description.clone())
                                .unwrap_or_else(|| rule.output_sink.clone());
                            let preset_name = self.preset_display_name(rule.mode, &rule.preset_id);
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!("{output_name} → {preset_name}"))
                                        .size(9.0)
                                        .color(TEXT_DIM),
                                );
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    if widgets::icon_button(ui, Icon::Trash, true).clicked() {
                                        remove_rule = Some(index);
                                    }
                                });
                            });
                        }
                        if let Some(index) = remove_rule {
                            self.config.super_presets.remove(index);
                            self.mark_dirty();
                        }

                        ui.separator();
                        theme::heading(ui, tr("System", "Sistema"));
                        ui.label(
                            RichText::new(format!(
                                "{}: {}",
                                tr("Virtual sink", "Sink virtual"),
                                self.config.virtual_sink
                            ))
                            .size(10.0)
                            .color(TEXT_DIM),
                        );
                        ui.label(
                            RichText::new(format!(
                                "{}: {}",
                                tr("Status", "Estado"),
                                self.status_display()
                            ))
                            .size(10.0)
                            .color(TEXT_DIM),
                        );

                        let mut routing_enabled = self.config.routing_enabled;
                        if ui
                            .checkbox(
                                &mut routing_enabled,
                                tr("System-wide routing", "Enrutado global"),
                            )
                            .changed()
                        {
                            self.set_routing(routing_enabled);
                        }

                        ui.horizontal(|ui| {
                            if ui
                                .button(tr("Reconnect audio", "Reconectar audio"))
                                .clicked()
                            {
                                self.restart_engine();
                            }
                            if ui.button(tr("Save settings", "Guardar ajustes")).clicked() {
                                let _ = self.config.save();
                            }
                        });

                        ui.separator();
                        ui.label(RichText::new("eqLinux 0.1.0").size(10.0).color(TEXT));
                        ui.label(
                            RichText::new("Inspired by eqMac · bitgapp/eqMac")
                                .size(9.0)
                                .color(TEXT_DIM),
                        );
                    });
            });

        if !open {
            self.show_settings = false;
        }
    }

    fn help_window(&mut self, ctx: &egui::Context) {
        if !self.show_help {
            return;
        }

        let mut open = true;
        let language = self.config.language;
        let tr = |english: &'static str, spanish: &'static str| language.t(english, spanish);
        egui::Window::new(tr("About eqLinux", "Acerca de eqLinux"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_width(300.0);
                ui.heading("eqLinux");
                ui.add_space(4.0);
                ui.label(tr(
                    "System-wide audio equalizer and volume mixer for Linux, written in Rust.",
                    "Ecualizador y mezclador de volumen global para Linux, escrito en Rust.",
                ));
                ui.add_space(10.0);
                egui::Frame::new()
                    .fill(PANEL_DARK)
                    .stroke(egui::Stroke::new(
                        1.0,
                        egui::Color32::from_rgb(58, 61, 66),
                    ))
                    .corner_radius(egui::CornerRadius::same(8))
                    .inner_margin(egui::Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(tr("Made by", "Elaborado por"))
                                    .size(11.0)
                                    .color(TEXT_DIM),
                            );
                            ui.label(RichText::new("etovive").strong().color(TEXT));
                        });
                        ui.add_space(8.0);

                        let button_width = ui.available_width();
                        let (rect, response) = ui.allocate_exact_size(
                            Vec2::new(button_width, 54.0),
                            egui::Sense::click(),
                        );
                        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
                        let painter = ui.painter();
                        let fill = if response.hovered() {
                            egui::Color32::from_rgb(246, 249, 253)
                        } else {
                            egui::Color32::from_rgb(237, 243, 250)
                        };
                        painter.rect_filled(rect, 7.0, fill);
                        painter.rect_stroke(
                            rect,
                            7.0,
                            egui::Stroke::new(1.0, egui::Color32::from_rgb(190, 207, 228)),
                            egui::StrokeKind::Inside,
                        );

                        let logo_center = rect.left_center() + Vec2::new(27.0, 0.0);
                        painter.circle_filled(logo_center, 17.0, egui::Color32::WHITE);
                        painter.text(
                            logo_center + Vec2::new(-2.5, 1.0),
                            Align2::CENTER_CENTER,
                            "P",
                            egui::FontId::proportional(27.0),
                            egui::Color32::from_rgb(0, 156, 222),
                        );
                        painter.text(
                            logo_center + Vec2::new(2.5, -1.0),
                            Align2::CENTER_CENTER,
                            "P",
                            egui::FontId::proportional(27.0),
                            egui::Color32::from_rgb(0, 48, 135),
                        );
                        painter.text(
                            rect.left_center() + Vec2::new(54.0, -8.0),
                            Align2::LEFT_CENTER,
                            "PayPal",
                            egui::FontId::proportional(14.0),
                            egui::Color32::from_rgb(0, 48, 135),
                        );
                        painter.text(
                            rect.left_center() + Vec2::new(54.0, 11.0),
                            Align2::LEFT_CENTER,
                            tr("Buy me a coffee", "Invítame un café"),
                            egui::FontId::proportional(11.0),
                            egui::Color32::from_rgb(60, 69, 82),
                        );

                        if response.clicked() {
                            self.toast = Some((
                                match open_external_url(PAYPAL_URL) {
                                    Ok(()) => tr(
                                        "Opening PayPal in your browser…",
                                        "Abriendo PayPal en el navegador…",
                                    )
                                    .to_string(),
                                    Err(error) => format!(
                                        "{}: {error}",
                                        tr("Could not open PayPal", "No se pudo abrir PayPal")
                                    ),
                                },
                                Instant::now(),
                            ));
                        }
                    });
                ui.label("eqLinux en GitHub");
                ui.add_space(6.0);
                ui.label(RichText::new(tr("How it works", "Cómo funciona")).strong());
                ui.label(tr(
                    "eqLinux creates a virtual output, moves your applications to it, captures its monitor stream, applies the same ten-band parametric EQ as eqMac and plays the result to the output device you choose.",
                    "eqLinux crea una salida virtual, mueve tus aplicaciones a ella, captura su flujo monitor, aplica el mismo ecualizador paramétrico de diez bandas que eqMac y reproduce el resultado en la salida que elijas.",
                ));
                ui.add_space(6.0);
                ui.label(RichText::new(tr("Shortcuts", "Atajos")).strong());
                ui.label(tr(
                    "Drag a knob or fader vertically · double-click to reset · scroll to fine-tune.",
                    "Arrastra un mando o deslizador en vertical · doble clic para restablecer · rueda para ajuste fino.",
                ));
                ui.add_space(6.0);
                if ui
                    .link(tr("eqMac on GitHub", "eqMac en GitHub"))
                    .clicked()
                {
                    self.toast = Some((
                        match open_external_url("https://github.com/bitgapp/eqMac") {
                            Ok(()) => tr(
                                "Opening eqMac in your browser…",
                                "Abriendo eqMac en el navegador…",
                            )
                            .to_string(),
                            Err(error) => format!(
                                "{}: {error}",
                                tr("Could not open eqMac", "No se pudo abrir eqMac")
                            ),
                        },
                        Instant::now(),
                    ));
                }
                ui.label(
                    RichText::new(tr(
                        "eqMac is Apache-2.0 licensed. eqLinux is an independent reimplementation.",
                        "eqMac tiene licencia Apache-2.0. eqLinux es una reimplementación independiente.",
                    ))
                    .size(9.0)
                    .color(TEXT_DIM),
                );
            });

        if !open {
            self.show_help = false;
        }
    }

    fn save_window(&mut self, ctx: &egui::Context) {
        let Some(dialog) = &mut self.save_dialog else {
            return;
        };

        let mut open = true;
        let mut save = false;
        let mut cancel = false;

        let language = self.config.language;
        let tr = |english: &'static str, spanish: &'static str| language.t(english, spanish);
        egui::Window::new(tr("Save Preset", "Guardar preset"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_width(250.0);
                ui.label(tr("Preset name", "Nombre del preset"));
                let response = ui.add(egui::TextEdit::singleline(&mut dialog.name));
                if !response.has_focus() {
                    response.request_focus();
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button(tr("Cancel", "Cancelar")).clicked() {
                        cancel = true;
                    }
                    let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if ui.button(tr("Save", "Guardar")).clicked() || enter {
                        save = true;
                    }
                });
            });

        if save {
            let name = dialog.name.trim().to_string();
            let mode = dialog.mode;
            self.save_dialog = None;
            if !name.is_empty() {
                self.save_preset(&name, mode);
            }
        } else if cancel || !open {
            self.save_dialog = None;
        }
    }

    fn toast(&mut self, ctx: &egui::Context) {
        let Some((message, started)) = &self.toast else {
            return;
        };
        if started.elapsed() > Duration::from_secs(2) {
            self.toast = None;
            return;
        }
        ctx.request_repaint_after(Duration::from_millis(100));

        egui::Area::new(egui::Id::unique("eqlinux-toast"))
            .anchor(Align2::CENTER_BOTTOM, [0.0, -14.0])
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(PANEL_DARK)
                    .corner_radius(6.0)
                    .inner_margin(egui::Margin::symmetric(12, 6))
                    .stroke(egui::Stroke::new(1.0, theme::accent()))
                    .show(ui, |ui| {
                        ui.label(RichText::new(message).size(10.0).color(TEXT));
                    });
            });
    }

    fn process_tray_events(&mut self, ctx: &egui::Context) {
        let tray_events: Vec<_> = self.tray_events.try_iter().collect();
        for event in tray_events {
            match event {
                TrayEvent::Action(TrayAction::ShowWindow) => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
                        egui::UserAttentionType::Critical,
                    ));
                    ctx.request_repaint();
                }
                TrayEvent::Action(TrayAction::HideWindow) => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                }
                TrayEvent::Action(TrayAction::ToggleProcessing) => {
                    self.config.app_enabled = !self.config.app_enabled;
                    self.publish();
                    let (summary, body) = if self.config.language == Language::Spanish {
                        (
                            "eqLinux",
                            if self.config.app_enabled {
                                "Procesamiento de audio activado"
                            } else {
                                "Procesamiento de audio desactivado"
                            },
                        )
                    } else {
                        (
                            "eqLinux",
                            if self.config.app_enabled {
                                "Audio processing enabled"
                            } else {
                                "Audio processing disabled"
                            },
                        )
                    };
                    self.toast = Some((body.to_string(), Instant::now()));
                    notify_desktop(summary, body);
                    ctx.request_repaint();
                }
                TrayEvent::Action(TrayAction::Quit) => {
                    self.quit_requested = true;
                }
                TrayEvent::Unavailable(error) => {
                    self.status = format!("System tray unavailable: {error}");
                }
            }
        }
    }
}

impl eframe::App for EqLinuxApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_tray_events(ctx);

        if let Some(dirty) = self.dirty_at {
            if dirty.elapsed() > Duration::from_millis(350) {
                let _ = self.config.save();
                self.dirty_at = None;
            } else {
                ctx.request_repaint_after(Duration::from_millis(120));
            }
        }

        if self.last_sink_refresh.elapsed() > Duration::from_secs(2) {
            self.refresh_sinks();
            self.last_sink_refresh = Instant::now();
        }

        if let Some(limit) = self.quit_after {
            if self.started_at.elapsed() >= limit {
                self.quit_requested = true;
            }
        }

        if self.quit_requested {
            self.quit_requested = false;
            self.shutdown();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            std::process::exit(0);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.process_tray_events(&ctx);

        if let Some(dirty) = self.dirty_at {
            if dirty.elapsed() > Duration::from_millis(350) {
                let _ = self.config.save();
                self.dirty_at = None;
            } else {
                ctx.request_repaint_after(Duration::from_millis(120));
            }
        }

        if self.last_sink_refresh.elapsed() > Duration::from_secs(2) {
            self.refresh_sinks();
            self.last_sink_refresh = Instant::now();
        }

        if self.config.ui.spectrum {
            ctx.request_repaint_after(Duration::from_millis(33));
        }

        if self.quit_after.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }

        if let Some(engine) = &self.engine {
            let status = engine.status();
            if !status.is_empty() {
                self.status = status;
            }
            self.spectrum = engine.spectrum();
        }

        theme::paint_background(ui);
        let content = egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(12, 6))
            .show(ui, |ui| {
                self.header(ui);
                ui.add_space(2.0);

                if self.config.ui.show_volume || self.config.ui.show_balance {
                    self.volume_section(ui);
                    widgets::divider(ui);
                }

                if self.config.ui.show_equalizer {
                    self.equalizer_section(ui);
                    widgets::divider(ui);
                }

                if self.config.ui.show_outputs {
                    self.outputs_section(ui);
                }

                if self.config.ui.show_app_mixer {
                    widgets::divider(ui);
                    self.app_mixer_section(ui);
                }

                self.footer(ui);
            });
        let desired_height = content.response.rect.height().max(400.0);
        if (desired_height - self.last_content_height).abs() > 0.5 {
            self.last_content_height = desired_height;
            let width = ctx.content_rect().width();
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                width,
                desired_height,
            )));
            ctx.request_repaint();
        }

        self.settings_window(&ctx);
        self.help_window(&ctx);
        self.save_window(&ctx);
        self.toast(&ctx);

        if let Some(limit) = self.quit_after {
            if self.started_at.elapsed() >= limit {
                self.quit_requested = true;
            }
        }

        if self.quit_requested {
            self.quit_requested = false;
            self.shutdown();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            std::process::exit(0);
        }
    }
}

impl Drop for EqLinuxApp {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn notify_desktop(summary: &str, body: &str) {
    let _ = std::process::Command::new("notify-send")
        .args(["-a", "eqLinux", "-i", "eqlinux", summary, body])
        .spawn();
}

fn open_external_url(url: &str) -> Result<(), String> {
    match std::process::Command::new("xdg-open").arg(url).spawn() {
        Ok(_) => Ok(()),
        Err(xdg_error) => std::process::Command::new("gio")
            .args(["open", url])
            .spawn()
            .map(|_| ())
            .map_err(|gio_error| format!("xdg-open: {xdg_error}; gio open: {gio_error}")),
    }
}

fn desktop_exec_argument(path: &Path) -> String {
    let mut argument = String::from("\"");
    for character in path.to_string_lossy().chars() {
        if matches!(character, '\\' | '"' | '$' | '`') {
            argument.push('\\');
        }
        if character == '%' {
            argument.push('%');
        }
        argument.push(character);
    }
    argument.push('"');
    argument
}

#[cfg(test)]
mod tests {
    use super::desktop_exec_argument;
    use std::path::Path;

    #[test]
    fn desktop_exec_argument_quotes_and_escapes_paths() {
        assert_eq!(
            desktop_exec_argument(Path::new("/tmp/eq Linux/100%/eq$Linux")),
            "\"/tmp/eq Linux/100%%/eq\\$Linux\""
        );
    }
}

fn knob_column(ui: &mut egui::Ui, label: &str, value: &mut f64, enabled: bool) -> bool {
    let mut changed = false;
    ui.vertical_centered(|ui| {
        theme::small_label(ui, label);
        if widgets::knob(ui, value, MIN_GAIN, MAX_GAIN, 72.0, 0.0, enabled).changed() {
            changed = true;
        }
        widgets::value_screen(ui, &format_gain(*value), enabled, 56.0);
    });
    changed
}

fn basic_slider_row(ui: &mut egui::Ui, label: &str, value: &mut f64, enabled: bool) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.add_sized(
            Vec2::new(48.0, 18.0),
            egui::Label::new(RichText::new(label).size(10.0).color(TEXT_DIM)),
        );
        if widgets::flat_slider(ui, value, MIN_GAIN, MAX_GAIN, 232.0, 0.0, true, enabled).changed()
        {
            changed = true;
        }
        widgets::value_screen(ui, &format_gain(*value), enabled, 56.0);
    });
    changed
}

fn amount_knob_column(ui: &mut egui::Ui, label: &str, value: &mut f64, enabled: bool) -> bool {
    let mut changed = false;
    ui.vertical_centered(|ui| {
        theme::small_label(ui, label);
        if widgets::knob(ui, value, 0.0, 1.0, 72.0, 0.0, enabled).changed() {
            changed = true;
        }
        widgets::value_screen(ui, &format!("{:.0}%", *value * 100.0), enabled, 56.0);
    });
    changed
}

fn basic_amount_slider_row(ui: &mut egui::Ui, label: &str, value: &mut f64, enabled: bool) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.add_sized(
            Vec2::new(48.0, 18.0),
            egui::Label::new(RichText::new(label).size(10.0).color(TEXT_DIM)),
        );
        if widgets::flat_slider(ui, value, 0.0, 1.0, 232.0, 0.0, false, enabled).changed() {
            changed = true;
        }
        widgets::value_screen(ui, &format!("{:.0}%", *value * 100.0), enabled, 56.0);
    });
    changed
}

fn format_gain(gain: f64) -> String {
    let sign = if gain > 0.0 { "+" } else { "" };
    format!("{sign}{gain:.1}dB")
}
