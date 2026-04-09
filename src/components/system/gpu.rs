// Asus Hub - Unofficial Control Center for Asus Laptops
// Copyright (C) 2026 Guido Philipp
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see https://www.gnu.org/licenses/.

use gtk4 as gtk;
use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;

use crate::services::config::AppConfig;
use crate::services::dbus;
use crate::services::dbus::GfxMode;

/// State for the GPU mode settings component.
pub struct GpuModel {
    /// Whether the `supergfxctl` daemon is reachable; disables controls when `false`.
    supergfxctl_available: bool,
    /// The currently active GPU mode, used to suppress no-op callback invocations.
    current_mode: GfxMode,
    /// Combined, deduplicated list of modes to display: active mode prepended to supported modes.
    display_modes: Vec<GfxMode>,
    /// Stored reference so `update_cmd` can imperatively update the model and selection.
    combo_row: adw::ComboRow,
}

/// Input messages for the GPU mode component.
#[derive(Debug)]
pub enum GpuMsg {
    /// User selected index `idx` in the mode dropdown.
    ChangeMode(u32),
}

/// Async command results for the GPU mode component.
#[derive(Debug)]
pub enum GpuCommandOutput {
    /// Result of the initial `supergfxctl` availability check.
    SupergfxctlChecked(bool),
    /// Current mode and the list of modes available for switching, read at startup.
    InitModeAndSupported(GfxMode, Vec<GfxMode>),
    /// Confirmation that `SetMode` succeeded; carries the daemon-confirmed new mode.
    ModeSet(GfxMode),
    /// An error message to forward as a toast notification.
    Fehler(String),
}

#[relm4::component(pub)]
impl Component for GpuModel {
    type Init = ();
    type Input = GpuMsg;
    type Output = String;
    type CommandOutput = GpuCommandOutput;

    view! {
        adw::PreferencesGroup {
            set_title: &t!("gpu_group_title"),
            set_description: Some(&t!("gpu_group_desc")),

            add = &gtk::Label {
                #[watch]
                set_visible: !model.supergfxctl_available,
                set_label: &t!("supergfxctl_missing_warning"),
                add_css_class: "error",
                set_wrap: true,
                set_xalign: 0.0,
                set_margin_top: 8,
                set_margin_start: 12,
                set_margin_end: 12,
                set_margin_bottom: 4,
            },

            add = &model.combo_row.clone() -> adw::ComboRow {
                set_title: &t!("gpu_mode_title"),
                set_subtitle: &t!("gpu_mode_subtitle"),

                #[watch]
                set_sensitive: model.supergfxctl_available,
            },

            add = &gtk::Label {
                set_label: &t!("gpu_reboot_warning"),
                add_css_class: "dim-label",
                set_wrap: true,
                set_xalign: 0.0,
                set_margin_top: 8,
                set_margin_start: 12,
                set_margin_end: 12,
                set_margin_bottom: 4,
            },
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let combo_row = adw::ComboRow::new();

        combo_row.connect_selected_notify({
            let sender = sender.clone();
            move |row| {
                sender.input(GpuMsg::ChangeMode(row.selected()));
            }
        });

        let saved_mode = GfxMode::from(AppConfig::load().gpu_mode);

        // Set a placeholder model so the ComboRow always renders,
        // even before the daemon responds or when it is unavailable.
        combo_row.set_model(Some(&gtk::StringList::new(&[&t!(saved_mode.i18n_key())])));

        let model = GpuModel {
            supergfxctl_available: false,
            current_mode: saved_mode,
            display_modes: Vec::new(),
            combo_row,
        };

        let widgets = view_output!();

        // Single command: check availability first, then read mode only if reachable.
        // This prevents spurious error toasts when supergfxctl is not installed.
        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    let available = dbus::check_supergfxctl_available().await;
                    out.emit(GpuCommandOutput::SupergfxctlChecked(available));

                    if !available {
                        return;
                    }

                    let current = match dbus::get_gpu_mode().await {
                        Ok(m) => m,
                        Err(e) => {
                            out.emit(GpuCommandOutput::Fehler(e));
                            return;
                        }
                    };
                    let supported = match dbus::get_supported_gpu_modes().await {
                        Ok(v) => v,
                        Err(e) => {
                            out.emit(GpuCommandOutput::Fehler(e));
                            return;
                        }
                    };
                    out.emit(GpuCommandOutput::InitModeAndSupported(current, supported));
                })
                .drop_on_shutdown()
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: GpuMsg, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            GpuMsg::ChangeMode(idx) => {
                let Some(&mode) = self.display_modes.get(idx as usize) else {
                    return;
                };
                if mode == self.current_mode {
                    return;
                }
                self.current_mode = mode;
                AppConfig::update(|c| c.gpu_mode = mode as u32);

                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            match dbus::set_gpu_mode(mode).await {
                                Ok(m) => out.emit(GpuCommandOutput::ModeSet(m)),
                                Err(e) => out.emit(GpuCommandOutput::Fehler(e)),
                            }
                        })
                        .drop_on_shutdown()
                });
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: GpuCommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            GpuCommandOutput::SupergfxctlChecked(available) => {
                self.supergfxctl_available = available;
            }
            GpuCommandOutput::InitModeAndSupported(current, supported) => {
                // Build display list: active mode first, then any additional supported modes.
                let mut modes = vec![current];
                for m in supported {
                    if !modes.contains(&m) {
                        modes.push(m);
                    }
                }
                self.display_modes = modes;
                self.current_mode = current;

                let labels: Vec<&str> = self.display_modes.iter().map(|m| m.i18n_key()).collect();
                let translated: Vec<String> = labels.iter().map(|k| t!(*k).to_string()).collect();
                let str_refs: Vec<&str> = translated.iter().map(|s| s.as_str()).collect();
                self.combo_row.set_model(Some(&gtk::StringList::new(&str_refs)));

                let selected_idx = self
                    .display_modes
                    .iter()
                    .position(|m| *m == current)
                    .unwrap_or(0) as u32;
                self.combo_row.set_selected(selected_idx);
            }
            GpuCommandOutput::ModeSet(mode) => {
                tracing::info!(
                    "{}",
                    t!("gpu_mode_set", mode = t!(mode.i18n_key()).to_string())
                );
            }
            GpuCommandOutput::Fehler(e) => {
                let _ = sender.output(e);
            }
        }
    }
}
