use relm4::adw;
use relm4::adw::prelude::*;
use relm4::prelude::*;

use super::helpers::qdbus_ausfuehren;

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
    type Output = ();
    type CommandOutput = OledCareCommandOutput;

    view! {
        adw::PreferencesGroup {
            set_title: "ASUS OLED Care",

            add = &adw::SwitchRow {
                set_title: "Pixelaktualisierung",
                set_subtitle: "Starten eines speziellen Bildschirmschoners nach Inaktivität, um OLED-Pixel gleichmäßig zu belasten.",

                #[watch]
                set_active: model.pixel_refresh_aktiv,

                connect_active_notify[sender] => move |switch| {
                    sender.input(OledCareMsg::PixelRefreshUmschalten(switch.is_active()));
                },
            },

            add = &adw::SwitchRow {
                set_title: "KDE-Panel automatisch ausblenden",
                set_subtitle: "Blendet das KDE-Panel automatisch aus, um statische Elemente auf dem OLED-Display zu reduzieren.",

                #[watch]
                set_active: model.panel_ausblenden_aktiv,

                connect_active_notify[sender] => move |switch| {
                    sender.input(OledCareMsg::PanelAusblendenUmschalten(switch.is_active()));
                },
            },

            add = &adw::SwitchRow {
                set_title: "Transparenzeffekt des Panels",
                set_subtitle: "Aktiviert die Transparenz des KDE-Panels, um OLED-Einbrennen zu reduzieren.",

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
        let model = OledCareModel {
            pixel_refresh_aktiv: false,
            panel_ausblenden_aktiv: false,
            transparenz_aktiv: false,
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

                let idle_time = if aktiv { "300" } else { "600" };
                sender.command(move |out, shutdown| {
                    shutdown
                        .register(async move {
                            let args = [
                                "--file",
                                "powermanagementprofilesrc",
                                "--group",
                                "AC",
                                "--group",
                                "DPMSControl",
                                "--key",
                                "idleTime",
                                idle_time,
                            ];
                            kwriteconfig(
                                &args,
                                &out,
                                OledCareCommandOutput::PixelRefreshGesetzt(aktiv),
                            )
                            .await;
                        })
                        .drop_on_shutdown()
                });
            }
            OledCareMsg::PanelAusblendenUmschalten(aktiv) => {
                if aktiv == self.panel_ausblenden_aktiv {
                    return;
                }
                self.panel_ausblenden_aktiv = aktiv;

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
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            OledCareCommandOutput::PanelGesetzt(aktiv) => {
                eprintln!(
                    "KDE-Panel Auto-Hide auf {} gesetzt",
                    if aktiv { "autohide" } else { "none" }
                );
            }
            OledCareCommandOutput::TransparenzGesetzt(aktiv) => {
                eprintln!(
                    "Panel-Transparenz auf {} gesetzt",
                    if aktiv { "transparent" } else { "opaque" }
                );
            }
            OledCareCommandOutput::PixelRefreshGesetzt(aktiv) => {
                eprintln!(
                    "DPMS idleTime auf {} gesetzt",
                    if aktiv { "300s" } else { "600s" }
                );
            }
            OledCareCommandOutput::Fehler(e) => {
                eprintln!("Fehler: {e}");
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

/// Führt kwriteconfig6 mit den gegebenen Argumenten aus.
async fn kwriteconfig(
    args: &[&str],
    out: &relm4::Sender<OledCareCommandOutput>,
    erfolg: OledCareCommandOutput,
) {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let result = tokio::task::spawn_blocking(move || {
        std::process::Command::new("kwriteconfig6")
            .args(&args)
            .status()
    })
    .await;

    match result {
        Ok(Ok(status)) if status.success() => {
            out.emit(erfolg);
        }
        Ok(Ok(status)) => {
            out.emit(OledCareCommandOutput::Fehler(format!(
                "kwriteconfig6 fehlgeschlagen mit Exit-Code: {}",
                status.code().unwrap_or(-1)
            )));
        }
        Ok(Err(e)) => {
            out.emit(OledCareCommandOutput::Fehler(format!(
                "kwriteconfig6 starten fehlgeschlagen: {e}"
            )));
        }
        Err(e) => {
            out.emit(OledCareCommandOutput::Fehler(format!(
                "spawn_blocking fehlgeschlagen: {e}"
            )));
        }
    }
}
