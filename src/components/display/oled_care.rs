use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;
use rust_i18n::t;

use super::helpers::qdbus_ausfuehren;
use crate::services::commands::run_command_blocking;
use crate::services::config::AppConfig;

pub struct OledCareModel {
    pixel_refresh_aktiv: bool,
    panel_ausblenden_aktiv: bool,
    transparenz_aktiv: bool,
}

#[derive(Debug)]
pub enum OledCareMsg {
    PixelRefreshUmschalten(bool),
    PanelAusblendenUmschalten(bool),
    TransparenzUmschalten(bool),
}

#[derive(Debug)]
pub enum OledCareCommandOutput {
    PanelGesetzt(bool),
    TransparenzGesetzt(bool),
    PixelRefreshGesetzt(bool),
    Fehler(String),
}

#[relm4::component(pub)]
impl Component for OledCareModel {
    type Init = ();
    type Input = OledCareMsg;
    type Output = String;
    type CommandOutput = OledCareCommandOutput;

    view! {
        adw::PreferencesGroup {
            set_title: &t!("oled_care_group_title"),

            add = &adw::SwitchRow {
                set_title: &t!("oled_care_pixel_refresh_title"),
                set_subtitle: &t!("oled_care_pixel_refresh_subtitle"),

                #[watch]
                set_active: model.pixel_refresh_aktiv,

                connect_active_notify[sender] => move |switch| {
                    sender.input(OledCareMsg::PixelRefreshUmschalten(switch.is_active()));
                },
            },

            add = &adw::SwitchRow {
                set_title: &t!("oled_care_panel_autohide_title"),
                set_subtitle: &t!("oled_care_panel_autohide_subtitle"),

                #[watch]
                set_active: model.panel_ausblenden_aktiv,

                connect_active_notify[sender] => move |switch| {
                    sender.input(OledCareMsg::PanelAusblendenUmschalten(switch.is_active()));
                },
            },

            add = &adw::SwitchRow {
                set_title: &t!("oled_care_transparency_title"),
                set_subtitle: &t!("oled_care_transparency_subtitle"),

                #[watch]
                set_active: model.transparenz_aktiv,

                connect_active_notify[sender] => move |switch| {
                    sender.input(OledCareMsg::TransparenzUmschalten(switch.is_active()));
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let config = AppConfig::load();
        let model = OledCareModel {
            pixel_refresh_aktiv: config.oled_care_pixel_refresh,
            panel_ausblenden_aktiv: config.oled_care_panel_autohide,
            transparenz_aktiv: config.oled_care_transparenz,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: OledCareMsg, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            OledCareMsg::PixelRefreshUmschalten(aktiv) => {
                if aktiv == self.pixel_refresh_aktiv {
                    return;
                }
                self.pixel_refresh_aktiv = aktiv;

                AppConfig::update(|c| c.oled_care_pixel_refresh = aktiv);

                let idle_time = if aktiv { "300" } else { "600" };
                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            match run_command_blocking(
                                "kwriteconfig6",
                                &[
                                    "--file",
                                    "powermanagementprofilesrc",
                                    "--group",
                                    "AC",
                                    "--group",
                                    "DPMSControl",
                                    "--key",
                                    "idleTime",
                                    idle_time,
                                ],
                            )
                            .await
                            {
                                Ok(()) => {
                                    out.emit(OledCareCommandOutput::PixelRefreshGesetzt(aktiv))
                                }
                                Err(e) => out.emit(OledCareCommandOutput::Fehler(e)),
                            }
                        })
                        .drop_on_shutdown()
                });
            }
            OledCareMsg::PanelAusblendenUmschalten(aktiv) => {
                if aktiv == self.panel_ausblenden_aktiv {
                    return;
                }
                self.panel_ausblenden_aktiv = aktiv;

                AppConfig::update(|c| c.oled_care_panel_autohide = aktiv);

                let hiding = if aktiv { "autohide" } else { "none" };
                let script = format!("panels().forEach(function(p){{p.hiding='{}';}})", hiding);
                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            plasmashell_evaluate(
                                &script,
                                &out,
                                OledCareCommandOutput::PanelGesetzt(aktiv),
                            )
                            .await;
                        })
                        .drop_on_shutdown()
                });
            }
            OledCareMsg::TransparenzUmschalten(aktiv) => {
                if aktiv == self.transparenz_aktiv {
                    return;
                }
                self.transparenz_aktiv = aktiv;

                AppConfig::update(|c| c.oled_care_transparenz = aktiv);

                let opacity = if aktiv { "transparent" } else { "opaque" };
                let script = format!("panels().forEach(function(p){{p.opacity='{}';}})", opacity);
                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            plasmashell_evaluate(
                                &script,
                                &out,
                                OledCareCommandOutput::TransparenzGesetzt(aktiv),
                            )
                            .await;
                        })
                        .drop_on_shutdown()
                });
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: OledCareCommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            OledCareCommandOutput::PanelGesetzt(aktiv) => {
                let value = if aktiv { "autohide" } else { "none" };
                eprintln!("{}", t!("oled_care_panel_set", value = value));
            }
            OledCareCommandOutput::TransparenzGesetzt(aktiv) => {
                let value = if aktiv { "transparent" } else { "opaque" };
                eprintln!("{}", t!("oled_care_transparency_set", value = value));
            }
            OledCareCommandOutput::PixelRefreshGesetzt(aktiv) => {
                let value = if aktiv { "300s" } else { "600s" };
                eprintln!("{}", t!("oled_care_dpms_set", value = value));
            }
            OledCareCommandOutput::Fehler(e) => {
                let _ = sender.output(e);
            }
        }
    }
}

/// Führt ein PlasmaShell evaluateScript via qdbus aus.
async fn plasmashell_evaluate(
    script: &str,
    out: &relm4::Sender<OledCareCommandOutput>,
    erfolg: OledCareCommandOutput,
) {
    let args = vec![
        "org.kde.plasmashell".to_string(),
        "/PlasmaShell".to_string(),
        "org.kde.PlasmaShell.evaluateScript".to_string(),
        script.to_string(),
    ];
    match qdbus_ausfuehren(args).await {
        Ok(()) => out.emit(erfolg),
        Err(e) => out.emit(OledCareCommandOutput::Fehler(e)),
    }
}
